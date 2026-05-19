//! Pretty-print output for the youtube-uploader CLI.
//!
//! All user-facing output goes through this module for consistent formatting.
//! Uses Unicode icons for visual clarity:
//!
//! | Icon | Meaning       |
//! |------|---------------|
//! | ✔    | Success       |
//! | ✘    | Error         |
//! | ⚠    | Warning       |
//! | →    | Info          |
//! | ▸    | Link / action |
//! | •    | List item     |

// ─── Core formatting ────────────────────────────────────────────────

/// Print a section header in a box.
///
/// ```text
///   +------------------------------------------+
///   |            Upload Complete                |
///   +------------------------------------------+
/// ```
pub fn header(title: &str) {
    let width = title.len().max(40);
    let bar = "─".repeat(width + 2);
    eprintln!();
    eprintln!("  ╔{bar}╗");
    eprintln!("  ║ {} ║", pad_center(title, width));
    eprintln!("  ╚{bar}╝");
}

/// Print a section header with a colored accent line.
///
/// ```text
///   ▸ Profiles
///   ──────────
/// ```
pub fn sub_header(title: &str) {
    eprintln!();
    eprintln!("  \u{25B8} {title}");
    eprintln!("  {}", "─".repeat(title.len() + 2));
}

/// Pad a string to center it within a given width.
fn pad_center(s: &str, width: usize) -> String {
    let pad = width.saturating_sub(s.chars().count());
    let left = pad / 2;
    let right = pad - left;
    format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
}

// ─── Key-value display ──────────────────────────────────────────────

/// Print a key-value pair aligned under a header.
///
/// ```text
///        Title: My Awesome Video
///   Workspace: youtube
/// ```
pub fn kv(key: &str, value: &str) {
    eprintln!("  {:>14}  {}", format!("{key}:"), value);
}

/// Print a key-value pair where the value may be long and wraps.
///
/// ```text
///   Description:  This is a very long description that
///                 continues on the next line indented.
/// ```
pub fn kv_wrap(key: &str, value: &str) {
    let indent = 17; // 14 + "  " alignment
    let max_width = 80 - indent;
    let mut first = true;
    for chunk in wrap_text(value, max_width) {
        if first {
            eprintln!("  {:>14}  {}", format!("{key}:"), chunk);
            first = false;
        } else {
            eprintln!("{}{chunk}", " ".repeat(indent));
        }
    }
}

/// Print a key-value pair with a warning/trailing badge.
pub fn kv_badge(key: &str, value: &str, badge: &str) {
    eprintln!("  {:>14}  {} {}", format!("{key}:"), value, badge);
}

/// Print a key-value pair where the value is "yes" or "no" with icon.
pub fn kv_bool(key: &str, value: bool) {
    let icon = if value { "\u{2714}" } else { "\u{2718}" };
    let label = if value { "yes" } else { "no" };
    eprintln!("  {:>14}  {} {}", format!("{key}:"), label, icon);
}

// ─── Status messages ────────────────────────────────────────────────

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

// ─── List helpers ───────────────────────────────────────────────────

/// Print a list item with a bullet.
#[allow(dead_code)]
pub fn bullet(text: &str) {
    eprintln!("  \u{2022} {text}");
}

/// Print a numbered item.
pub fn numbered(n: usize, text: &str) {
    eprintln!("  {:>2}. {text}", n);
}

/// Print a labeled list item (key: value).
#[allow(dead_code)]
pub fn bullet_kv(label: &str, value: &str) {
    eprintln!("  \u{2022} {}: {}", label, value);
}

/// Print a blank line (thin spacer).
#[allow(dead_code)]
pub fn spacer() {
    eprintln!();
}

/// Print a divider line.
#[allow(dead_code)]
pub fn divider() {
    eprintln!("  {}", "─".repeat(50));
}

// ─── Feature banner ─────────────────────────────────────────────────

/// Print a startup banner showing what's being done.
///
/// ```text
///   ╔══════════════════════════════════╗
///   ║        youtube-uploader v0.4       ║
///   ╚══════════════════════════════════╝
/// ```
#[allow(dead_code)]
pub fn version_banner(version: &str) {
    let label = format!("youtube-uploader v{version}");
    let width = label.len().max(30);
    let bar = "═".repeat(width + 2);
    eprintln!();
    eprintln!("  ╔{bar}╗");
    eprintln!("  ║ {} ║", pad_center(&label, width));
    eprintln!("  ╚{bar}╝");
    eprintln!();
}

