//! RustMedia - Complete Media Management Plugin for RustPress
//!
//! RustMedia provides comprehensive media management including:
//!
//! - **File Upload**: Single, multiple, and chunked uploads
//! - **Image Processing**: Resize, crop, rotate, optimize
//! - **Thumbnails**: Automatic generation with customizable sizes
//! - **Folder Organization**: Hierarchical folder structure
//! - **Storage Backends**: Local filesystem and S3-compatible
//! - **Media Library**: Grid/list views with search and filters
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use rustmedia::RustMediaPlugin;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create plugin
//!     let plugin = RustMediaPlugin::new();
//!
//!     // Initialize
//!     plugin.initialize().await.expect("Failed to initialize");
//!
//!     // Upload a file
//!     let data = std::fs::read("photo.jpg").unwrap();
//!     let media = plugin.upload_file(data, "photo.jpg").await.unwrap();
//!     println!("Uploaded: {}", media.url);
//! }
//! ```
//!
//! ## Features
//!
//! - **Image Optimization**: Automatic compression and format conversion
//! - **WebP Support**: Convert images to WebP for smaller file sizes
//! - **EXIF Extraction**: Read and optionally strip image metadata
//! - **Deduplication**: Prevent duplicate uploads using content hashing
//! - **Chunked Uploads**: Support for large file uploads
//! - **URL Uploads**: Download and store files from URLs
//! - **Watermarks**: Apply watermarks to uploaded images
//!
//! ## Configuration
//!
//! ```rust,ignore
//! use rustmedia::{RustMediaPlugin, MediaSettings};
//!
//! let mut settings = MediaSettings::default();
//! settings.max_file_size = 50 * 1024 * 1024; // 50MB
//! settings.auto_optimize = true;
//! settings.convert_to_webp = true;
//!
//! let plugin = RustMediaPlugin::with_settings(settings);
//! ```

pub mod models;
pub mod services;
pub mod handlers;
pub mod admin;
pub mod settings;
pub mod plugin;

// Re-exports
pub use models::{
    MediaItem, MediaFolder, MediaType, MediaFilter, MediaListResponse,
    ImageSize, ResizeMode, ImageFormat, ImageTransformRequest,
    Thumbnail, ImageDimensions, FolderTreeNode, FolderBreadcrumb,
    UploadOptions, ChunkedUpload, ChunkInfo, OptimizationResult,
};

pub use services::{
    MediaService, FolderService, ImageService,
    StorageService, OptimizerService, UploadService,
};

pub use handlers::{
    MediaHandler, FolderHandler, UploadHandler,
};

pub use settings::MediaSettings;
pub use plugin::{RustMediaPlugin, PluginInfo, plugin_info};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the plugin with default settings
pub fn init() -> RustMediaPlugin {
    RustMediaPlugin::new()
}

/// Initialize with custom settings
pub fn init_with_settings(settings: MediaSettings) -> RustMediaPlugin {
    RustMediaPlugin::with_settings(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = RustMediaPlugin::new();
        assert_eq!(plugin.name(), "RustMedia");
    }

    #[test]
    fn test_default_settings() {
        let settings = MediaSettings::default();
        assert_eq!(settings.storage_backend, "local");
        assert!(settings.auto_optimize);
        assert!(settings.generate_thumbnails);
        assert_eq!(settings.jpeg_quality, 85);
    }

    #[test]
    fn test_settings_validation() {
        let mut settings = MediaSettings::default();
        assert!(settings.validate().is_ok());

        settings.jpeg_quality = 0;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_extension_check() {
        let settings = MediaSettings::default();
        assert!(settings.is_extension_allowed("jpg"));
        assert!(settings.is_extension_allowed("PNG"));
        assert!(!settings.is_extension_allowed("exe"));
    }

    #[test]
    fn test_mime_type_check() {
        let settings = MediaSettings::default();
        assert!(settings.is_mime_type_allowed("image/jpeg"));
        assert!(settings.is_mime_type_allowed("video/mp4"));
        assert!(!settings.is_mime_type_allowed("application/x-executable"));
    }

    #[tokio::test]
    async fn test_folder_service() {
        let service = FolderService::new();

        // Create folder
        let folder = service.create("Test Folder", None, None).await.unwrap();
        assert_eq!(folder.name, "Test Folder");
        assert_eq!(folder.slug, "test-folder");

        // Get folder
        let retrieved = service.get(folder.id).await.unwrap();
        assert_eq!(retrieved.id, folder.id);

        // Update folder
        let updated = service.update(folder.id, Some("Renamed Folder".to_string()), None).await.unwrap();
        assert_eq!(updated.name, "Renamed Folder");

        // Delete folder
        service.delete(folder.id, false).await.unwrap();
        assert!(service.get(folder.id).await.is_none());
    }

    #[tokio::test]
    async fn test_nested_folders() {
        let service = FolderService::new();

        // Create parent
        let parent = service.create("Parent", None, None).await.unwrap();

        // Create children
        let child1 = service.create("Child 1", Some(parent.id), None).await.unwrap();
        let child2 = service.create("Child 2", Some(parent.id), None).await.unwrap();

        // Get children
        let children = service.get_children(parent.id).await;
        assert_eq!(children.len(), 2);

        // Get tree
        let tree = service.get_tree().await;
        assert!(!tree.is_empty());

        // Get breadcrumbs
        let breadcrumbs = service.get_breadcrumbs(child1.id).await;
        assert_eq!(breadcrumbs.len(), 2); // Parent + Child

        // Cleanup
        service.delete(parent.id, true).await.unwrap();
    }

    #[test]
    fn test_image_resize_mode() {
        use models::ResizeMode;

        let fit = ResizeMode::Fit;
        let fill = ResizeMode::Fill;
        let exact = ResizeMode::Exact;

        // Test enum variants exist
        assert!(matches!(fit, ResizeMode::Fit));
        assert!(matches!(fill, ResizeMode::Fill));
        assert!(matches!(exact, ResizeMode::Exact));
    }

    #[test]
    fn test_media_type_detection() {
        use models::MediaType;

        let image = MediaType::from_mime("image/jpeg");
        assert!(matches!(image, MediaType::Image));

        let video = MediaType::from_mime("video/mp4");
        assert!(matches!(video, MediaType::Video));

        let audio = MediaType::from_mime("audio/mpeg");
        assert!(matches!(audio, MediaType::Audio));

        let document = MediaType::from_mime("application/pdf");
        assert!(matches!(document, MediaType::Document));
    }

    #[test]
    fn test_plugin_info() {
        let info = plugin_info();
        assert_eq!(info.name, "RustMedia");
        assert!(!info.routes.is_empty());
        assert!(!info.hooks.is_empty());
    }

    #[test]
    fn test_image_sizes() {
        let settings = MediaSettings::default();
        let sizes = settings.get_enabled_sizes();
        assert!(!sizes.is_empty());

        // Check default sizes exist
        let names: Vec<&str> = sizes.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"thumbnail"));
        assert!(names.contains(&"small"));
        assert!(names.contains(&"medium"));
        assert!(names.contains(&"large"));
    }
}
