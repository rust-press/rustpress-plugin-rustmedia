//! Upload Handlers

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{MediaItem, UploadOptions, ChunkedUpload, ChunkInfo};
use crate::services::{UploadService, MediaService, upload::UploadError};

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub id: String,
    pub filename: String,
    pub url: String,
    pub mime_type: String,
    pub size: u64,
    pub size_formatted: String,
    pub thumbnails: Vec<ThumbnailInfo>,
}

#[derive(Debug, Serialize)]
pub struct ThumbnailInfo {
    pub name: String,
    pub url: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
pub struct UploadRequest {
    pub folder_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub alt_text: Option<String>,
    pub tags: Option<Vec<String>>,
    pub optimize: Option<bool>,
    pub generate_thumbnails: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ChunkUploadInitRequest {
    pub filename: String,
    pub total_size: u64,
    pub chunk_size: usize,
    pub total_chunks: usize,
    pub mime_type: Option<String>,
    pub folder_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChunkUploadInitResponse {
    pub upload_id: String,
    pub chunk_size: usize,
    pub total_chunks: usize,
}

#[derive(Debug, Deserialize)]
pub struct ChunkUploadRequest {
    pub upload_id: String,
    pub chunk_index: usize,
}

#[derive(Debug, Serialize)]
pub struct ChunkUploadResponse {
    pub upload_id: String,
    pub chunk_index: usize,
    pub chunks_received: usize,
    pub total_chunks: usize,
    pub progress_percent: f64,
}

#[derive(Debug, Serialize)]
pub struct ChunkUploadCompleteResponse {
    pub id: String,
    pub filename: String,
    pub url: String,
    pub size: u64,
}

#[derive(Debug, Deserialize)]
pub struct UrlUploadRequest {
    pub url: String,
    pub folder_id: Option<String>,
    pub filename: Option<String>,
}

/// Upload handler
pub struct UploadHandler {
    upload_service: Arc<UploadService>,
    media_service: Arc<MediaService>,
}

impl UploadHandler {
    pub fn new(upload_service: Arc<UploadService>, media_service: Arc<MediaService>) -> Self {
        Self {
            upload_service,
            media_service,
        }
    }

    /// Handle single file upload
    pub async fn upload(
        &self,
        data: Vec<u8>,
        filename: &str,
        request: UploadRequest,
        user_id: Option<Uuid>,
    ) -> Result<UploadResponse, String> {
        let folder_id = request.folder_id
            .map(|f| Uuid::parse_str(&f))
            .transpose()
            .map_err(|e| e.to_string())?;

        let options = UploadOptions {
            folder_id,
            title: request.title,
            description: request.description,
            alt_text: request.alt_text,
            tags: request.tags.unwrap_or_default(),
            optimize: request.optimize.unwrap_or(true),
            generate_thumbnails: request.generate_thumbnails.unwrap_or(true),
        };

        let media = self.upload_service.upload(data, filename, options, user_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(Self::to_response(&media))
    }

    /// Handle multiple file uploads
    pub async fn upload_multiple(
        &self,
        files: Vec<(Vec<u8>, String)>,
        folder_id: Option<String>,
        user_id: Option<Uuid>,
    ) -> Vec<Result<UploadResponse, String>> {
        let folder_uuid = folder_id
            .and_then(|f| Uuid::parse_str(&f).ok());

        let mut results = Vec::new();

        for (data, filename) in files {
            let options = UploadOptions {
                folder_id: folder_uuid,
                title: None,
                description: None,
                alt_text: None,
                tags: vec![],
                optimize: true,
                generate_thumbnails: true,
            };

            let result = self.upload_service.upload(data, &filename, options, user_id)
                .await
                .map(|m| Self::to_response(&m))
                .map_err(|e| e.to_string());

            results.push(result);
        }

        results
    }

    /// Initialize chunked upload
    pub async fn init_chunked_upload(
        &self,
        request: ChunkUploadInitRequest,
        user_id: Option<Uuid>,
    ) -> Result<ChunkUploadInitResponse, String> {
        let folder_id = request.folder_id
            .map(|f| Uuid::parse_str(&f))
            .transpose()
            .map_err(|e| e.to_string())?;

        let upload = self.upload_service.init_chunked_upload(
            &request.filename,
            request.total_size,
            request.chunk_size,
            request.total_chunks,
            request.mime_type,
            folder_id,
            user_id,
        ).await.map_err(|e| e.to_string())?;

        Ok(ChunkUploadInitResponse {
            upload_id: upload.id.to_string(),
            chunk_size: upload.chunk_size,
            total_chunks: upload.total_chunks,
        })
    }

    /// Upload a chunk
    pub async fn upload_chunk(
        &self,
        upload_id: &str,
        chunk_index: usize,
        data: Vec<u8>,
    ) -> Result<ChunkUploadResponse, String> {
        let uuid = Uuid::parse_str(upload_id).map_err(|e| e.to_string())?;

        let upload = self.upload_service.upload_chunk(uuid, chunk_index, data)
            .await
            .map_err(|e| e.to_string())?;

        let chunks_received = upload.chunks.iter().filter(|c| c.received).count();
        let progress = (chunks_received as f64 / upload.total_chunks as f64) * 100.0;

        Ok(ChunkUploadResponse {
            upload_id: upload.id.to_string(),
            chunk_index,
            chunks_received,
            total_chunks: upload.total_chunks,
            progress_percent: progress,
        })
    }

    /// Complete chunked upload
    pub async fn complete_chunked_upload(
        &self,
        upload_id: &str,
    ) -> Result<ChunkUploadCompleteResponse, String> {
        let uuid = Uuid::parse_str(upload_id).map_err(|e| e.to_string())?;

        let media = self.upload_service.complete_chunked_upload(uuid)
            .await
            .map_err(|e| e.to_string())?;

        Ok(ChunkUploadCompleteResponse {
            id: media.id.to_string(),
            filename: media.filename,
            url: media.url,
            size: media.size,
        })
    }

    /// Cancel chunked upload
    pub async fn cancel_chunked_upload(&self, upload_id: &str) -> Result<(), String> {
        let uuid = Uuid::parse_str(upload_id).map_err(|e| e.to_string())?;
        self.upload_service.cancel_chunked_upload(uuid).await.map_err(|e| e.to_string())
    }

    /// Get chunked upload status
    pub async fn get_chunked_upload_status(&self, upload_id: &str) -> Result<ChunkedUploadStatus, String> {
        let uuid = Uuid::parse_str(upload_id).map_err(|e| e.to_string())?;

        let upload = self.upload_service.get_chunked_upload(uuid)
            .await
            .ok_or_else(|| "Upload not found".to_string())?;

        let chunks_received = upload.chunks.iter().filter(|c| c.received).count();
        let bytes_received: u64 = upload.chunks.iter()
            .filter(|c| c.received)
            .map(|c| c.size as u64)
            .sum();

        Ok(ChunkedUploadStatus {
            upload_id: upload.id.to_string(),
            filename: upload.filename,
            total_size: upload.total_size,
            bytes_received,
            chunks_received,
            total_chunks: upload.total_chunks,
            progress_percent: (chunks_received as f64 / upload.total_chunks as f64) * 100.0,
            started_at: upload.started_at.to_rfc3339(),
            expires_at: upload.expires_at.to_rfc3339(),
        })
    }

    /// Upload from URL
    pub async fn upload_from_url(
        &self,
        request: UrlUploadRequest,
        user_id: Option<Uuid>,
    ) -> Result<UploadResponse, String> {
        let folder_id = request.folder_id
            .map(|f| Uuid::parse_str(&f))
            .transpose()
            .map_err(|e| e.to_string())?;

        let media = self.upload_service.upload_from_url(
            &request.url,
            request.filename.as_deref(),
            folder_id,
            user_id,
        ).await.map_err(|e| e.to_string())?;

        Ok(Self::to_response(&media))
    }

    /// Validate file before upload
    pub fn validate_file(&self, filename: &str, size: u64, mime_type: Option<&str>) -> Result<(), String> {
        self.upload_service.validate_file(filename, size, mime_type)
            .map_err(|e| e.to_string())
    }

    /// Get allowed file types
    pub fn get_allowed_types(&self) -> Vec<String> {
        self.upload_service.get_allowed_types()
    }

    /// Get max file size
    pub fn get_max_file_size(&self) -> u64 {
        self.upload_service.get_max_file_size()
    }

    fn to_response(media: &MediaItem) -> UploadResponse {
        UploadResponse {
            id: media.id.to_string(),
            filename: media.filename.clone(),
            url: media.url.clone(),
            mime_type: media.mime_type.clone(),
            size: media.size,
            size_formatted: media.formatted_size(),
            thumbnails: media.thumbnails.iter().map(|t| ThumbnailInfo {
                name: t.size_name.clone(),
                url: t.url.clone(),
                width: t.width,
                height: t.height,
            }).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ChunkedUploadStatus {
    pub upload_id: String,
    pub filename: String,
    pub total_size: u64,
    pub bytes_received: u64,
    pub chunks_received: usize,
    pub total_chunks: usize,
    pub progress_percent: f64,
    pub started_at: String,
    pub expires_at: String,
}
