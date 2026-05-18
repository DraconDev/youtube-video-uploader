//! Pretty-print output for the video-uploader CLI.
//!
//! All user-facing output goes through here for consistent formatting.

/// Print a section header with a box.
pub fn header(title: &str) {
    let width = title.len().max(40);
    let bar = "-".repeat(width + 2);
    eprintln!();
    eprintln!("  +{bar}+");
    eprintln!("  | {} |", pad_center(title, width));
    eprintln!("  +{bar}+");
}

/// Pad a string to center it within a given width.
fn pad_center(s: &str, width: usize) -> String {
    let pad = width.saturating_sub(s.chars().count());
    let left = pad / 2;
    let right = pad - left;
    format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
}

/// Print a sub-header (thin line under text).
pub fn sub_header(title: &str) {
    eprintln!();
    eprintln!("  {title}");
    eprintln!("  {}", "-".repeat(title.len()));
}

/// Print a key-value pair aligned.
pub fn kv(key: &str, value: &str) {
    eprintln!("  {:>14}  {}", format!("{key}:"), value);
}

/// Print a key-value pair with a warning badge.
pub fn kv_badge(key: &str, value: &str, badge: &str) {
    eprintln!("  {:>14}  {} {}", format!("{key}:"), value, badge);
}

/// Print a success message with a checkmark.
pub fn success(msg: &str) {
    eprintln!("  \u{2714} {msg}");
}

/// Print an info message.
pub fn info(msg: &str) {
    eprintln!("  \u{2192} {msg}");
}

/// Print a warning message.
pub fn warn(msg: &str) {
    eprintln!("  \u{26A0} {msg}");
}

/// Print an error message.
pub fn print_error(msg: &str) {
    eprintln!("  \u{2718} {msg}");
}

/// Print a list item with a bullet.
#[allow(dead_code)]
pub fn bullet(text: &str) {
    eprintln!("  \u{2022} {text}");
}

/// Print a numbered item.
#[allow(dead_code)]
pub fn numbered(n: usize, text: &str) {
    eprintln!("  {:>2}. {text}", n);
}

/// Print a blank line (thin spacer).
#[allow(dead_code)]
pub fn spacer() {
    eprintln!();
}

/// Print the auth instructions banner.
pub fn auth_banner(code: &str, url: &str) {
    eprintln!();
    header("One-time YouTube Authorization");
    eprintln!("  1. Open this URL on any device:");
    eprintln!();
    eprintln!("     {url}");
    eprintln!();
    eprintln!("  2. Enter this code:");
    eprintln!();
    eprintln!("     +{:-<13}+", "");
    eprintln!("     | {code:^11} |");
    eprintln!("     +{:-<13}+", "");
    eprintln!();
    info("Waiting for authorization... (Ctrl+C to cancel)");
    eprintln!();
}

/// Print the upload result.
pub fn upload_result(
    workspace: &str,
    video_id: &str,
    url: &str,
    title: &str,
    visibility: &str,
) {
    eprintln!();
    header("Upload Complete");
    kv("Title", title);
    kv("Workspace", workspace);
    kv("Visibility", visibility);
    kv("Video ID", video_id);
    eprintln!();
    eprintln!("  \u{25B8} {url}");
    eprintln!();
}

/// Print the batch result summary.
pub fn batch_summary(total: usize, succeeded: usize, failed: usize) {
    eprintln!();
    if failed == 0 {
        header("Batch Complete");
    } else {
        header("Batch Complete (with errors)");
    }
    kv("Total", &total.to_string());
    kv("Succeeded", &succeeded.to_string());
    if failed > 0 {
        kv_badge("Failed", &failed.to_string(), "\u{26A0}");
    }
    eprintln!();
}

/// Print the batch progress line.
pub fn batch_progress(current: usize, total: usize, workspace: &str, title: &str) {
    eprintln!("\n  [{}/{}] [{}] Uploading: {}", current, total, workspace, title);
}

/// Print a batch item result.
pub fn batch_item_result(url: &str, video_id: &str) {
    success(&format!("{url} ({video_id})"));
}

/// Print a batch item error.
pub fn batch_item_error(err: &str) {
    print_error(err);
}

