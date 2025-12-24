//! RustMedia Settings

use serde::{Deserialize, Serialize};
use crate::models::{ImageSize, ResizeMode};

/// Media plugin settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaSettings {
    // Storage
    /// Storage backend (local, s3)
    pub storage_backend: String,
    /// Path for local storage
    pub storage_path: String,
    /// Base URL for media files
    pub base_url: String,

    // Upload limits
    /// Maximum file size in bytes
    pub max_file_size: u64,
    /// Allowed file extensions
    pub allowed_extensions: Vec<String>,
    /// Allowed MIME types
    pub allowed_mime_types: Vec<String>,

    // Image processing
    /// JPEG quality (1-100)
    pub jpeg_quality: u8,
    /// PNG compression level (0-9)
    pub png_compression: u8,
    /// WebP quality (1-100)
    pub webp_quality: u8,
    /// Maximum image width
    pub max_image_width: u32,
    /// Maximum image height
    pub max_image_height: u32,
    /// Auto-optimize images
    pub auto_optimize: bool,
    /// Strip EXIF metadata
    pub strip_metadata: bool,
    /// Convert to WebP
    pub convert_to_webp: bool,
    /// Enable progressive JPEG
    pub progressive_jpeg: bool,

    // Thumbnails
    /// Generate thumbnails
    pub generate_thumbnails: bool,
    /// Thumbnail sizes
    pub image_sizes: Vec<ImageSize>,

    // Organization
    /// Organize by date
    pub organize_by_date: bool,
    /// Date format for organization
    pub date_format: String,
    /// Slugify filenames
    pub slugify_filenames: bool,
    /// Deduplicate files by hash
    pub deduplicate: bool,

    // Security
    /// Scan uploads for malware
    pub scan_uploads: bool,
    /// Validate file contents
    pub validate_contents: bool,
    /// Maximum filename length
    pub max_filename_length: usize,

    // Chunked uploads
    /// Enable chunked uploads
    pub chunked_uploads: bool,
    /// Chunk size in bytes
    pub chunk_size: usize,
    /// Chunk upload expiry in hours
    pub chunk_expiry_hours: u32,

    // CDN
    /// CDN enabled
    pub cdn_enabled: bool,
    /// CDN base URL
    pub cdn_url: String,

    // Watermark
    /// Enable watermark
    pub watermark_enabled: bool,
    /// Watermark image path
    pub watermark_path: String,
    /// Watermark position
    pub watermark_position: String,
    /// Watermark opacity (0-100)
    pub watermark_opacity: u8,

    // S3 settings (when using S3 backend)
    /// S3 bucket name
    pub s3_bucket: String,
    /// S3 region
    pub s3_region: String,
    /// S3 access key
    pub s3_access_key: String,
    /// S3 secret key
    pub s3_secret_key: String,
    /// S3 endpoint (for compatible services)
    pub s3_endpoint: String,
    /// S3 path prefix
    pub s3_prefix: String,
}

