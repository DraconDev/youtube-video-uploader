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

/// Print the workspace list.
pub fn workspace_list(workspaces: &[(&str, bool)]) {
    if workspaces.is_empty() {
        info("No workspaces configured. Run: video-uploader auth");
    } else {
        sub_header("Workspaces");
        for (name, is_default) in workspaces {
            if *is_default {
                eprintln!("  \u{2022} {name} (default)");
            } else {
                eprintln!("  \u{2022} {name}");
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
        for (name, p) in profiles {
            let vis = p.visibility.as_deref().unwrap_or("(default)");
            let cat = p.category.as_deref().unwrap_or("(default)");
            let kids = p.made_for_kids
                .map(|b| if b { "yes" } else { "no" })
                .unwrap_or("-");
            let lic = p.license.as_deref().unwrap_or("(default)");
            let lang = p.language.as_deref().unwrap_or("-");
            eprintln!("  \u{2022} {name}");
            eprintln!("      vis={vis}  cat={cat}  kids={kids}  lic={lic}  lang={lang}");
        }
        eprintln!();
        info("Edit profiles at ~/.config/video-uploader/profiles/");
    }
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
