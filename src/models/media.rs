//! Media Models
//!
//! Core media item structures.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Media item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    /// Unique ID
    pub id: Uuid,
    /// Original filename
    pub filename: String,
    /// Sanitized filename (URL-safe)
    pub slug: String,
    /// File title/alt text
    pub title: Option<String>,
    /// Description/caption
    pub description: Option<String>,
    /// Alt text for images
    pub alt_text: Option<String>,
    /// MIME type
    pub mime_type: String,
    /// Media type category
    pub media_type: MediaType,
    /// File size in bytes
    pub size: u64,
    /// File extension
    pub extension: String,
    /// Relative path from uploads root
    pub path: String,
    /// Full URL
    pub url: String,
    /// Folder ID (optional)
    pub folder_id: Option<Uuid>,
    /// Image dimensions (for images)
    pub dimensions: Option<ImageDimensions>,
    /// Duration in seconds (for audio/video)
    pub duration: Option<f64>,
    /// Metadata (EXIF, etc.)
    pub metadata: MediaMetadata,
    /// Generated thumbnails
    pub thumbnails: Vec<Thumbnail>,
    /// Upload timestamp
    pub uploaded_at: DateTime<Utc>,
    /// Last modified timestamp
    pub updated_at: DateTime<Utc>,
    /// Uploader user ID
    pub uploaded_by: Option<Uuid>,
    /// Usage count
    pub usage_count: u32,
    /// Tags
    pub tags: Vec<String>,
    /// Custom fields
    pub custom: HashMap<String, String>,
    /// Content hash for deduplication
    pub content_hash: String,
    /// Is soft deleted
    pub deleted: bool,
}

impl MediaItem {
    /// Create a new media item
    pub fn new(
        filename: impl Into<String>,
        mime_type: impl Into<String>,
        size: u64,
        path: impl Into<String>,
    ) -> Self {
        let filename = filename.into();
        let mime_type = mime_type.into();
        let path = path.into();

        let extension = std::path::Path::new(&filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let slug = sanitize_filename(&filename);
        let media_type = MediaType::from_mime(&mime_type);

        let now = Utc::now();

        Self {
            id: Uuid::now_v7(),
            filename,
            slug,
            title: None,
            description: None,
            alt_text: None,
            mime_type,
            media_type,
            size,
            extension,
            path,
            url: String::new(),
            folder_id: None,
            dimensions: None,
            duration: None,
            metadata: MediaMetadata::default(),
            thumbnails: Vec::new(),
            uploaded_at: now,
            updated_at: now,
            uploaded_by: None,
            usage_count: 0,
            tags: Vec::new(),
            custom: HashMap::new(),
            content_hash: String::new(),
            deleted: false,
        }
    }

    /// Check if item is an image
    pub fn is_image(&self) -> bool {
        matches!(self.media_type, MediaType::Image)
    }

    /// Check if item is a video
    pub fn is_video(&self) -> bool {
        matches!(self.media_type, MediaType::Video)
    }

    /// Check if item is audio
    pub fn is_audio(&self) -> bool {
        matches!(self.media_type, MediaType::Audio)
    }

    /// Check if item is a document
    pub fn is_document(&self) -> bool {
        matches!(self.media_type, MediaType::Document)
    }

    /// Get formatted file size
    pub fn formatted_size(&self) -> String {
        format_bytes(self.size)
    }

    /// Get thumbnail URL by size
    pub fn thumbnail_url(&self, size: &str) -> Option<&str> {
        self.thumbnails
            .iter()
            .find(|t| t.size_name == size)
            .map(|t| t.url.as_str())
    }
}

/// Media type category
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MediaType {
    /// Image files
    Image,
    /// Video files
    Video,
    /// Audio files
    Audio,
    /// Documents (PDF, DOC, etc.)
    Document,
    /// Archives (ZIP, RAR, etc.)
    Archive,
    /// Other files
    Other,
}

impl MediaType {
    /// Determine media type from MIME type
    pub fn from_mime(mime: &str) -> Self {
        if mime.starts_with("image/") {
            Self::Image
        } else if mime.starts_with("video/") {
            Self::Video
        } else if mime.starts_with("audio/") {
            Self::Audio
        } else if mime.starts_with("application/pdf")
            || mime.contains("document")
            || mime.contains("spreadsheet")
            || mime.contains("presentation")
            || mime.starts_with("text/")
        {
            Self::Document
        } else if mime.contains("zip")
            || mime.contains("tar")
            || mime.contains("rar")
            || mime.contains("gzip")
        {
            Self::Archive
        } else {
            Self::Other
        }
    }

    /// Get icon name for this type
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
            Self::Audio => "music",
            Self::Document => "file-text",
            Self::Archive => "archive",
            Self::Other => "file",
        }
    }
}

impl Default for MediaType {
    fn default() -> Self {
        Self::Other
    }
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Image => write!(f, "Image"),
            Self::Video => write!(f, "Video"),
            Self::Audio => write!(f, "Audio"),
            Self::Document => write!(f, "Document"),
            Self::Archive => write!(f, "Archive"),
            Self::Other => write!(f, "Other"),
        }
    }
}

