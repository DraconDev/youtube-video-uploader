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
pub struct StderrProgressListener;

impl ProgressListener for StderrProgressListener {
    fn on_progress(&self, uploaded: u64, total: u64) {
        if total > 0 {
            let pct = (uploaded as f64 / total as f64) * 100.0;
            if std::io::stderr().is_terminal() {
                eprint!("\r  {:>6.2}% ({}/{} bytes)", pct, uploaded, total);
            } else {
                eprintln!("  {:>6.2}% ({}/{} bytes)", pct, uploaded, total);
            }
        }
    }

    fn on_complete(&self, result: &UploadResult) {
        if std::io::stderr().is_terminal() {
            eprintln!("\n  Uploaded to {}: {}", result.workspace, result.url);
        } else {
            eprintln!("[complete] {}: {}", result.workspace, result.url);
        }
    }

    fn on_error(&self, error: &UploadError) {
        eprintln!("  Upload failed: {}", error);
    }
}
