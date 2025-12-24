//! Upload Service
//!
//! File upload handling with validation and processing.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

use crate::models::{MediaItem, UploadOptions, ChunkedUpload, ChunkInfo, ImageFormat};
use super::storage::StorageService;
use super::image::ImageService;
use super::media::MediaService;
use super::optimizer::OptimizerService;

/// Upload service error
#[derive(Debug, thiserror::Error)]
pub enum UploadError {
    #[error("File too large: {0} bytes (max: {1})")]
    FileTooLarge(u64, u64),
    #[error("File type not allowed: {0}")]
    TypeNotAllowed(String),
    #[error("Invalid file: {0}")]
    InvalidFile(String),
    #[error("Upload not found: {0}")]
    NotFound(String),
    #[error("Upload expired")]
    Expired,
    #[error("Chunk missing: {0}")]
    ChunkMissing(usize),
    #[error("Storage error: {0}")]
    Storage(#[from] super::storage::StorageError),
    #[error("Image error: {0}")]
    Image(#[from] super::image::ImageError),
    #[error("Media error: {0}")]
    Media(#[from] super::media::MediaError),
    #[error("Network error: {0}")]
    Network(String),
}

/// Upload settings
#[derive(Debug, Clone)]
pub struct UploadSettings {
    /// Maximum file size in bytes
    pub max_file_size: u64,
    /// Allowed MIME types
    pub allowed_types: Vec<String>,
    /// Allowed extensions
    pub allowed_extensions: Vec<String>,
    /// Chunk size for chunked uploads
    pub chunk_size: usize,
    /// Chunk upload expiry duration
    pub chunk_expiry_hours: u32,
    /// Auto-optimize images
    pub auto_optimize: bool,
    /// Auto-generate thumbnails
    pub auto_thumbnails: bool,
}

impl Default for UploadSettings {
    fn default() -> Self {
        Self {
            max_file_size: 100 * 1024 * 1024, // 100MB
            allowed_types: vec![
                // Images
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "image/gif".to_string(),
                "image/webp".to_string(),
                "image/svg+xml".to_string(),
                "image/bmp".to_string(),
                "image/tiff".to_string(),
                // Videos
                "video/mp4".to_string(),
                "video/webm".to_string(),
                "video/ogg".to_string(),
                "video/quicktime".to_string(),
                // Audio
                "audio/mpeg".to_string(),
                "audio/ogg".to_string(),
                "audio/wav".to_string(),
                "audio/webm".to_string(),
                // Documents
                "application/pdf".to_string(),
                "application/msword".to_string(),
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
                "application/vnd.ms-excel".to_string(),
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
                "application/vnd.ms-powerpoint".to_string(),
                "application/vnd.openxmlformats-officedocument.presentationml.presentation".to_string(),
                // Archives
                "application/zip".to_string(),
                "application/x-rar-compressed".to_string(),
                "application/x-7z-compressed".to_string(),
                // Text
                "text/plain".to_string(),
                "text/csv".to_string(),
            ],
            allowed_extensions: vec![
                // Images
                "jpg".to_string(), "jpeg".to_string(), "png".to_string(),
                "gif".to_string(), "webp".to_string(), "svg".to_string(),
                "bmp".to_string(), "tiff".to_string(), "tif".to_string(),
                // Videos
                "mp4".to_string(), "webm".to_string(), "ogv".to_string(),
                "mov".to_string(), "avi".to_string(),
                // Audio
                "mp3".to_string(), "ogg".to_string(), "wav".to_string(),
                "weba".to_string(), "flac".to_string(),
                // Documents
                "pdf".to_string(), "doc".to_string(), "docx".to_string(),
                "xls".to_string(), "xlsx".to_string(), "ppt".to_string(),
                "pptx".to_string(),
                // Archives
                "zip".to_string(), "rar".to_string(), "7z".to_string(),
                // Text
                "txt".to_string(), "csv".to_string(),
            ],
            chunk_size: 5 * 1024 * 1024, // 5MB chunks
            chunk_expiry_hours: 24,
            auto_optimize: true,
            auto_thumbnails: true,
        }
    }
}

/// Upload service
pub struct UploadService {
    /// Storage service
    storage: Arc<StorageService>,
    /// Image service
    image_service: Arc<ImageService>,
    /// Media service
    media_service: Arc<MediaService>,
    /// Optimizer service
    optimizer: Arc<OptimizerService>,
    /// Settings
    settings: UploadSettings,
    /// Chunked uploads in progress
    chunked_uploads: Arc<RwLock<HashMap<Uuid, ChunkedUpload>>>,
}

impl UploadService {
    /// Create a new upload service
    pub fn new(
        storage: Arc<StorageService>,
        image_service: Arc<ImageService>,
        media_service: Arc<MediaService>,
        optimizer: Arc<OptimizerService>,
    ) -> Self {
        Self {
            storage,
            image_service,
            media_service,
            optimizer,
            settings: UploadSettings::default(),
            chunked_uploads: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Configure settings
    pub fn configure(&mut self, settings: UploadSettings) {
        self.settings = settings;
    }

    /// Upload a file
    pub async fn upload(
        &self,
        data: Vec<u8>,
        filename: &str,
        options: UploadOptions,
        user_id: Option<Uuid>,
    ) -> Result<MediaItem, UploadError> {
        // Validate file
        let mime_type = self.detect_mime_type(&data, filename);
        self.validate_file(filename, data.len() as u64, Some(&mime_type))?;

        // Process image if applicable
        let processed_data = if self.is_image(&mime_type) && options.optimize {
            self.optimizer.resize_and_optimize(&data, 2048, 2048)
                .await
                .map(|o| o.data)
                .unwrap_or(data)
        } else {
            data
        };

        // Upload via media service
        let media = self.media_service.upload(
            processed_data,
            filename,
            options,
            user_id,
        ).await?;

        Ok(media)
    }

    /// Initialize chunked upload
    pub async fn init_chunked_upload(
        &self,
        filename: &str,
        total_size: u64,
        chunk_size: usize,
        total_chunks: usize,
        mime_type: Option<String>,
        folder_id: Option<Uuid>,
        user_id: Option<Uuid>,
    ) -> Result<ChunkedUpload, UploadError> {
        // Validate
        if total_size > self.settings.max_file_size {
            return Err(UploadError::FileTooLarge(total_size, self.settings.max_file_size));
        }

        let ext = std::path::Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if !self.settings.allowed_extensions.contains(&ext) {
            return Err(UploadError::TypeNotAllowed(ext));
        }

        // Create chunks info
        let chunks: Vec<ChunkInfo> = (0..total_chunks)
            .map(|i| {
                let start = i * chunk_size;
                let end = std::cmp::min(start + chunk_size, total_size as usize);
                ChunkInfo {
                    index: i,
                    start,
                    end,
                    size: end - start,
                    received: false,
                    checksum: None,
                }
            })
            .collect();

        let upload = ChunkedUpload {
            id: Uuid::now_v7(),
            filename: filename.to_string(),
            total_size,
            chunk_size,
            total_chunks,
            chunks,
            mime_type,
            folder_id,
            user_id,
            temp_path: format!("temp/chunks/{}", Uuid::now_v7()),
            started_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(self.settings.chunk_expiry_hours as i64),
        };

        // Store
        let mut uploads = self.chunked_uploads.write().await;
        uploads.insert(upload.id, upload.clone());

        // Create temp directory
        self.storage.create_directory(&upload.temp_path).await?;

        Ok(upload)
    }

    /// Upload a chunk
    pub async fn upload_chunk(
        &self,
        upload_id: Uuid,
        chunk_index: usize,
        data: Vec<u8>,
    ) -> Result<ChunkedUpload, UploadError> {
        let mut uploads = self.chunked_uploads.write().await;

        let upload = uploads.get_mut(&upload_id)
            .ok_or_else(|| UploadError::NotFound(upload_id.to_string()))?;

        // Check expiry
        if Utc::now() > upload.expires_at {
            uploads.remove(&upload_id);
            return Err(UploadError::Expired);
        }

        // Validate chunk index
        if chunk_index >= upload.total_chunks {
            return Err(UploadError::InvalidFile(format!("Invalid chunk index: {}", chunk_index)));
        }

        // Save chunk to temp storage
        let chunk_path = format!("{}/chunk_{}", upload.temp_path, chunk_index);
        self.storage.write(&chunk_path, &data).await?;

        // Update chunk info
        if let Some(chunk) = upload.chunks.get_mut(chunk_index) {
            chunk.received = true;
            chunk.checksum = Some(format!("{:x}", md5::compute(&data)));
        }

        Ok(upload.clone())
    }

    /// Complete chunked upload
    pub async fn complete_chunked_upload(&self, upload_id: Uuid) -> Result<MediaItem, UploadError> {
        let upload = {
            let uploads = self.chunked_uploads.read().await;
            uploads.get(&upload_id)
                .cloned()
                .ok_or_else(|| UploadError::NotFound(upload_id.to_string()))?
        };

        // Verify all chunks received
        for (i, chunk) in upload.chunks.iter().enumerate() {
            if !chunk.received {
                return Err(UploadError::ChunkMissing(i));
            }
        }

        // Assemble file
        let mut data = Vec::with_capacity(upload.total_size as usize);
        for i in 0..upload.total_chunks {
            let chunk_path = format!("{}/chunk_{}", upload.temp_path, i);
            let chunk_data = self.storage.read(&chunk_path).await?;
            data.extend(chunk_data);
        }

        // Upload assembled file
        let options = UploadOptions {
            folder_id: upload.folder_id,
            title: None,
            description: None,
            alt_text: None,
            tags: vec![],
            optimize: self.settings.auto_optimize,
            generate_thumbnails: self.settings.auto_thumbnails,
        };

        let media = self.upload(&data, &upload.filename, options, upload.user_id).await?;

        // Cleanup temp files
        self.storage.delete_directory(&upload.temp_path).await?;

        // Remove from tracking
        let mut uploads = self.chunked_uploads.write().await;
        uploads.remove(&upload_id);

        Ok(media)
    }

    /// Cancel chunked upload
    pub async fn cancel_chunked_upload(&self, upload_id: Uuid) -> Result<(), UploadError> {
        let upload = {
            let mut uploads = self.chunked_uploads.write().await;
            uploads.remove(&upload_id)
                .ok_or_else(|| UploadError::NotFound(upload_id.to_string()))?
        };

        // Cleanup temp files
        self.storage.delete_directory(&upload.temp_path).await?;

        Ok(())
    }

    /// Get chunked upload
    pub async fn get_chunked_upload(&self, upload_id: Uuid) -> Option<ChunkedUpload> {
        let uploads = self.chunked_uploads.read().await;
        uploads.get(&upload_id).cloned()
    }

    /// Upload from URL
    pub async fn upload_from_url(
        &self,
        url: &str,
        filename: Option<&str>,
        folder_id: Option<Uuid>,
        user_id: Option<Uuid>,
    ) -> Result<MediaItem, UploadError> {
        // Download file
        let response = reqwest::get(url).await
            .map_err(|e| UploadError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(UploadError::Network(format!("HTTP {}", response.status())));
        }

        // Get filename from URL or header
        let final_filename = filename
            .map(|s| s.to_string())
            .or_else(|| {
                response.headers()
                    .get("content-disposition")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| {
                        v.split("filename=")
                            .nth(1)
                            .map(|s| s.trim_matches('"').to_string())
                    })
            })
            .or_else(|| {
                url.split('/').last()
                    .map(|s| s.split('?').next().unwrap_or(s).to_string())
            })
            .unwrap_or_else(|| format!("download_{}", Uuid::now_v7()));

        let data = response.bytes().await
            .map_err(|e| UploadError::Network(e.to_string()))?
            .to_vec();

        let options = UploadOptions {
            folder_id,
            title: None,
            description: None,
            alt_text: None,
            tags: vec![],
            optimize: self.settings.auto_optimize,
            generate_thumbnails: self.settings.auto_thumbnails,
        };

        self.upload(data, &final_filename, options, user_id).await
    }

    /// Validate file
    pub fn validate_file(&self, filename: &str, size: u64, mime_type: Option<&str>) -> Result<(), UploadError> {
        // Check size
        if size > self.settings.max_file_size {
            return Err(UploadError::FileTooLarge(size, self.settings.max_file_size));
        }

        // Check extension
        let ext = std::path::Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if !self.settings.allowed_extensions.contains(&ext) {
            return Err(UploadError::TypeNotAllowed(ext));
        }

        // Check MIME type
        if let Some(mime) = mime_type {
            if !self.settings.allowed_types.iter().any(|t| t == mime || t.starts_with(&format!("{}/*", mime.split('/').next().unwrap_or("")))) {
                return Err(UploadError::TypeNotAllowed(mime.to_string()));
            }
        }

        Ok(())
    }

    /// Get allowed file types
    pub fn get_allowed_types(&self) -> Vec<String> {
        self.settings.allowed_types.clone()
    }

    /// Get allowed extensions
    pub fn get_allowed_extensions(&self) -> Vec<String> {
        self.settings.allowed_extensions.clone()
    }

    /// Get max file size
    pub fn get_max_file_size(&self) -> u64 {
        self.settings.max_file_size
    }

    /// Detect MIME type
    fn detect_mime_type(&self, data: &[u8], filename: &str) -> String {
        // Try to detect from content
        if let Some(kind) = infer::get(data) {
            return kind.mime_type().to_string();
        }

        // Fall back to extension
        let ext = std::path::Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "mp4" => "video/mp4",
            "webm" => "video/webm",
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "pdf" => "application/pdf",
            "doc" => "application/msword",
            "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "xls" => "application/vnd.ms-excel",
            "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "zip" => "application/zip",
            "txt" => "text/plain",
            "csv" => "text/csv",
            _ => "application/octet-stream",
        }.to_string()
    }

    /// Check if MIME type is an image
    fn is_image(&self, mime_type: &str) -> bool {
        mime_type.starts_with("image/") && mime_type != "image/svg+xml"
    }

    /// Cleanup expired uploads
    pub async fn cleanup_expired(&self) -> usize {
        let mut uploads = self.chunked_uploads.write().await;
        let now = Utc::now();

        let expired: Vec<Uuid> = uploads.iter()
            .filter(|(_, u)| u.expires_at < now)
            .map(|(id, _)| *id)
            .collect();

        let count = expired.len();

        for id in expired {
            if let Some(upload) = uploads.remove(&id) {
                let _ = self.storage.delete_directory(&upload.temp_path).await;
            }
        }

        count
    }
}
