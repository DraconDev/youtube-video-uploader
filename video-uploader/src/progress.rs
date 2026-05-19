use std::io::IsTerminal as _;

use crate::{UploadError, UploadResult};

/// Trait for receiving upload progress updates.
///
/// Implement this to show custom progress indicators during uploads.
///
/// # Examples
///
/// ```
/// use video_uploader::{ProgressListener, UploadError, UploadResult};
///
/// struct MyProgress;
///
/// impl ProgressListener for MyProgress {
///     fn on_progress(&self, uploaded: u64, total: u64) {
///         eprintln!("  {:.1}% ({}/{})", uploaded as f64 / total as f64 * 100.0, uploaded, total);
///     }
///     fn on_complete(&self, result: &UploadResult) {
///         eprintln!("Done: {}", result.url);
///     }
///     fn on_error(&self, error: &UploadError) {
///         eprintln!("Error: {error}");
///     }
/// }
/// ```
pub trait ProgressListener: Send + Sync {
    /// Called periodically with bytes uploaded and total file size.
    fn on_progress(&self, uploaded: u64, total: u64);

    /// Called when upload completes successfully.
    fn on_complete(&self, result: &UploadResult);

    /// Called when upload fails.
    fn on_error(&self, error: &UploadError);
}

/// A no-op progress listener for when you don't care about progress.
///
/// Useful for background/batch uploads where no output is desired.
///
/// ```
/// use video_uploader::NoopProgressListener;
/// use video_uploader::ProgressListener;
///
/// let listener = NoopProgressListener;
/// listener.on_progress(50, 100); // does nothing
/// ```
pub struct NoopProgressListener;

impl ProgressListener for NoopProgressListener {
    fn on_progress(&self, _uploaded: u64, _total: u64) {}
    fn on_complete(&self, _result: &UploadResult) {}
    fn on_error(&self, _error: &UploadError) {}
}

/// A progress listener that prints to stderr.
/// Uses carriage return (`\r`) when attached to a TTY for inline progress,
/// falls back to full-line output when output is piped/redirected.
/// Shows upload speed and ETA when progress is reported multiple times.
pub struct StderrProgressListener {
    start: std::time::Instant,
}

impl Default for StderrProgressListener {
    fn default() -> Self {
        Self::new()
    }
}

impl StderrProgressListener {
    /// Create a new stderr progress listener.
    pub fn new() -> Self {
        Self {
            start: std::time::Instant::now(),
        }
    }
}

impl ProgressListener for StderrProgressListener {
    fn on_progress(&self, uploaded: u64, total: u64) {
        if total > 0 {
            let pct = (uploaded as f64 / total as f64) * 100.0;
            let elapsed = self.start.elapsed().as_secs_f64();

            // Calculate speed and ETA
            let speed_str = if elapsed > 0.0 && uploaded > 0 {
                let speed = uploaded as f64 / elapsed;
                format_speed(speed)
            } else {
                "--".to_string()
            };

            let eta_str = if uploaded > 0 && uploaded < total && elapsed > 0.0 {
                let speed = uploaded as f64 / elapsed;
                let remaining_bytes = total - uploaded;
                let eta_secs = remaining_bytes as f64 / speed;
                format_duration(eta_secs)
            } else {
                "--".to_string()
            };

            if std::io::stderr().is_terminal() {
                eprint!(
                    "\r  {:>6.2}% {}/s  ETA {} ({}/{})",
                    pct, speed_str, eta_str, uploaded, total
                );
            } else {
                eprintln!(
                    "  {:>6.2}% {}/s  ETA {} ({}/{})",
                    pct, speed_str, eta_str, uploaded, total
                );
            }
        }
    }

    fn on_complete(&self, result: &UploadResult) {
        let elapsed = self.start.elapsed();
        if std::io::stderr().is_terminal() {
            eprintln!(
                "\n  {} uploaded to {}: {}",
                format_duration(elapsed.as_secs_f64()),
                result.workspace,
                result.url
            );
        } else {
            eprintln!(
                "[complete] {}: {} ({})",
                result.workspace,
                result.url,
                format_duration(elapsed.as_secs_f64())
            );
        }
    }

    fn on_error(&self, error: &UploadError) {
        eprintln!("  Upload failed: {}", error);
    }
}

/// Format bytes/second as a human-readable string.
fn format_speed(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1_000_000_000.0 {
        format!("{:.1} GB", bytes_per_sec / 1_000_000_000.0)
    } else if bytes_per_sec >= 1_000_000.0 {
        format!("{:.1} MB", bytes_per_sec / 1_000_000.0)
    } else if bytes_per_sec >= 1_000.0 {
        format!("{:.0} KB", bytes_per_sec / 1_000.0)
    } else {
        format!("{:.0} B", bytes_per_sec)
    }
}

/// Format seconds as a human-readable duration.
fn format_duration(secs: f64) -> String {
    if secs.is_nan() || secs.is_infinite() || secs < 0.0 {
        return "--".to_string();
    }
    let total_secs = secs as u64;
    if total_secs < 60 {
        format!("{}s", total_secs)
    } else if total_secs < 3600 {
        format!("{}m {}s", total_secs / 60, total_secs % 60)
    } else {
        format!("{}h {}m", total_secs / 3600, (total_secs % 3600) / 60)
    }
}
