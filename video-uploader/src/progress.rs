use crate::{UploadError, UploadResult};

/// Trait for receiving upload progress updates.
pub trait ProgressListener: Send + Sync {
    /// Called periodically with bytes uploaded and total file size.
    fn on_progress(&self, uploaded: u64, total: u64);

    /// Called when upload completes successfully.
    fn on_complete(&self, result: &UploadResult);

    /// Called when upload fails.
    fn on_error(&self, error: &UploadError);
}

/// A no-op progress listener for when you don't care about progress.
pub struct NoopProgressListener;

impl ProgressListener for NoopProgressListener {
    fn on_progress(&self, _uploaded: u64, _total: u64) {}
    fn on_complete(&self, _result: &UploadResult) {}
    fn on_error(&self, _error: &UploadError) {}
}

/// A progress listener that prints to stderr.
pub struct StderrProgressListener;

impl ProgressListener for StderrProgressListener {
    fn on_progress(&self, uploaded: u64, total: u64) {
        if total > 0 {
            let pct = (uploaded as f64 / total as f64) * 100.0;
            eprint!("\r  {:>6.2}% ({}/{} bytes)", pct, uploaded, total);
        }
    }

    fn on_complete(&self, result: &UploadResult) {
        eprintln!("\n  Uploaded to {}: {}", result.platform, result.url);
    }

    fn on_error(&self, error: &UploadError) {
        eprintln!("\n  Upload failed: {}", error);
    }
}
