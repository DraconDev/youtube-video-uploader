mod output;
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
#[command(about = "Upload videos to YouTube via the Data API v3", version)]
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

    #[arg(
        short = 'P',
        long,
        help = "Upload profile name (from ~/.config/video-uploader/profiles/<name>.toml)"
    )]
    profile: Option<String>,

    /// Output format: human (pretty-printed) or json (machine-readable)
    #[arg(long, global = true, default_value = "human")]
    output: OutputFormat,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
enum Commands {
    /// Authenticate with YouTube (one-time per workspace)
    Auth {
        #[arg(long)]
        client_id: Option<String>,

        #[arg(long)]
        client_secret: Option<String>,
    },
    /// Upload a single video
    Upload {
        #[arg(long, help = "Path to video file")]
        file: String,

        #[arg(long)]
        title: String,

        #[arg(long, short)]
        description: Option<String>,

        #[arg(long, help = "Comma-separated tags")]
        tags: Option<String>,

        #[arg(long, short, help = "Visibility: public, unlisted, private (default: private)")]
        visibility: Option<VisibilityArg>,

        #[arg(long, help = "YouTube category ID (default: 22 People & Blogs)")]
        category: Option<String>,

        #[arg(long, help = "Mark as made for kids (required by YouTube)")]
        made_for_kids: Option<bool>,

        #[arg(long, help = "License: youtube or creative-common")]
        license: Option<String>,

        #[arg(long, help = "BCP-47 language code (e.g. en, es, fr)")]
        language: Option<String>,

        #[arg(long, help = "Video contains AI/synthetic media")]
        contains_synthetic_media: Option<bool>,

        #[arg(long, help = "Allow embedding on other sites")]
        embeddable: Option<bool>,

        #[arg(long, help = "Show public view counts")]
        public_stats_viewable: Option<bool>,

        #[arg(long, help = "Scheduled publish time (ISO 8601, e.g. 2026-05-20T09:00:00Z)")]
        publish_at: Option<String>,

        #[arg(long, help = "Text to append to the description")]
        description_suffix: Option<String>,

        #[arg(long, help = "Recording date (ISO 8601, e.g. 2026-05-18)")]
        recording_date: Option<String>,

        #[arg(long, help = "Path to per-video metadata TOML (auto-discovered: <video>.meta.toml)")]
        meta: Option<String>,
    },
    /// List configured workspaces
    List,
    /// Batch upload from a CSV manifest
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
    /// Manage workspaces (set default, rename, remove)
    Workspace {
        #[command(subcommand)]
        action: WorkspaceAction,
    },

    /// Manage upload profiles
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
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

/// Subcommands for managing upload profiles.
#[derive(clap::Subcommand, Debug)]
enum ProfileAction {
    /// List available profiles
    List,
    /// Show the contents of a profile
    Show {
        name: String,
    },
    /// Delete a profile
    Remove {
        name: String,
    },
}

#[derive(clap::ValueEnum, Clone, Default, Debug)]
enum OutputFormat {
    #[default]
    Human,
    Json,
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
    profile: Option<String>,
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
            Some("public") => VisibilityArg::Public,
            Some("unlisted") => VisibilityArg::Unlisted,
            _ => VisibilityArg::Private,
        };
        let workspace = get("workspace");
        let profile = get("profile");

