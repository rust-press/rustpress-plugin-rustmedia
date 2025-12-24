//! Media Handlers

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{MediaItem, MediaFilter, MediaListResponse, MediaType};
use crate::services::{MediaService, media::MediaStats};

#[derive(Debug, Serialize)]
pub struct MediaItemResponse {
    pub id: String,
    pub filename: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub alt_text: Option<String>,
    pub mime_type: String,
    pub media_type: String,
    pub size: u64,
    pub size_formatted: String,
    pub url: String,
    pub dimensions: Option<DimensionsResponse>,
    pub thumbnails: Vec<ThumbnailResponse>,
    pub uploaded_at: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DimensionsResponse {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize)]
pub struct ThumbnailResponse {
    pub name: String,
    pub url: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMediaRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub alt_text: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Media handler
pub struct MediaHandler {
    media_service: Arc<MediaService>,
}

impl MediaHandler {
    pub fn new(media_service: Arc<MediaService>) -> Self {
        Self { media_service }
    }

    /// Get media item
    pub async fn get(&self, id: &str) -> Result<MediaItemResponse, String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;

        let media = self.media_service.get(uuid).await
            .ok_or_else(|| "Media not found".to_string())?;

        Ok(Self::to_response(&media))
    }

    /// List media items
    pub async fn list(&self, filter: MediaFilter) -> MediaListResponse {
        self.media_service.list(filter).await
    }

    /// Update media item
    pub async fn update(&self, id: &str, request: UpdateMediaRequest) -> Result<MediaItemResponse, String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;

        let media = self.media_service.update(
            uuid,
            request.title,
            request.description,
            request.alt_text,
            request.tags,
        ).await.map_err(|e| e.to_string())?;

        Ok(Self::to_response(&media))
    }

    /// Delete media item
    pub async fn delete(&self, id: &str, permanent: bool) -> Result<(), String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;
        self.media_service.delete(uuid, permanent).await.map_err(|e| e.to_string())
    }

    /// Move to folder
    pub async fn move_to_folder(&self, id: &str, folder_id: Option<String>) -> Result<MediaItemResponse, String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;
        let folder_uuid = folder_id.map(|f| Uuid::parse_str(&f)).transpose().map_err(|e| e.to_string())?;

        let media = self.media_service.move_to_folder(uuid, folder_uuid).await.map_err(|e| e.to_string())?;
        Ok(Self::to_response(&media))
    }

    /// Search media
    pub async fn search(&self, query: &str, limit: usize) -> Vec<MediaItemResponse> {
        self.media_service.search(query, limit).await
            .into_iter()
            .map(|m| Self::to_response(&m))
            .collect()
    }

    /// Get recent uploads
    pub async fn recent(&self, limit: usize) -> Vec<MediaItemResponse> {
        self.media_service.get_recent(limit).await
            .into_iter()
            .map(|m| Self::to_response(&m))
            .collect()
    }

    /// Get statistics
    pub async fn stats(&self) -> MediaStats {
        self.media_service.get_stats().await
    }

    fn to_response(media: &MediaItem) -> MediaItemResponse {
        MediaItemResponse {
            id: media.id.to_string(),
            filename: media.filename.clone(),
            title: media.title.clone(),
            description: media.description.clone(),
            alt_text: media.alt_text.clone(),
            mime_type: media.mime_type.clone(),
            media_type: format!("{}", media.media_type),
            size: media.size,
            size_formatted: media.formatted_size(),
            url: media.url.clone(),
            dimensions: media.dimensions.map(|d| DimensionsResponse {
                width: d.width,
                height: d.height,
            }),
            thumbnails: media.thumbnails.iter().map(|t| ThumbnailResponse {
                name: t.size_name.clone(),
                url: t.url.clone(),
                width: t.width,
                height: t.height,
            }).collect(),
            uploaded_at: media.uploaded_at.to_rfc3339(),
            tags: media.tags.clone(),
        }
    }
}