// ─── Auth flow ──────────────────────────────────────────────────────

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
    eprintln!("     ╔{}╗", "═".repeat(13));
    eprintln!("     ║ {} ║", pad_center(code, 11));
    eprintln!("     ╚{}╝", "═".repeat(13));
    eprintln!();
    info("Waiting for authorization... (Ctrl+C to cancel)");
    eprintln!();
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

// ─── Upload results ─────────────────────────────────────────────────

/// Print the upload result.
pub fn upload_result(workspace: &str, video_id: &str, url: &str, title: &str, visibility: &str) {
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

/// Details for an upload result display.
#[allow(dead_code)]
pub struct UploadResultDetails<'a> {
    pub workspace: &'a str,
    pub video_id: &'a str,
    pub url: &'a str,
    pub title: &'a str,
    pub visibility: &'a str,
    pub description: Option<&'a str>,
    pub tags: Option<&'a [String]>,
    pub category: Option<&'a str>,
}

/// Print the upload result with full metadata.
#[allow(dead_code)]
pub fn upload_result_full(d: &UploadResultDetails<'_>) {
    eprintln!();
    header("Upload Complete");
    kv("Title", d.title);
    kv("Workspace", d.workspace);
    kv("Visibility", d.visibility);
    kv("Video ID", d.video_id);
    if let Some(desc) = d.description.filter(|s| !s.is_empty()) {
        kv_wrap("Description", desc);
    }
    if let Some(t) = d.tags.filter(|t| !t.is_empty()) {
        kv("Tags", &t.join(", "));
    }
    if let Some(c) = d.category {
        kv("Category", c);
    }
    eprintln!();
    eprintln!("  \u{25B8} {}", d.url);
    eprintln!();
}

/// Print the upload result as JSON (for automation/CI).
pub fn upload_result_json(result: &youtube_uploader::UploadResult) {
    match serde_json::to_string_pretty(result) {
        Ok(json) => println!("{json}"),
        Err(e) => eprintln!("Error serializing result: {e}"),
    }
}

// ─── Batch results ──────────────────────────────────────────────────

/// Print the batch progress line.
pub fn batch_progress(current: usize, total: usize, workspace: &str, title: &str) {
    eprintln!(
        "\n  [{}/{}] [{}] Uploading: {}",
        current, total, workspace, title
    );
}

/// Print a batch item result.
pub fn batch_item_result(url: &str, video_id: &str) {
    success(&format!("{url} ({video_id})"));
}

