use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::Semaphore;
use video_uploader::{
    CredentialStore, StderrProgressListener, VideoUpload, YouTubeUploader, Zeroizing,
    auth::{device_code, now_secs},
    config::PlatformCredentials,
};

#[derive(Parser)]
#[command(name = "video-uploader")]
#[command(about = "Upload videos to YouTube")]
struct Cli {
    #[arg(short, long)]
    passphrase: Option<String>,

    #[arg(
        long,
        help = "Read passphrase from a file (avoids command-line exposure)"
    )]
    passphrase_file: Option<String>,

    #[arg(
        short,
        long,
        help = "Target workspace (defaults to the configured default workspace)"
    )]
    workspace: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Auth {
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

        #[arg(long, short)]
        description: Option<String>,

        #[arg(long, help = "Comma-separated tags")]
        tags: Option<String>,

        #[arg(long, short, default_value = "private", help = "Visibility: public, unlisted, private")]
        visibility: VisibilityArg,

        #[arg(long, help = "YouTube category ID (default: 22 People & Blogs)")]
        category: Option<String>,

        #[arg(long, help = "Mark as made for kids (required by YouTube)")]
        made_for_kids: Option<bool>,
    },
    List,
    Batch {
        #[arg(
            long,
            help = "Path to CSV manifest (columns: file,title,description,tags,visibility,workspace)"
        )]
        manifest: String,

        #[arg(long, help = "Validate without uploading")]
        dry_run: bool,

        #[arg(long, default_value = "4", help = "Number of concurrent uploads")]
        concurrency: usize,
    },
    Workspace {
        #[command(subcommand)]
        action: WorkspaceAction,
    },
}

#[derive(Subcommand)]
enum WorkspaceAction {
    Default {
        name: String,
    },
    Rename {
        old: String,
        new: String,
    },
    Remove {
        name: String,
    },
}

#[derive(clap::ValueEnum, Clone, Default, Debug)]
enum VisibilityArg {
    #[default]
    Private,
    Unlisted,
    Public,
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

#[derive(Debug, Clone)]
struct BatchEntry {
    file: String,
    title: String,
    description: Option<String>,
    tags: Vec<String>,
    visibility: VisibilityArg,
    workspace: Option<String>,
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
        let workspace = get("workspace");

