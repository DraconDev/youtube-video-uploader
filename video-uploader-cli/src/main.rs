use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::Arc;
use video_uploader::{
    CredentialStore, StderrProgressListener, UploaderRegistry, VideoUpload,
    auth::{device_code, now_secs},
    config::PlatformCredentials,
    validate_daemon_url,
};

#[derive(Parser)]
#[command(name = "video-uploader")]
#[command(about = "Upload videos to YouTube and Odysee")]
struct Cli {
    #[arg(short, long)]
    passphrase: Option<String>,

    #[arg(
        long,
        help = "Read passphrase from a file (avoids command-line exposure)"
    )]
    passphrase_file: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Auth {
        #[arg(value_enum, default_value = "youtube")]
        platform: PlatformArg,

        #[arg(long)]
        client_id: Option<String>,

        #[arg(long)]
        client_secret: Option<String>,
    },
    Upload {
        #[arg(long, help = "Path to video file")]
        file: String,

        #[arg(long)]
        title: String,

        #[arg(long)]
        description: Option<String>,

        #[arg(long)]
        tags: Option<String>,

        #[arg(long, default_value = "public")]
        visibility: VisibilityArg,

        #[arg(long)]
        platforms: Option<String>,
    },
    List,
    Batch {
        #[arg(
            long,
            help = "Path to CSV manifest (columns: file,title,description,tags,visibility,platforms)"
        )]
        manifest: String,

        #[arg(long, help = "Validate without uploading")]
        dry_run: bool,
    },
}

#[derive(clap::ValueEnum, Clone, Default)]
enum PlatformArg {
    #[default]
    Youtube,
    Odysee,
}

#[derive(clap::ValueEnum, Clone, Default, Debug)]
enum VisibilityArg {
    #[default]
    Public,
    Unlisted,
    Private,
}

impl From<VisibilityArg> for video_uploader::Visibility {
    fn from(v: VisibilityArg) -> Self {
        match v {
            VisibilityArg::Public => video_uploader::Visibility::Public,
            VisibilityArg::Unlisted => video_uploader::Visibility::Unlisted,
            VisibilityArg::Private => video_uploader::Visibility::Private,
        }
    }
}

impl std::fmt::Display for PlatformArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlatformArg::Youtube => write!(f, "youtube"),
            PlatformArg::Odysee => write!(f, "odysee"),
        }
    }
}

#[derive(Debug)]
struct BatchEntry {
    file: String,
    title: String,
    description: Option<String>,
    tags: Vec<String>,
    visibility: VisibilityArg,
    platforms: Vec<String>,
}

