use proptest::prelude::*;
use youtube_uploader::{VideoUpload, validation::validate};

proptest! {
    #[test]
    fn validate_input_fuzz(data: Vec<u8>, ext: String, title: String) {
        let ext = if ext.is_empty() || ext.contains('/') || ext.contains('\\') || ext.contains('\0') {
            "mp4".to_string()
        } else {
            ext
        };

        let temp_dir_result = tempfile::TempDir::new();
        prop_assume!(temp_dir_result.is_ok(), "Failed to create temp dir");
        let temp_dir = temp_dir_result.unwrap();
        let file_path = temp_dir.path().join(format!("video.{}", ext));

        let write_result = std::fs::write(&file_path, &data);
        prop_assume!(write_result.is_ok(), "Failed to write test file");

        let video = VideoUpload::new(file_path.to_str().unwrap_or("/tmp/video.mp4"), &title);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _ = rt.block_on(validate(&video));
    }
}
