//! Folder Handlers

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{MediaFolder, FolderTreeNode, FolderBreadcrumb};
use crate::services::FolderService;

#[derive(Debug, Serialize)]
pub struct FolderResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub parent_id: Option<String>,
    pub path: String,
    pub item_count: u32,
    pub total_size: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateFolderRequest {
    pub name: String,
    pub parent_id: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFolderRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Folder handler
pub struct FolderHandler {
    folder_service: Arc<FolderService>,
}

impl FolderHandler {
    pub fn new(folder_service: Arc<FolderService>) -> Self {
        Self { folder_service }
    }

    /// Create folder
    pub async fn create(&self, request: CreateFolderRequest, user_id: Option<Uuid>) -> Result<FolderResponse, String> {
        let parent_id = request.parent_id
            .map(|p| Uuid::parse_str(&p))
            .transpose()
            .map_err(|e| e.to_string())?;

        let folder = self.folder_service.create(&request.name, parent_id, user_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(Self::to_response(&folder))
    }

    /// Get folder
    pub async fn get(&self, id: &str) -> Result<FolderResponse, String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;

        let folder = self.folder_service.get(uuid).await
            .ok_or_else(|| "Folder not found".to_string())?;

        Ok(Self::to_response(&folder))
    }

    /// Update folder
    pub async fn update(&self, id: &str, request: UpdateFolderRequest) -> Result<FolderResponse, String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;

        let folder = self.folder_service.update(uuid, request.name, request.description)
            .await
            .map_err(|e| e.to_string())?;

        Ok(Self::to_response(&folder))
    }

    /// Delete folder
    pub async fn delete(&self, id: &str, force: bool) -> Result<(), String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;
        self.folder_service.delete(uuid, force).await.map_err(|e| e.to_string())
    }

    /// List root folders
    pub async fn list_roots(&self) -> Vec<FolderResponse> {
        self.folder_service.get_roots().await
            .into_iter()
            .map(|f| Self::to_response(&f))
            .collect()
    }

    /// Get children
    pub async fn get_children(&self, id: &str) -> Result<Vec<FolderResponse>, String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;

        Ok(self.folder_service.get_children(uuid).await
            .into_iter()
            .map(|f| Self::to_response(&f))
            .collect())
    }

    /// Get folder tree
    pub async fn get_tree(&self) -> Vec<FolderTreeNode> {
        self.folder_service.get_tree().await
    }

    /// Get breadcrumbs
    pub async fn get_breadcrumbs(&self, id: &str) -> Result<Vec<FolderBreadcrumb>, String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;
        Ok(self.folder_service.get_breadcrumbs(uuid).await)
    }

    /// Move folder
    pub async fn move_folder(&self, id: &str, new_parent_id: Option<String>) -> Result<FolderResponse, String> {
        let uuid = Uuid::parse_str(id).map_err(|e| e.to_string())?;
        let parent_uuid = new_parent_id
            .map(|p| Uuid::parse_str(&p))
            .transpose()
            .map_err(|e| e.to_string())?;

        let folder = self.folder_service.move_folder(uuid, parent_uuid)
            .await
            .map_err(|e| e.to_string())?;

        Ok(Self::to_response(&folder))
    }

    fn to_response(folder: &MediaFolder) -> FolderResponse {
        FolderResponse {
            id: folder.id.to_string(),
            name: folder.name.clone(),
            slug: folder.slug.clone(),
            description: folder.description.clone(),
            parent_id: folder.parent_id.map(|p| p.to_string()),
            path: folder.path.clone(),
            item_count: folder.item_count,
            total_size: folder.formatted_size(),
            created_at: folder.created_at.to_rfc3339(),
        }
    }
}