impl Default for MediaSettings {
    fn default() -> Self {
        Self {
            // Storage
            storage_backend: "local".to_string(),
            storage_path: "uploads/media".to_string(),
            base_url: "/media".to_string(),

            // Upload limits
            max_file_size: 100 * 1024 * 1024, // 100MB
            allowed_extensions: vec![
                // Images
                "jpg".to_string(), "jpeg".to_string(), "png".to_string(),
                "gif".to_string(), "webp".to_string(), "svg".to_string(),
                "bmp".to_string(), "ico".to_string(),
                // Videos
                "mp4".to_string(), "webm".to_string(), "ogv".to_string(),
                "mov".to_string(), "avi".to_string(), "mkv".to_string(),
                // Audio
                "mp3".to_string(), "ogg".to_string(), "wav".to_string(),
                "flac".to_string(), "m4a".to_string(),
                // Documents
                "pdf".to_string(), "doc".to_string(), "docx".to_string(),
                "xls".to_string(), "xlsx".to_string(), "ppt".to_string(),
                "pptx".to_string(), "txt".to_string(), "csv".to_string(),
                // Archives
                "zip".to_string(), "rar".to_string(), "7z".to_string(),
                "tar".to_string(), "gz".to_string(),
            ],
            allowed_mime_types: vec![
                // Images
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "image/gif".to_string(),
                "image/webp".to_string(),
                "image/svg+xml".to_string(),
                "image/bmp".to_string(),
                "image/x-icon".to_string(),
                // Videos
                "video/mp4".to_string(),
                "video/webm".to_string(),
                "video/ogg".to_string(),
                "video/quicktime".to_string(),
                "video/x-msvideo".to_string(),
                // Audio
                "audio/mpeg".to_string(),
                "audio/ogg".to_string(),
                "audio/wav".to_string(),
                "audio/flac".to_string(),
                "audio/mp4".to_string(),
                // Documents
                "application/pdf".to_string(),
                "application/msword".to_string(),
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
                "application/vnd.ms-excel".to_string(),
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
                "application/vnd.ms-powerpoint".to_string(),
                "application/vnd.openxmlformats-officedocument.presentationml.presentation".to_string(),
                "text/plain".to_string(),
                "text/csv".to_string(),
                // Archives
                "application/zip".to_string(),
                "application/x-rar-compressed".to_string(),
                "application/x-7z-compressed".to_string(),
                "application/x-tar".to_string(),
                "application/gzip".to_string(),
            ],

            // Image processing
            jpeg_quality: 85,
            png_compression: 6,
            webp_quality: 80,
            max_image_width: 4096,
            max_image_height: 4096,
            auto_optimize: true,
            strip_metadata: true,
            convert_to_webp: false,
            progressive_jpeg: true,

            // Thumbnails
            generate_thumbnails: true,
            image_sizes: vec![
                ImageSize {
                    name: "thumbnail".to_string(),
                    width: 150,
                    height: 150,
                    mode: ResizeMode::Fill,
                    quality: 85,
                    enabled: true,
                },
                ImageSize {
                    name: "small".to_string(),
                    width: 300,
                    height: 300,
                    mode: ResizeMode::Fit,
                    quality: 85,
                    enabled: true,
                },
                ImageSize {
                    name: "medium".to_string(),
                    width: 600,
                    height: 600,
                    mode: ResizeMode::Fit,
                    quality: 85,
                    enabled: true,
                },
                ImageSize {
                    name: "large".to_string(),
                    width: 1200,
                    height: 1200,
                    mode: ResizeMode::Fit,
                    quality: 85,
                    enabled: true,
                },
            ],

            // Organization
            organize_by_date: true,
            date_format: "%Y/%m".to_string(),
            slugify_filenames: true,
            deduplicate: true,

            // Security
            scan_uploads: false,
            validate_contents: true,
            max_filename_length: 255,

            // Chunked uploads
            chunked_uploads: true,
            chunk_size: 5 * 1024 * 1024, // 5MB
            chunk_expiry_hours: 24,

            // CDN
            cdn_enabled: false,
            cdn_url: String::new(),

            // Watermark
            watermark_enabled: false,
            watermark_path: String::new(),
            watermark_position: "bottom-right".to_string(),
            watermark_opacity: 50,

            // S3
            s3_bucket: String::new(),
            s3_region: "us-east-1".to_string(),
            s3_access_key: String::new(),
            s3_secret_key: String::new(),
            s3_endpoint: String::new(),
            s3_prefix: String::new(),
        }
    }
}

impl MediaSettings {
    /// Load settings from file
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let settings: Self = serde_json::from_str(&content)?;
        Ok(settings)
    }

    /// Save settings to file
    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get effective base URL (CDN or regular)
    pub fn get_base_url(&self) -> &str {
        if self.cdn_enabled && !self.cdn_url.is_empty() {
            &self.cdn_url
        } else {
            &self.base_url
        }
    }

    /// Check if extension is allowed
    pub fn is_extension_allowed(&self, ext: &str) -> bool {
        self.allowed_extensions.iter().any(|e| e.eq_ignore_ascii_case(ext))
    }

    /// Check if MIME type is allowed
    pub fn is_mime_type_allowed(&self, mime: &str) -> bool {
        self.allowed_mime_types.iter().any(|m| m == mime)
    }

    /// Get enabled image sizes
    pub fn get_enabled_sizes(&self) -> Vec<&ImageSize> {
        self.image_sizes.iter().filter(|s| s.enabled).collect()
    }

    /// Validate settings
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.storage_path.is_empty() {
            errors.push("Storage path cannot be empty".to_string());
        }

        if self.max_file_size == 0 {
            errors.push("Max file size must be greater than 0".to_string());
        }

        if self.jpeg_quality == 0 || self.jpeg_quality > 100 {
            errors.push("JPEG quality must be between 1 and 100".to_string());
        }

        if self.png_compression > 9 {
            errors.push("PNG compression must be between 0 and 9".to_string());
        }

        if self.webp_quality == 0 || self.webp_quality > 100 {
            errors.push("WebP quality must be between 1 and 100".to_string());
        }

        if self.storage_backend == "s3" {
            if self.s3_bucket.is_empty() {
                errors.push("S3 bucket name is required".to_string());
            }
            if self.s3_access_key.is_empty() {
                errors.push("S3 access key is required".to_string());
            }
            if self.s3_secret_key.is_empty() {
                errors.push("S3 secret key is required".to_string());
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