/// Print the workspace list with optional channel names.
pub fn workspace_list(workspaces: &[(&str, bool, Option<&str>)]) {
    if workspaces.is_empty() {
        info("No workspaces configured. Run: video-uploader auth");
    } else {
        sub_header("Workspaces");
        for (name, is_default, channel) in workspaces {
            let default_marker = if *is_default { " (default)" } else { "" };
            if let Some(ch) = channel {
                eprintln!("  \u{2022} {name}{default_marker}  \u{2192} {ch}");
            } else {
                eprintln!("  \u{2022} {name}{default_marker}");
            }
        }
    }
}

/// Print the profile list.
pub fn profile_list(profiles: &[(String, video_uploader::UploadProfile)]) {
    if profiles.is_empty() {
        info("No profiles found.");
        eprintln!("  Create one at: ~/.config/video-uploader/profiles/<name>.toml");
    } else {
        sub_header("Upload Profiles");
        for (name, _p) in profiles {
            eprintln!("  \u{2022} {name}");
        }
        eprintln!();
        info("Use 'profile show <name>' to see details");
        eprintln!("  Edit profiles at ~/.config/video-uploader/profiles/");
    }
}

/// Print the full contents of a profile.
pub fn profile_show(name: &str, p: &video_uploader::UploadProfile) {
    sub_header(&format!("Profile: {name}"));
    if let Some(ref v) = p.visibility {
        kv("Visibility", v);
    }
    if let Some(ref c) = p.category {
        kv("Category", c);
    }
    if let Some(k) = p.made_for_kids {
        kv("Made for kids", if k { "yes" } else { "no" });
    }
    if let Some(ref l) = p.license {
        kv("License", l);
    }
    if let Some(ref l) = p.language {
        kv("Language", l);
    }
    if let Some(s) = p.contains_synthetic_media {
        kv("Synthetic media", if s { "yes" } else { "no" });
    }
    if let Some(e) = p.embeddable {
        kv("Embeddable", if e { "yes" } else { "no" });
    }
    if let Some(v) = p.public_stats_viewable {
        kv("Public stats", if v { "yes" } else { "no" });
    }
    if let Some(ref t) = p.tags {
        kv("Tags", &t.join(", "));
    }
    if let Some(ref s) = p.description_suffix {
        kv("Desc suffix", s);
    }
    if let Some(ref d) = p.publish_at {
        kv("Publish at", d);
    }
    if let Some(ref d) = p.recording_date {
        kv("Recording date", d);
    }
    eprintln!();
}

/// Print profile removed confirmation.
pub fn profile_removed(name: &str) {
    success(&format!("Profile '{name}' removed"));
}

/// Print the batch dry run preview.
pub fn dry_run(entries: &[(String, String, Option<String>)]) {
    sub_header(&format!("Dry Run \u{2014} {} video(s)", entries.len()));
    for (i, (file, title, workspace)) in entries.iter().enumerate() {
        let ws = workspace.as_deref().unwrap_or("(default)");
        eprintln!("  {:>3}. [{}] {} \u{2192} {}", i + 1, ws, file, title);
    }
    eprintln!();
    warn("No videos were uploaded (dry run). Remove --dry-run to upload for real.");
}

/// Print the auth success message.
pub fn auth_success(workspace: &str) {
    eprintln!();
    success(&format!("Workspace '{workspace}' configured successfully!"));
}

/// Print the auth success message with channel info.
pub fn auth_success_with_channel(workspace: &str, channel_name: &str, channel_id: &str) {
    eprintln!();
    header("Authorization Complete");
    kv("Workspace", workspace);
    kv("Channel", channel_name);
    kv("Channel ID", channel_id);
    eprintln!();
    success("You can now upload to this channel.");
    eprintln!();
}

/// Print workspace operation results.
pub fn workspace_default_set(name: &str) {
    success(&format!("Default workspace set to '{name}'"));
}

pub fn workspace_renamed(old: &str, new: &str) {
    success(&format!("Workspace '{old}' renamed to '{new}'"));
}

pub fn workspace_removed(name: &str) {
    success(&format!("Workspace '{name}' removed"));
}

/// Print the channel info result.
pub fn channel_info(workspace: &str, channel_name: &str, channel_id: &str) {
    sub_header(&format!("Channel: {channel_name}"));
    kv("Workspace", workspace);
    kv("Channel ID", channel_id);
    eprintln!();
}