        entries.push(BatchEntry {
            file: canonical_file.to_string_lossy().to_string(),
            title,
            description,
            tags,
            visibility,
            workspace,
            profile,
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
async fn main() -> anyhow::Result<()> {
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
                output::auth_banner(&resp.user_code, &resp.verification_url);
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
            output::auth_success(&workspace);
        }

        Commands::Upload {
            file,
            title,
            description,
            tags,
            visibility,
            category,
            made_for_kids,
            license,
            language,
            contains_synthetic_media,
            embeddable,
            public_stats_viewable,
            publish_at,
            description_suffix,
            recording_date,
            meta,
        } => {
            let store = Arc::new(Mutex::new(CredentialStore::load(&passphrase)?));
            let workspace = resolve_workspace(&*store.lock().await, cli.workspace.as_deref())?;
            let youtube = YouTubeUploader::new(store, &passphrase, &workspace);
            let file = expand_tilde(&file);

            // 1. Load meta TOML: explicit --meta flag > auto-discover <video>.meta.toml
            let meta_path = meta.as_deref().map(std::path::PathBuf::from)
                .or_else(|| video_uploader::VideoMeta::discover(std::path::Path::new(&file)));
            let video_meta = if let Some(ref path) = meta_path {
                output::info(&format!("Loading metadata from {}", path.display()));
                video_uploader::VideoMeta::load_from(path)?
            } else {
                video_uploader::VideoMeta::default()
            };

            // 2. Determine profile: meta can specify a profile, CLI --profile overrides
            let profile_name = cli.profile.as_deref().or(video_meta.profile.as_deref());
            let profile = video_uploader::UploadProfile::resolve(profile_name)?;

            // 3. Build video: built-in defaults → profile → meta TOML → CLI flags
            //    (each layer overrides the previous)
            //    Use empty title initially — all fields come from the resolution layers.
            let mut video = VideoUpload::new(&file, "");

            // Apply profile defaults (lowest priority)
            video = video.apply_profile(&profile);

            // Apply meta TOML (overrides profile)
            video = video_meta.apply_to(video);

            // Apply explicit CLI flags (highest priority — always wins)
            // Title is a required CLI arg, so it always overrides meta.
            video = video.with_title(&title);
            if let Some(ref desc) = description {
                video = video.with_description(desc);
            }
            if let Some(ref t) = tags {
                video = video.with_tags(t.split(',').map(|s| s.trim().to_string()).collect());
            }
            if let Some(vis) = visibility {
                video = video.with_visibility(vis.into());
            }

            if let Some(cat) = category {
                video = video.with_category(&cat);
            }
            if let Some(kids) = made_for_kids {
                video = video.with_made_for_kids(kids);
            }
            if let Some(ref lic) = license
                && let Ok(l) = lic.parse()
            {
                video = video.with_license(l);
            }
            if let Some(ref lang) = language {
                video = video.with_language(lang);
            }
            if let Some(flag) = contains_synthetic_media {
                video = video.with_contains_synthetic_media(flag);
            }
            if let Some(flag) = embeddable {
                video = video.with_embeddable(flag);
            }
            if let Some(flag) = public_stats_viewable {
                video = video.with_public_stats_viewable(flag);
            }
            if let Some(ref dt) = publish_at {
                video = video.with_publish_at(dt);
            }
            if let Some(ref suffix) = description_suffix {
                video = video.with_description_suffix(suffix);
            }
            if let Some(ref date) = recording_date {
                video = video.with_recording_date(date);
            }

            let progress = Arc::new(StderrProgressListener::new());
            match youtube.upload(&video, Some(progress.clone())).await {
                Ok(r) => {
                    match cli.output {
                        OutputFormat::Human => {
                            output::upload_result(
                                &r.workspace,
                                &r.video_id,
                                &r.url,
                                &r.title,
                                &video.visibility().to_string(),
                            );
                        }
                        OutputFormat::Json => {
                            output::upload_result_json(&r);
                        }
                    }
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
                let preview: Vec<(String, String, Option<String>)> = entries
                    .iter()
                    .map(|e| (e.file.clone(), e.title.clone(), e.workspace.clone()))
                    .collect();
                output::dry_run(&preview);
                return Ok(());
            }

            // Pre-validate all entries before uploading
            let mut validation_errors = Vec::new();
            for (i, entry) in entries.iter().enumerate() {
                let video = VideoUpload::new(expand_tilde(&entry.file), &entry.title)
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
            let progress = Arc::new(StderrProgressListener::new());
            let semaphore = Arc::new(Semaphore::new(concurrency));
            let global_profile = cli.profile.clone();
            let total = entries.len();
            let mut handles = Vec::with_capacity(total);

            for (i, entry) in entries.iter().enumerate() {
                let entry = entry.clone();
                let store = Arc::clone(&store);
                let passphrase = passphrase.clone();
                let progress = Arc::clone(&progress);
                let row_global_profile = global_profile.clone();
                let semaphore = Arc::clone(&semaphore);

                handles.push(tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    let ws_display = entry.workspace.as_deref().unwrap_or("default");
                    output::batch_progress(i + 1, total, ws_display, &entry.title);

                    let store_guard = store.lock().await;
                    let ws = match resolve_workspace(&store_guard, entry.workspace.as_deref()) {
                        Ok(w) => w,
                        Err(e) => {
                            output::print_error(&format!("Workspace error: {e}"));
                            return 1u32;
                        }
                    };
                    drop(store_guard);

                    let video = {
                        // Per-row profile: CSV profile column > --profile flag
                        let row_profile = entry.profile.as_deref().or(row_global_profile.as_deref());
                        let profile = match video_uploader::UploadProfile::resolve(row_profile) {
                            Ok(p) => p,
                            Err(e) => {
                                output::print_error(&format!("Profile error: {e}"));
                                return 1u32;
                            }
                        };

                        // Auto-discover meta TOML for this video
                        let meta_path = video_uploader::VideoMeta::discover(
                            std::path::Path::new(&expand_tilde(&entry.file))
                        );
                        let video_meta = match meta_path {
                            Some(ref path) => match video_uploader::VideoMeta::load_from(path) {
                                Ok(m) => m,
                                Err(e) => {
                                    output::print_error(&format!("Meta error: {e}"));
                                    return 1u32;
                                }
                            },
                            None => video_uploader::VideoMeta::default(),
                        };

                        // Resolution: profile → meta → CSV fields
                        let mut v = VideoUpload::new(expand_tilde(&entry.file), &entry.title);
                        v = v.apply_profile(&profile);
                        v = video_meta.apply_to(v);

                        // CSV fields (CLI-equivalent, highest priority)
                        if let Some(ref desc) = entry.description {
                            v = v.with_description(desc);
                        }
                        if !entry.tags.is_empty() {
                            v = v.with_tags(entry.tags.clone());
                        }
                        v = v.with_visibility(entry.visibility.clone().into());
                        v
                    };

                    let youtube = YouTubeUploader::new(store, &passphrase, &ws);
                    match youtube.upload(&video, Some(progress.clone())).await {
                        Ok(r) => {
                            output::batch_item_result(&r.url, &r.video_id);
                            0u32
                        }
                        Err(e) => {
                            output::batch_item_error(&e.to_string());
                            1u32
                        }
                    }
                }));
            }

            let mut failures = 0u32;
            for handle in handles {
                let result = handle
                    .await
                    .map_err(|e| anyhow::anyhow!("Task join: {e}"))?;
                if result > 0 {
                    failures += result;
                }
                // Note: we don't collect individual video results from spawned tasks yet.
                // For full JSON batch output, we'd need to return Result<UploadResult, String> instead of u32.
            }

            if failures > 0 {
                return Err(anyhow::anyhow!(
                    "Batch completed with {} failure(s) out of {} video(s)",
                    failures,
                    entries.len()
                ));
            }
            let succeeded = total - validation_errors.len();
            match cli.output {
                OutputFormat::Human => output::batch_summary(total, succeeded, validation_errors.len()),
                OutputFormat::Json => {
                    let summary = serde_json::json!({
                        "total": total,
                        "succeeded": succeeded,
                        "failed": validation_errors.len(),
                    });
                    println!("{summary}");
                }
            }
        }

        Commands::List => {
            let store = CredentialStore::load(&passphrase)?;
            let default = store.default_workspace();
            let workspaces: Vec<_> = store
                .workspaces()
                .map(|w| (w.as_str(), default == Some(w)))
                .collect();
            output::workspace_list(&workspaces);
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
                    output::workspace_default_set(&name);
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
                    output::workspace_renamed(&old, &new);
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
                    output::workspace_removed(&name);
                }
            }
        }
        Commands::Profile { action } => {
            match action {
                ProfileAction::List => {
                    let profiles_map = video_uploader::UploadProfile::list()?;
                    let profiles: Vec<_> = profiles_map.into_iter().collect();
                    output::profile_list(&profiles);
                }
                ProfileAction::Show { name } => {
                    let profile = video_uploader::UploadProfile::load(&name)?;
                    output::profile_show(&name, &profile);
                }
                ProfileAction::Remove { name } => {
                    video_uploader::UploadProfile::remove(&name)?;
                    output::profile_removed(&name);
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