fn parse_csv_manifest(path: &str) -> anyhow::Result<Vec<BatchEntry>> {
    let path = expand_tilde(path);
    let canonical = std::fs::canonicalize(&path)
        .map_err(|e| anyhow::anyhow!("CSV manifest path not accessible: {} ({})", path, e))?;
    let base_dir = canonical
        .parent()
        .ok_or_else(|| anyhow::anyhow!("CSV manifest has no parent directory"))?;

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_path(&path)?;

    let headers = rdr.headers()?.clone();
    let mut entries = Vec::new();

    for (i, result) in rdr.records().enumerate() {
        let record = result?;
        let get = |name: &str| -> Option<String> {
            headers.iter().position(|h| h == name).and_then(|i| {
                let v = record.get(i)?.trim();
                if v.is_empty() {
                    None
                } else {
                    Some(v.to_string())
                }
            })
        };

        let file =
            get("file").ok_or_else(|| anyhow::anyhow!("Missing 'file' column in manifest"))?;
        let abs_file = if file.starts_with('/') {
            file.clone()
        } else if file.starts_with("~/") {
            expand_tilde(&file)
        } else {
            base_dir.join(&file).to_string_lossy().to_string()
        };
        if !abs_file.starts_with('/') {
            return Err(anyhow::anyhow!(
                "CSV manifest entry {} file path is not absolute: {} (resolved to {})",
                i + 1,
                file,
                abs_file
            ));
        }
        let canonical_file = std::fs::canonicalize(&abs_file).map_err(|e| {
            anyhow::anyhow!(
                "CSV manifest entry {} file path cannot be resolved: {} ({})",
                i + 1,
                abs_file,
                e
            )
        })?;
        let title =
            get("title").ok_or_else(|| anyhow::anyhow!("Missing 'title' column in manifest"))?;
        let description = get("description");
        let tags: Vec<String> = get("tags")
            .map(|t| {
                t.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        let visibility = match get("visibility").as_deref() {
            Some("private") => VisibilityArg::Private,
            Some("unlisted") => VisibilityArg::Unlisted,
            _ => VisibilityArg::Public,
        };
        let platforms = get("platforms")
            .map(|p| {
                p.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_else(|| vec!["youtube".to_string()]);

        entries.push(BatchEntry {
            file: canonical_file.to_string_lossy().to_string(),
            title,
            description,
            tags,
            visibility,
            platforms,
        });
    }

    Ok(entries)
}

fn get_env_passphrase() -> Result<String, anyhow::Error> {
    if let Ok(env_pass) = std::env::var("VIDEO_UPLOADER_PASSPHRASE") {
        if !env_pass.is_empty() {
            Ok(env_pass)
        } else {
            Err(anyhow::anyhow!("VIDEO_UPLOADER_PASSPHRASE is empty"))
        }
    } else {
        Err(anyhow::anyhow!(
            "--passphrase is required (or set VIDEO_UPLOADER_PASSPHRASE, or use --passphrase-file)"
        ))
    }
}

fn get_passphrase(
    cli_passphrase: Option<&str>,
    passphrase_file: Option<&str>,
) -> Result<String, anyhow::Error> {
    if let Some(file) = passphrase_file {
        let content =
            std::fs::read_to_string(file).map_err(|e| anyhow::anyhow!("passphrase file: {}", e))?;
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Err(anyhow::anyhow!("passphrase file '{}' is empty", file));
        }
        if trimmed.len() < 8 {
            return Err(anyhow::anyhow!(
                "Passphrase must be at least 8 characters (got {} from {})",
                trimmed.len(),
                file
            ));
        }
        return Ok(trimmed.to_string());
    }

    let pass = if let Some(p) = cli_passphrase {
        if !p.is_empty() {
            p.to_string()
        } else {
            get_env_passphrase()?
        }
    } else {
        get_env_passphrase()?
    };

    if pass.len() < 8 {
        return Err(anyhow::anyhow!(
            "Passphrase must be at least 8 characters (got {})",
            pass.len()
        ));
    }

    Ok(pass)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let cli = Cli::parse();
    let passphrase = get_passphrase(cli.passphrase.as_deref(), cli.passphrase_file.as_deref())
        .map_err(|e| {
            eprintln!("error: {}", e);
            anyhow::anyhow!("{}", e)
        })?;

    match cli.command {
        Commands::Auth {
            platform,
            client_id,
            client_secret,
        } => {
            let mut store = CredentialStore::load(&passphrase)?;

            match platform {
                PlatformArg::Youtube => {
                    let client_id = client_id
                        .or_else(|| std::env::var("YOUTUBE_CLIENT_ID").ok())
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "YOUTUBE_CLIENT_ID not set. Pass --client-id or set the env var."
                            )
                        })?;
                    let client_secret = client_secret.or_else(|| std::env::var("YOUTUBE_CLIENT_SECRET").ok())
                        .ok_or_else(|| anyhow::anyhow!("YOUTUBE_CLIENT_SECRET not set. Pass --client-secret or set the env var."))?;

                    tracing::info!("Starting YouTube device code flow...");
                    tracing::info!("YouTube OAuth2 requires a one-time setup.");
                    tracing::info!(
                        "Get your credentials from: https://console.cloud.google.com/apis/credentials"
                    );

                    let token =
                        device_code::run_device_code_flow(&client_id, &client_secret, |resp| {
                            println!();
                            println!("===========================================");
                            println!("  IMPORTANT: One-time YouTube authorization");
                            println!("===========================================");
                            println!();
                            println!("  1. Open this URL on any device:");
                            println!();
                            println!("     {}", resp.verification_url);
                            println!();
                            println!("  2. Enter this code: {}", resp.user_code);
                            println!();
                            println!("  Waiting for authorization... (Ctrl+C to cancel)");
                            println!();
                        })
                        .await?;

                    let creds = PlatformCredentials {
                        refresh_token: token.refresh_token,
                        access_token: Some(token.access_token),
                        token_expires_at: Some(now_secs() + token.expires_in),
                        client_id: Some(client_id),
                        client_secret: Some(client_secret),
                        api_key: None,
                        daemon_url: None,
                    };

                    store.set("youtube", creds);
                    store.save(&passphrase)?;
                    println!("\nYouTube credentials saved successfully!");
                }
                PlatformArg::Odysee => {
                    let daemon_url = std::env::var("ODYSEE_DAEMON_URL")
                        .unwrap_or_else(|_| "http://localhost:5279".to_string());
                    validate_daemon_url(&daemon_url)
                        .map_err(|e| anyhow::anyhow!("Invalid Odysee daemon URL: {e}"))?;
                    let creds = PlatformCredentials {
                        daemon_url: Some(daemon_url.clone()),
                        api_key: None,
                        refresh_token: None,
                        access_token: None,
                        token_expires_at: None,
                        client_id: None,
                        client_secret: None,
                    };
                    store.set("odysee", creds);
                    store.save(&passphrase)?;
                    println!("Odysee credentials saved successfully!");
                    println!(
                        "Note: Odysee requires lbrynet daemon running at {}",
                        daemon_url
                    );
                }
            }
        }

        Commands::Upload {
            file,
            title,
            description,
            tags,
            visibility,
            platforms,
        } => {
            let registry = UploaderRegistry::load(&passphrase)?;
            let file = expand_tilde(&file);
            let video = VideoUpload::new(&file, &title)
                .description(description.unwrap_or_default())
                .tags(
                    tags.map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                        .unwrap_or_default(),
                )
                .visibility(visibility.into());

            let platform_list: Vec<String> = platforms
                .as_ref()
                .map(|p| p.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_else(|| vec!["youtube".to_string()]);

            const KNOWN_PLATFORMS: [&str; 2] = ["youtube", "odysee"];
            for platform in &platform_list {
                if !KNOWN_PLATFORMS.contains(&platform.as_str()) {
                    tracing::warn!(
                        "Unknown platform '{}'. Valid platforms: {}",
                        platform,
                        KNOWN_PLATFORMS.join(", ")
                    );
                }
            }

            let progress = Arc::new(StderrProgressListener);
            let mut results = Vec::new();

            for platform in platform_list {
                match registry
                    .upload_to(&platform, &video, Some(progress.clone()))
                    .await
                {
                    Ok(r) => results.push(format!("{}: {} ({})", r.platform, r.url, r.platform_id)),
                    Err(e) => eprintln!("{platform} failed: {e}"),
                }
            }

            println!("\n--- Results ---");
            for r in results {
                println!("  {r}");
            }
        }

        Commands::Batch { manifest, dry_run } => {
            let entries = parse_csv_manifest(&manifest)?;
            println!("Batch manifest loaded: {} video(s)", entries.len());

            if dry_run {
                println!("\n--- Dry run ---");
                for (i, entry) in entries.iter().enumerate() {
                    println!(
                        "  {}. {} → {} (platforms: {})",
                        i + 1,
                        entry.file,
                        entry.title,
                        entry.platforms.join(", ")
                    );
                }
                return Ok(());
            }

            let store = CredentialStore::load(&passphrase)?;
            let registry = UploaderRegistry::new(store, &passphrase);
            let progress = Arc::new(StderrProgressListener);

            for (i, entry) in entries.iter().enumerate() {
                println!(
                    "\n[{} / {}] Uploading: {}",
                    i + 1,
                    entries.len(),
                    entry.title
                );

                let video = VideoUpload::new(expand_tilde(&entry.file), &entry.title)
                    .description(entry.description.clone().unwrap_or_default())
                    .tags(entry.tags.clone())
                    .visibility(entry.visibility.clone().into());

                for platform in &entry.platforms {
                    let result = registry
                        .upload_to(platform, &video, Some(progress.clone()))
                        .await;
                    match result {
                        Ok(r) => println!("  {}: {}", platform, r.url),
                        Err(e) => eprintln!("  {} failed: {}", platform, e),
                    }
                }
            }

            println!("\nBatch complete. {} video(s) processed.", entries.len());
        }

        Commands::List => {
            let store = CredentialStore::load(&passphrase)?;
            let platforms: Vec<_> = store.platforms().collect();
            if platforms.is_empty() {
                println!("No platforms configured. Run: video-uploader auth --platform <name>");
            } else {
                println!("Configured platforms:");
                for p in platforms {
                    println!("  - {p}");
                }
            }
        }
    }

    Ok(())
}

fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        dirs::home_dir()
            .map(|home| home.join(rest))
            .and_then(|p| p.to_str().map(String::from))
            .unwrap_or_else(|| path.into())
    } else if path == "~" {
        dirs::home_dir()
            .and_then(|p| p.to_str().map(String::from))
            .unwrap_or_else(|| path.into())
    } else {
        path.into()
    }
}
