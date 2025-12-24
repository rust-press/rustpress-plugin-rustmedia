//! Media Service
//!
//! Core media management operations.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;
use uuid::Uuid;
use sha2::{Sha256, Digest};

use crate::models::{
    MediaItem, MediaType, MediaFilter, MediaListResponse,
    ImageDimensions, MediaMetadata, Thumbnail,
};
use super::storage::{StorageService, StorageError};
use super::image::{ImageService, ImageError};

/// Media service error
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("Image error: {0}")]
    Image(#[from] ImageError),
    #[error("Media not found: {0}")]
    NotFound(String),
    #[error("Invalid media: {0}")]
    Invalid(String),
    #[error("Duplicate file: {0}")]
    Duplicate(String),
}

/// Media service
pub struct MediaService {
    /// Storage service
    storage: Arc<StorageService>,
    /// Image service
    image_service: Arc<ImageService>,
    /// Media items (in-memory, would be database in production)
    items: Arc<RwLock<HashMap<Uuid, MediaItem>>>,
    /// Content hash index for deduplication
    hash_index: Arc<RwLock<HashMap<String, Uuid>>>,
    /// Enable deduplication
    deduplicate: bool,
    /// Auto-generate thumbnails
    auto_thumbnails: bool,
}

impl MediaService {
    /// Create a new media service
    pub fn new(storage: Arc<StorageService>, image_service: Arc<ImageService>) -> Self {
        Self {
            storage,
            image_service,
            items: Arc::new(RwLock::new(HashMap::new())),
            hash_index: Arc::new(RwLock::new(HashMap::new())),
            deduplicate: true,
            auto_thumbnails: true,
        }
    }

    /// Upload a new media item
    pub async fn upload(
        &self,
        data: &[u8],
        filename: &str,
        mime_type: &str,
        folder_id: Option<Uuid>,
        user_id: Option<Uuid>,
    ) -> Result<MediaItem, MediaError> {
        // Calculate content hash
        let mut hasher = Sha256::new();
        hasher.update(data);
        let content_hash = hex::encode(hasher.finalize());

        // Check for duplicates
        if self.deduplicate {
            let hash_index = self.hash_index.read().await;
            if let Some(&existing_id) = hash_index.get(&content_hash) {
                let items = self.items.read().await;
                if let Some(existing) = items.get(&existing_id) {
                    return Err(MediaError::Duplicate(existing.filename.clone()));
                }
            }
        }

        // Store the file
        let stored = self.storage.store(data, filename, mime_type).await?;

        // Create media item
        let mut media = MediaItem::new(filename, mime_type, stored.size, &stored.path);
        media.url = stored.url;
        media.folder_id = folder_id;
        media.uploaded_by = user_id;
        media.content_hash = content_hash.clone();

        // Process based on type
        if media.is_image() {
            // Get dimensions
            if let Ok(dims) = self.image_service.get_dimensions(data) {
                media.dimensions = Some(dims);
            }

            // Generate thumbnails
            if self.auto_thumbnails {
                match self.image_service.generate_thumbnails(data, &stored.path).await {
                    Ok(thumbnails) => media.thumbnails = thumbnails,
                    Err(e) => tracing::warn!("Failed to generate thumbnails: {}", e),
                }
            }

            // Extract EXIF
            if let Ok(exif) = self.extract_exif(data) {
                media.metadata.exif = Some(exif);
            }
        }

        // Store in index
        let id = media.id;
        {
            let mut items = self.items.write().await;
            items.insert(id, media.clone());
        }

        // Update hash index
        if self.deduplicate {
            let mut hash_index = self.hash_index.write().await;
            hash_index.insert(content_hash, id);
        }

        Ok(media)
    }

    /// Get media item by ID
    pub async fn get(&self, id: Uuid) -> Option<MediaItem> {
        let items = self.items.read().await;
        items.get(&id).cloned()
    }