        entries.push(BatchEntry {
            file: canonical_file.to_string_lossy().to_string(),
            title,
            description,
            tags,
            visibility,
            workspace,
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
) -> Result<Zeroizing<String>, anyhow::Error> {
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
        return Ok(Zeroizing::new(trimmed.to_string()));
    }

    let pass = if let Some(p) = cli_passphrase {
        if !p.is_empty() {
            Zeroizing::new(p.to_string())
        } else {
            Zeroizing::new(get_env_passphrase()?)
        }
    } else {
        Zeroizing::new(get_env_passphrase()?)
    };

    if pass.len() < 8 {
        return Err(anyhow::anyhow!(
            "Passphrase must be at least 8 characters (got {})",
            pass.len()
        ));
    }

    Ok(pass)
}

fn resolve_workspace(
    store: &CredentialStore,
    explicit: Option<&str>,
) -> Result<String, anyhow::Error> {
    if let Some(name) = explicit {
        if store.get(name).is_none() {
            return Err(anyhow::anyhow!(
                "Workspace '{}' not found. Run 'video-uploader workspace list' to see available workspaces.",
                name
            ));
        }
        return Ok(name.to_string());
    }

    if let Some(default) = store.default_workspace()
        && store.get(default).is_some()
    {
        return Ok(default.to_string());
    }

    let names: Vec<_> = store.workspaces().collect();
    if names.len() == 1 {
        return Ok(names[0].clone());
    }

    Err(anyhow::anyhow!(
        "No workspace specified and no default workspace configured. Use --workspace or set a default with 'video-uploader workspace default <name>'."
    ))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env from CWD or project root (silent no-op if missing)
    let _ = dotenvy::dotenv();

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
            client_id,
            client_secret,
        } => {
            let mut store = CredentialStore::load(&passphrase)?;
            let client_id = client_id
                .or_else(|| std::env::var("YOUTUBE_CLIENT_ID").ok())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "YOUTUBE_CLIENT_ID not set. Pass --client-id or set the env var."
                    )
                })?;
            let client_secret = client_secret
                .or_else(|| std::env::var("YOUTUBE_CLIENT_SECRET").ok())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "YOUTUBE_CLIENT_SECRET not set. Pass --client-secret or set the env var."
                    )
                })?;

            let workspace = cli
                .workspace
                .clone()
                .or_else(|| store.default_workspace().map(String::from))
                .unwrap_or_else(|| "youtube".to_string());

            tracing::info!("Starting YouTube authorization for workspace '{}'...", workspace);
            tracing::info!("YouTube OAuth2 requires a one-time setup.");
            tracing::info!(
                "Get your credentials from: https://console.cloud.google.com/apis/credentials"
            );

            // Try device code flow first (works with "TVs and Limited Input" client type),
            // fall back to authorization code flow (works with "Web" and "Installed" client types)
            let token = match device_code::run_device_code_flow(&client_id, &client_secret, |resp| {
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
            .await
            {
                Ok(token) => token,
                Err(e) => {
                    let err_msg = e.to_string();
                    if err_msg.contains("invalid_client") || err_msg.contains("TVs and Limited Input") {
                        tracing::info!("Device code flow not supported for this client type, switching to browser auth...");
                        video_uploader::auth::auth_code::auth_code_flow(&client_id, &client_secret).await?
                    } else {
                        return Err(anyhow::anyhow!("Auth failed: {e}"));
                    }
                }
            };

            let creds = PlatformCredentials {
                refresh_token: token.refresh_token.map(Zeroizing::new),
                access_token: Some(Zeroizing::new(token.access_token)),
                token_expires_at: Some(now_secs() + token.expires_in),
                client_id: Some(Zeroizing::new(client_id)),
                client_secret: Some(Zeroizing::new(client_secret)),
                api_key: None,
            };

            store.set(&workspace, creds);
            if store.default_workspace().is_none() {
                store.set_default(&workspace);
            }
            store.save(&passphrase)?;
            println!("\nCredentials saved successfully for workspace '{}'!", workspace);
        }

        Commands::Upload {
            file,
            title,
            description,
            tags,
            visibility,
            category,
            made_for_kids,
        } => {
            let store = Arc::new(Mutex::new(CredentialStore::load(&passphrase)?));
            let workspace = resolve_workspace(&*store.lock().await, cli.workspace.as_deref())?;
            let youtube = YouTubeUploader::new(store, &passphrase, &workspace);
            let file = expand_tilde(&file);
            let mut video = VideoUpload::new(&file, &title)
                .with_description(description.unwrap_or_default())
                .with_tags(
                    tags.map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                        .unwrap_or_default(),
                )
                .with_visibility(visibility.into());

            if let Some(cat) = category {
                video = video.with_category(&cat);
            }
            if let Some(kids) = made_for_kids {
                video = video.with_made_for_kids(kids);
            }

            let progress = Arc::new(StderrProgressListener);
            match youtube.upload(&video, Some(progress.clone())).await {
                Ok(r) => {
                    println!("\n--- Result ---");
                    println!("  YouTube: {} ({})", r.url, r.video_id);
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Upload failed: {e}"));
                }
            }
        }

        Commands::Batch {
            manifest,
            dry_run,
            concurrency,
        } => {
            let entries = parse_csv_manifest(&manifest)?;
            println!("Batch manifest loaded: {} video(s)", entries.len());

            if dry_run {
                println!("\n--- Dry run ---");
                for (i, entry) in entries.iter().enumerate() {
                    let ws = entry.workspace.as_deref().unwrap_or("(default)");
                    println!("  {}. [{}] {} → {}", i + 1, ws, entry.file, entry.title,);
                }
                return Ok(());
            }

            // Pre-validate all entries before uploading
            let mut validation_errors = Vec::new();
            for (i, entry) in entries.iter().enumerate() {
                let video = VideoUpload::new(expand_tilde(&entry.file), &entry.title)
                    .with_description(entry.description.clone().unwrap_or_default())
                    .with_tags(entry.tags.clone())
                    .with_visibility(entry.visibility.clone().into());
                if let Err(e) = video_uploader::validation::validate(&video).await {
                    validation_errors.push(format!("Row {}: {}", i + 1, e));
                }
            }
            if !validation_errors.is_empty() {
                return Err(anyhow::anyhow!(
                    "Validation failed:\n{}",
                    validation_errors.join("\n")
                ));
            }

            let store = Arc::new(Mutex::new(CredentialStore::load(&passphrase)?));
            let progress = Arc::new(StderrProgressListener);
            let semaphore = Arc::new(Semaphore::new(concurrency));
            let total = entries.len();
            let mut handles = Vec::with_capacity(total);

            for (i, entry) in entries.iter().enumerate() {
                let entry = entry.clone();
                let store = Arc::clone(&store);
                let passphrase = passphrase.clone();
                let progress = Arc::clone(&progress);
                let semaphore = Arc::clone(&semaphore);

                handles.push(tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    let ws_display = entry.workspace.as_deref().unwrap_or("default");
                    println!("\n[{} / {}] [{}] Uploading: {}", i + 1, total, ws_display, entry.title);

                    let store_guard = store.lock().await;
                    let ws = match resolve_workspace(&store_guard, entry.workspace.as_deref()) {
                        Ok(w) => w,
                        Err(e) => {
                            eprintln!("  Workspace error: {e}");
                            return 1u32;
                        }
                    };
                    drop(store_guard);

                    let video = VideoUpload::new(expand_tilde(&entry.file), &entry.title)
                        .with_description(entry.description.clone().unwrap_or_default())
                        .with_tags(entry.tags.clone())
                        .with_visibility(entry.visibility.clone().into());

                    let youtube = YouTubeUploader::new(store, &passphrase, &ws);
                    match youtube.upload(&video, Some(progress.clone())).await {
                        Ok(r) => {
                            println!("  YouTube: {} ({})", r.url, r.video_id);
                            0u32
                        }
                        Err(e) => {
                            eprintln!("  YouTube failed: {e}");
                            1u32
                        }
                    }
                }));
            }

            let mut failures = 0u32;
            for handle in handles {
                failures += handle
                    .await
                    .map_err(|e| anyhow::anyhow!("Task join: {e}"))?;
            }

            if failures > 0 {
                return Err(anyhow::anyhow!(
                    "Batch completed with {} failure(s) out of {} video(s)",
                    failures,
                    entries.len()
                ));
            }
            println!("\nBatch complete. {} video(s) processed.", entries.len());
        }

        Commands::List => {
            let store = CredentialStore::load(&passphrase)?;
            let workspaces: Vec<_> = store.workspaces().collect();
            if workspaces.is_empty() {
                println!("No workspaces configured. Run: video-uploader auth");
            } else {
                println!("Workspaces:");
                let default = store.default_workspace();
                for w in workspaces {
                    let marker = if default == Some(w) { " (default)" } else { "" };
                    println!("  - {}{}", w, marker);
                }
            }
        }

        Commands::Workspace { action } => {
            let mut store = CredentialStore::load(&passphrase)?;
            match action {
                WorkspaceAction::Default { name } => {
                    if store.get(&name).is_none() {
                        return Err(anyhow::anyhow!(
                            "Workspace '{}' does not exist. Run 'video-uploader list' to see available workspaces.",
                            name
                        ));
                    }
                    store.set_default(&name);
                    store.save(&passphrase)?;
                    println!("Default workspace set to '{}'", name);
                }
                WorkspaceAction::Rename { old, new } => {
                    if store.get(&old).is_none() {
                        return Err(anyhow::anyhow!(
                            "Workspace '{}' does not exist.",
                            old
                        ));
                    }
                    let creds = store.remove(&old).expect("checked above");
                    store.set(&new, creds);
                    if store.default_workspace() == Some(&old) {
                        store.set_default(&new);
                    }
                    store.save(&passphrase)?;
                    println!("Workspace '{}' renamed to '{}'", old, new);
                }
                WorkspaceAction::Remove { name } => {
                    if store.get(&name).is_none() {
                        return Err(anyhow::anyhow!(
                            "Workspace '{}' does not exist.",
                            name
                        ));
                    }
                    store.remove(&name);
                    if store.default_workspace() == Some(&name) {
                        store.clear_default();
                    }
                    store.save(&passphrase)?;
                    println!("Workspace '{}' removed", name);
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