/// Image dimensions
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ImageDimensions {
    pub width: u32,
    pub height: u32,
}

impl ImageDimensions {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Get aspect ratio
    pub fn aspect_ratio(&self) -> f64 {
        self.width as f64 / self.height as f64
    }

    /// Check if portrait orientation
    pub fn is_portrait(&self) -> bool {
        self.height > self.width
    }

    /// Check if landscape orientation
    pub fn is_landscape(&self) -> bool {
        self.width > self.height
    }

    /// Check if square
    pub fn is_square(&self) -> bool {
        self.width == self.height
    }
}

/// Thumbnail variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thumbnail {
    /// Size name (e.g., "small", "medium", "large")
    pub size_name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Relative path
    pub path: String,
    /// Full URL
    pub url: String,
    /// File size in bytes
    pub size: u64,
}

/// Media metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MediaMetadata {
    /// EXIF data for images
    pub exif: Option<ExifData>,
    /// Video/audio codec info
    pub codec: Option<String>,
    /// Bitrate for audio/video
    pub bitrate: Option<u32>,
    /// Sample rate for audio
    pub sample_rate: Option<u32>,
    /// Frame rate for video
    pub frame_rate: Option<f64>,
    /// Artist/author
    pub artist: Option<String>,
    /// Copyright info
    pub copyright: Option<String>,
    /// Creation date from metadata
    pub created_date: Option<DateTime<Utc>>,
    /// GPS coordinates
    pub location: Option<GpsLocation>,
    /// Custom metadata
    pub custom: HashMap<String, String>,
}

/// EXIF data from images
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExifData {
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub exposure_time: Option<String>,
    pub f_number: Option<f64>,
    pub iso: Option<u32>,
    pub focal_length: Option<f64>,
    pub flash: Option<bool>,
    pub orientation: Option<u32>,
    pub date_taken: Option<DateTime<Utc>>,
    pub software: Option<String>,
}

/// GPS location data
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GpsLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
}

/// Upload request
#[derive(Debug, Clone, Deserialize)]
pub struct UploadRequest {
    /// File data (base64 encoded)
    pub data: Option<String>,
    /// Original filename
    pub filename: String,
    /// Target folder ID
    pub folder_id: Option<Uuid>,
    /// Title
    pub title: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Alt text
    pub alt_text: Option<String>,
    /// Tags
    pub tags: Option<Vec<String>>,
}

/// Upload response
#[derive(Debug, Clone, Serialize)]
pub struct UploadResponse {
    pub success: bool,
    pub media: Option<MediaItem>,
    pub error: Option<String>,
}

/// Media filter options
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MediaFilter {
    /// Filter by media type
    pub media_type: Option<MediaType>,
    /// Filter by folder
    pub folder_id: Option<Uuid>,
    /// Filter by tags
    pub tags: Option<Vec<String>>,
    /// Search in filename/title
    pub search: Option<String>,
    /// Date range start
    pub date_from: Option<DateTime<Utc>>,
    /// Date range end
    pub date_to: Option<DateTime<Utc>>,
    /// Minimum size
    pub min_size: Option<u64>,
    /// Maximum size
    pub max_size: Option<u64>,
    /// Include deleted items
    pub include_deleted: Option<bool>,
    /// Sort field
    pub sort_by: Option<String>,
    /// Sort direction
    pub sort_order: Option<String>,
    /// Page number
    pub page: Option<u32>,
    /// Items per page
    pub per_page: Option<u32>,
}

/// Media list response
#[derive(Debug, Clone, Serialize)]
pub struct MediaListResponse {
    pub items: Vec<MediaItem>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

/// Sanitize filename for URL safety
pub fn sanitize_filename(filename: &str) -> String {
    let re = regex::Regex::new(r"[^a-zA-Z0-9._-]").unwrap();
    let name = std::path::Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");

    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let sanitized = re.replace_all(name, "-").to_lowercase();

    if ext.is_empty() {
        sanitized.to_string()
    } else {
        format!("{}.{}", sanitized, ext.to_lowercase())
    }
}

/// Format bytes to human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_type_from_mime() {
        assert_eq!(MediaType::from_mime("image/jpeg"), MediaType::Image);
        assert_eq!(MediaType::from_mime("video/mp4"), MediaType::Video);
        assert_eq!(MediaType::from_mime("audio/mpeg"), MediaType::Audio);
        assert_eq!(MediaType::from_mime("application/pdf"), MediaType::Document);
        assert_eq!(MediaType::from_mime("application/zip"), MediaType::Archive);
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("My File Name.jpg"), "my-file-name.jpg");
        assert_eq!(sanitize_filename("test@#$%.png"), "test----.png");
        assert_eq!(sanitize_filename("normal.pdf"), "normal.pdf");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1572864), "1.50 MB");
    }
}