    /// Get media item by path
    pub async fn get_by_path(&self, path: &str) -> Option<MediaItem> {
        let items = self.items.read().await;
        items.values().find(|m| m.path == path).cloned()
    }

    /// Update media item metadata
    pub async fn update(
        &self,
        id: Uuid,
        title: Option<String>,
        description: Option<String>,
        alt_text: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<MediaItem, MediaError> {
        let mut items = self.items.write().await;

        let media = items.get_mut(&id)
            .ok_or_else(|| MediaError::NotFound(id.to_string()))?;

        if let Some(t) = title {
            media.title = Some(t);
        }
        if let Some(d) = description {
            media.description = Some(d);
        }
        if let Some(a) = alt_text {
            media.alt_text = Some(a);
        }
        if let Some(t) = tags {
            media.tags = t;
        }

        media.updated_at = Utc::now();

        Ok(media.clone())
    }

    /// Delete media item
    pub async fn delete(&self, id: Uuid, permanent: bool) -> Result<(), MediaError> {
        let mut items = self.items.write().await;

        let media = items.get_mut(&id)
            .ok_or_else(|| MediaError::NotFound(id.to_string()))?;

        if permanent {
            // Delete file from storage
            self.storage.delete(&media.path).await?;

            // Delete thumbnails
            for thumb in &media.thumbnails {
                let _ = self.storage.delete(&thumb.path).await;
            }

            // Remove from hash index
            let mut hash_index = self.hash_index.write().await;
            hash_index.remove(&media.content_hash);

            // Remove from items
            items.remove(&id);
        } else {
            // Soft delete
            media.deleted = true;
            media.updated_at = Utc::now();
        }

        Ok(())
    }

    /// Restore soft-deleted item
    pub async fn restore(&self, id: Uuid) -> Result<MediaItem, MediaError> {
        let mut items = self.items.write().await;

        let media = items.get_mut(&id)
            .ok_or_else(|| MediaError::NotFound(id.to_string()))?;

        media.deleted = false;
        media.updated_at = Utc::now();

        Ok(media.clone())
    }

    /// Move item to folder
    pub async fn move_to_folder(&self, id: Uuid, folder_id: Option<Uuid>) -> Result<MediaItem, MediaError> {
        let mut items = self.items.write().await;

        let media = items.get_mut(&id)
            .ok_or_else(|| MediaError::NotFound(id.to_string()))?;

        media.folder_id = folder_id;
        media.updated_at = Utc::now();

        Ok(media.clone())
    }

    /// List media items with filtering
    pub async fn list(&self, filter: MediaFilter) -> MediaListResponse {
        let items = self.items.read().await;

        let mut filtered: Vec<&MediaItem> = items.values()
            .filter(|m| {
                // Exclude deleted unless requested
                if !filter.include_deleted.unwrap_or(false) && m.deleted {
                    return false;
                }

                // Filter by type
                if let Some(ref media_type) = filter.media_type {
                    if &m.media_type != media_type {
                        return false;
                    }
                }

                // Filter by folder
                if let Some(folder_id) = filter.folder_id {
                    if m.folder_id != Some(folder_id) {
                        return false;
                    }
                }

                // Filter by search
                if let Some(ref search) = filter.search {
                    let search_lower = search.to_lowercase();
                    let matches = m.filename.to_lowercase().contains(&search_lower)
                        || m.title.as_ref().map(|t| t.to_lowercase().contains(&search_lower)).unwrap_or(false)
                        || m.description.as_ref().map(|d| d.to_lowercase().contains(&search_lower)).unwrap_or(false);
                    if !matches {
                        return false;
                    }
                }

                // Filter by tags
                if let Some(ref tags) = filter.tags {
                    if !tags.iter().any(|t| m.tags.contains(t)) {
                        return false;
                    }
                }

                // Filter by date range
                if let Some(date_from) = filter.date_from {
                    if m.uploaded_at < date_from {
                        return false;
                    }
                }
                if let Some(date_to) = filter.date_to {
                    if m.uploaded_at > date_to {
                        return false;
                    }
                }

                // Filter by size
                if let Some(min_size) = filter.min_size {
                    if m.size < min_size {
                        return false;
                    }
                }
                if let Some(max_size) = filter.max_size {
                    if m.size > max_size {
                        return false;
                    }
                }

                true
            })
            .collect();

        let total = filtered.len() as u64;

        // Sort
        let sort_by = filter.sort_by.as_deref().unwrap_or("uploaded_at");
        let sort_order = filter.sort_order.as_deref().unwrap_or("desc");

        filtered.sort_by(|a, b| {
            let cmp = match sort_by {
                "filename" => a.filename.cmp(&b.filename),
                "size" => a.size.cmp(&b.size),
                "type" => format!("{:?}", a.media_type).cmp(&format!("{:?}", b.media_type)),
                _ => a.uploaded_at.cmp(&b.uploaded_at),
            };

            if sort_order == "asc" { cmp } else { cmp.reverse() }
        });

        // Paginate
        let page = filter.page.unwrap_or(1).max(1);
        let per_page = filter.per_page.unwrap_or(20).clamp(1, 100);
        let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;

        let start = ((page - 1) * per_page) as usize;
        let items: Vec<MediaItem> = filtered
            .into_iter()
            .skip(start)
            .take(per_page as usize)
            .cloned()
            .collect();

        MediaListResponse {
            items,
            total,
            page,
            per_page,
            total_pages,
        }
    }

    /// Get usage statistics
    pub async fn get_stats(&self) -> MediaStats {
        let items = self.items.read().await;

        let mut stats = MediaStats::default();

        for item in items.values() {
            if item.deleted {
                continue;
            }

            stats.total_items += 1;
            stats.total_size += item.size;

            match item.media_type {
                MediaType::Image => stats.image_count += 1,
                MediaType::Video => stats.video_count += 1,
                MediaType::Audio => stats.audio_count += 1,
                MediaType::Document => stats.document_count += 1,
                _ => stats.other_count += 1,
            }
        }

        stats
    }

    /// Search media items
    pub async fn search(&self, query: &str, limit: usize) -> Vec<MediaItem> {
        let items = self.items.read().await;
        let query_lower = query.to_lowercase();

        items.values()
            .filter(|m| {
                !m.deleted && (
                    m.filename.to_lowercase().contains(&query_lower)
                    || m.title.as_ref().map(|t| t.to_lowercase().contains(&query_lower)).unwrap_or(false)
                    || m.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
                )
            })
            .take(limit)
            .cloned()
            .collect()
    }

    /// Increment usage count
    pub async fn increment_usage(&self, id: Uuid) -> Result<(), MediaError> {
        let mut items = self.items.write().await;

        if let Some(media) = items.get_mut(&id) {
            media.usage_count += 1;
        }

        Ok(())
    }

    /// Extract EXIF data from image
    fn extract_exif(&self, _data: &[u8]) -> Result<crate::models::ExifData, ImageError> {
        // Simplified - would use exif crate for real implementation
        Ok(crate::models::ExifData::default())
    }

    /// Get recent uploads
    pub async fn get_recent(&self, limit: usize) -> Vec<MediaItem> {
        let items = self.items.read().await;

        let mut recent: Vec<&MediaItem> = items.values()
            .filter(|m| !m.deleted)
            .collect();

        recent.sort_by(|a, b| b.uploaded_at.cmp(&a.uploaded_at));

        recent.into_iter().take(limit).cloned().collect()
    }
}

/// Media statistics
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct MediaStats {
    pub total_items: u64,
    pub total_size: u64,
    pub image_count: u64,
    pub video_count: u64,
    pub audio_count: u64,
    pub document_count: u64,
    pub other_count: u64,
}

impl MediaStats {
    pub fn formatted_size(&self) -> String {
        crate::models::format_bytes(self.total_size)
    }
}