/// Print a batch item error.
pub fn batch_item_error(err: &str) {
    print_error(err);
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

/// Print the batch result summary with per-video details.
#[allow(dead_code)]
pub fn batch_summary_detailed(
    total: usize,
    succeeded: usize,
    failed: usize,
    results: &[(String, bool, Option<String>)],
) {
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
    spacer();

    sub_header("Results");
    for (title, ok, url_or_err) in results {
        if *ok && url_or_err.is_some() {
            eprintln!(
                "  \u{2714} {} \u{2192} {}",
                title,
                url_or_err.as_deref().unwrap()
            );
        } else if *ok {
            success(title);
        } else {
            eprintln!(
                "  \u{2718} {} \u{2192} {}",
                title,
                url_or_err.as_deref().unwrap_or("unknown error")
            );
        }
    }
    eprintln!();
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

/// Print batch CSV column warning.
pub fn batch_csv_missing_columns(columns: &[&str]) {
    warn(&format!(
        "CSV manifest is missing optional columns: {}",
        columns.join(", ")
    ));
    info("Available: file, title, description, tags, visibility, workspace, profile");
}

// ─── Workspace display ──────────────────────────────────────────────

/// Print the workspace list with optional channel names.
pub fn workspace_list(workspaces: &[(&str, bool, Option<&str>)]) {
    if workspaces.is_empty() {
        info("No workspaces configured. Run: youtube-uploader auth");
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

// ─── Channel info ───────────────────────────────────────────────────

/// Print the channel info result.
pub fn channel_info(workspace: &str, channel_name: &str, channel_id: &str) {
    sub_header(&format!("Channel: {channel_name}"));
    kv("Workspace", workspace);
    kv("Channel ID", channel_id);
    eprintln!();
}

// ─── Profile display ────────────────────────────────────────────────

/// Print the profile list.
pub fn profile_list(profiles: &[(String, youtube_uploader::UploadProfile)]) {
    if profiles.is_empty() {
        info("No profiles found.");
        eprintln!("  Create one at: ~/.config/youtube-uploader/profiles/<name>.toml");
    } else {
        sub_header("Upload Profiles");
        for (name, _p) in profiles {
            eprintln!("  \u{2022} {name}");
        }
        eprintln!();
        info("Use 'profile show <name>' to see details");
        eprintln!("  Edit profiles at ~/.config/youtube-uploader/profiles/");
    }
}

/// Print the full contents of a profile.
pub fn profile_show(name: &str, p: &youtube_uploader::UploadProfile) {
    sub_header(&format!("Profile: {name}"));
    if let Some(ref v) = p.visibility {
        kv("Visibility", v);
    }
    if let Some(ref c) = p.category {
        kv("Category", c);
    }
    if let Some(k) = p.made_for_kids {
        kv_bool("Made for kids", k);
    }
    if let Some(ref l) = p.license {
        kv("License", l);
    }
    if let Some(ref l) = p.language {
        kv("Language", l);
    }
    if let Some(s) = p.contains_synthetic_media {
        kv_bool("Synthetic media", s);
    }
    if let Some(e) = p.embeddable {
        kv_bool("Embeddable", e);
    }
    if let Some(v) = p.public_stats_viewable {
        kv_bool("Public stats", v);
    }
    if let Some(ref t) = p.tags {
        kv("Tags", &t.join(", "));
    }
    if let Some(ref s) = p.description_suffix {
        kv_wrap("Desc suffix", s);
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

// ─── Validation / quota display ─────────────────────────────────────

/// Print quota information after an upload.
#[allow(dead_code)]
pub fn quota_info(used: u64, total: u64) {
    let pct = (used as f64 / total as f64 * 100.0) as u64;
    let bar_len = 20;
    let filled = (pct as usize * bar_len) / 100;
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_len - filled));
    eprintln!("  {:>14}  {bar} {pct}% ({used}/{total})", "API quota:");
}

/// Print validation errors.
pub fn validation_errors(errors: &[String]) {
    if errors.is_empty() {
        return;
    }
    sub_header(&format!("Validation \u{2718} {} error(s)", errors.len()));
    for (i, err) in errors.iter().enumerate() {
        numbered(i + 1, err);
    }
    eprintln!();
}

// ─── Text wrapping helper ───────────────────────────────────────────

/// Simple word-wrap: splits `text` into chunks of at most `max_width` chars.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() > max_width {
            lines.push(current);
            current = word.to_string();
        } else {
            current.push(' ');
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_text_short() {
        let lines = wrap_text("hello world", 80);
        assert_eq!(lines, vec!["hello world"]);
    }

    #[test]
    fn test_wrap_text_long() {
        let lines = wrap_text("the quick brown fox jumps over the lazy dog", 15);
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(line.len() <= 15, "line too long: '{line}'");
        }
    }

    #[test]
    fn test_wrap_text_empty() {
        let lines = wrap_text("", 80);
        assert_eq!(lines, vec![""]);
    }

    #[test]
    fn test_wrap_text_single_word() {
        let lines = wrap_text("supercalifragilisticexpialidocious", 10);
        // Single word longer than max_width is not split
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_kv_bool_format() {
        // Smoke test: just ensure these don't panic
        kv_bool("Feature", true);
        kv_bool("Feature", false);
    }

    #[test]
    fn test_header_does_not_panic() {
        header("Short");
        header("This is a very long header title that exceeds the minimum");
    }

    #[test]
    fn test_sub_header_does_not_panic() {
        sub_header("Profiles");
    }

    #[test]
    fn test_version_banner_does_not_panic() {
        version_banner("0.4.4");
    }

    #[test]
    fn test_validation_errors_format() {
        validation_errors(&[
            "Row 1: File not found".into(),
            "Row 3: Title cannot be empty".into(),
        ]);
    }

    #[test]
    fn test_validation_errors_empty_is_noop() {
        validation_errors(&[]);
    }

    #[test]
    fn test_batch_csv_missing_columns() {
        batch_csv_missing_columns(&["description", "tags"]);
    }
}
