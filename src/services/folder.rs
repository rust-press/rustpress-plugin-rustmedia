//! Folder Service
//!
//! Media folder management.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;
use uuid::Uuid;

use crate::models::{MediaFolder, FolderTreeNode, FolderBreadcrumb, slugify};

/// Folder service error
#[derive(Debug, thiserror::Error)]
pub enum FolderError {
    #[error("Folder not found: {0}")]
    NotFound(String),
    #[error("Folder already exists: {0}")]
    AlreadyExists(String),
    #[error("Invalid operation: {0}")]
    Invalid(String),
    #[error("Cannot delete non-empty folder")]
    NotEmpty,
}

/// Folder service
pub struct FolderService {
    /// Folders (in-memory, would be database in production)
    folders: Arc<RwLock<HashMap<Uuid, MediaFolder>>>,
}

impl FolderService {
    /// Create a new folder service
    pub fn new() -> Self {
        Self {
            folders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new folder
    pub async fn create(
        &self,
        name: &str,
        parent_id: Option<Uuid>,
        user_id: Option<Uuid>,
    ) -> Result<MediaFolder, FolderError> {
        let folders = self.folders.read().await;

        // Check parent exists
        if let Some(pid) = parent_id {
            if !folders.contains_key(&pid) {
                return Err(FolderError::NotFound(pid.to_string()));
            }
        }

        // Check for duplicate name in same parent
        let slug = slugify(name);
        let exists = folders.values().any(|f| {
            f.parent_id == parent_id && f.slug == slug
        });

        if exists {
            return Err(FolderError::AlreadyExists(name.to_string()));
        }

        drop(folders);

        // Create folder
        let mut folder = MediaFolder::new(name, parent_id);
        folder.created_by = user_id;

        // Build path
        if let Some(pid) = parent_id {
            let ancestors = self.get_ancestors(pid).await;
            let ancestor_refs: Vec<&MediaFolder> = ancestors.iter().collect();
            folder.build_path(&ancestor_refs);
        } else {
            folder.path = folder.slug.clone();
        }

        // Store
        let id = folder.id;
        let mut folders = self.folders.write().await;
        folders.insert(id, folder.clone());

        Ok(folder)
    }

    /// Get folder by ID
    pub async fn get(&self, id: Uuid) -> Option<MediaFolder> {
        let folders = self.folders.read().await;
        folders.get(&id).cloned()
    }

    /// Get folder by path
    pub async fn get_by_path(&self, path: &str) -> Option<MediaFolder> {
        let folders = self.folders.read().await;
        folders.values().find(|f| f.path == path).cloned()
    }

    /// Update folder
    pub async fn update(
        &self,
        id: Uuid,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<MediaFolder, FolderError> {
        let mut folders = self.folders.write().await;

        let folder = folders.get_mut(&id)
            .ok_or_else(|| FolderError::NotFound(id.to_string()))?;

        if let Some(new_name) = name {
            folder.name = new_name.clone();
            folder.slug = slugify(&new_name);
            // Note: Would need to update path for this folder and children
        }

        if let Some(desc) = description {
            folder.description = Some(desc);
        }

        folder.updated_at = Utc::now();

        Ok(folder.clone())
    }

    /// Delete folder
    pub async fn delete(&self, id: Uuid, force: bool) -> Result<(), FolderError> {
        let folders = self.folders.read().await;

        let folder = folders.get(&id)
            .ok_or_else(|| FolderError::NotFound(id.to_string()))?;

        // Check if folder has items
        if !force && folder.item_count > 0 {
            return Err(FolderError::NotEmpty);
        }

        // Check for system folder
        if folder.metadata.is_system {
            return Err(FolderError::Invalid("Cannot delete system folder".to_string()));
        }

        // Check for children
        let has_children = folders.values().any(|f| f.parent_id == Some(id));
        if !force && has_children {
            return Err(FolderError::NotEmpty);
        }

        drop(folders);

        // Delete children recursively if force
        if force {
            let children = self.get_children(id).await;
            for child in children {
                let _ = Box::pin(self.delete(child.id, true)).await;
            }
        }

        // Delete folder
        let mut folders = self.folders.write().await;
        folders.remove(&id);

        Ok(())
    }

    /// Move folder to new parent
    pub async fn move_folder(
        &self,
        id: Uuid,
        new_parent_id: Option<Uuid>,
    ) -> Result<MediaFolder, FolderError> {
        // Prevent moving to self or descendant
        if let Some(pid) = new_parent_id {
            if pid == id {
                return Err(FolderError::Invalid("Cannot move folder to itself".to_string()));
            }

            let descendants = self.get_descendants(id).await;
            if descendants.iter().any(|f| f.id == pid) {
                return Err(FolderError::Invalid("Cannot move folder to its descendant".to_string()));
            }
        }

        let mut folders = self.folders.write().await;

        let folder = folders.get_mut(&id)
            .ok_or_else(|| FolderError::NotFound(id.to_string()))?;

        folder.parent_id = new_parent_id;
        folder.updated_at = Utc::now();

        // Would need to rebuild paths for this folder and descendants

        Ok(folder.clone())
    }

    /// Get children of a folder
    pub async fn get_children(&self, parent_id: Uuid) -> Vec<MediaFolder> {
        let folders = self.folders.read().await;

        folders.values()
            .filter(|f| f.parent_id == Some(parent_id))
            .cloned()
            .collect()
    }

    /// Get root folders
    pub async fn get_roots(&self) -> Vec<MediaFolder> {
        let folders = self.folders.read().await;

        folders.values()
            .filter(|f| f.parent_id.is_none())
            .cloned()
            .collect()
    }

    /// Get all folders
    pub async fn get_all(&self) -> Vec<MediaFolder> {
        let folders = self.folders.read().await;
        folders.values().cloned().collect()
    }

    /// Get ancestors (parent, grandparent, etc.)
    pub async fn get_ancestors(&self, id: Uuid) -> Vec<MediaFolder> {
        let folders = self.folders.read().await;
        let mut ancestors = Vec::new();
        let mut current_id = Some(id);

        while let Some(cid) = current_id {
            if let Some(folder) = folders.get(&cid) {
                if folder.id != id {
                    ancestors.push(folder.clone());
                }
                current_id = folder.parent_id;
            } else {
                break;
            }
        }

        ancestors.reverse();
        ancestors
    }

    /// Get descendants (children, grandchildren, etc.)
    pub async fn get_descendants(&self, id: Uuid) -> Vec<MediaFolder> {
        let folders = self.folders.read().await;
        let mut descendants = Vec::new();
        let mut to_process = vec![id];

        while let Some(current_id) = to_process.pop() {
            for folder in folders.values() {
                if folder.parent_id == Some(current_id) {
                    descendants.push(folder.clone());
                    to_process.push(folder.id);
                }
            }
        }

        descendants
    }

    /// Build folder tree
    pub async fn get_tree(&self) -> Vec<FolderTreeNode> {
        let folders = self.folders.read().await;
        let roots: Vec<&MediaFolder> = folders.values()
            .filter(|f| f.parent_id.is_none())
            .collect();

        let mut tree = Vec::new();
        for root in roots {
            let node = self.build_tree_node(root, &folders);
            tree.push(node);
        }

        tree
    }

    /// Build tree node recursively
    fn build_tree_node(
        &self,
        folder: &MediaFolder,
        all_folders: &HashMap<Uuid, MediaFolder>,
    ) -> FolderTreeNode {
        let children: Vec<FolderTreeNode> = all_folders.values()
            .filter(|f| f.parent_id == Some(folder.id))
            .map(|f| self.build_tree_node(f, all_folders))
            .collect();

        FolderTreeNode {
            folder: folder.clone(),
            children,
        }
    }

    /// Get breadcrumbs for folder
    pub async fn get_breadcrumbs(&self, id: Uuid) -> Vec<FolderBreadcrumb> {
        let ancestors = self.get_ancestors(id).await;

        let mut breadcrumbs: Vec<FolderBreadcrumb> = ancestors
            .into_iter()
            .map(|f| FolderBreadcrumb {
                id: f.id,
                name: f.name,
                slug: f.slug,
            })
            .collect();

        // Add current folder
        if let Some(folder) = self.get(id).await {
            breadcrumbs.push(FolderBreadcrumb {
                id: folder.id,
                name: folder.name,
                slug: folder.slug,
            });
        }

        breadcrumbs
    }

    /// Update folder item count
    pub async fn update_item_count(&self, id: Uuid, delta: i32) {
        let mut folders = self.folders.write().await;

        if let Some(folder) = folders.get_mut(&id) {
            if delta > 0 {
                folder.item_count += delta as u32;
            } else {
                folder.item_count = folder.item_count.saturating_sub((-delta) as u32);
            }
            folder.updated_at = Utc::now();
        }
    }

    /// Update folder total size
    pub async fn update_total_size(&self, id: Uuid, delta: i64) {
        let mut folders = self.folders.write().await;

        if let Some(folder) = folders.get_mut(&id) {
            if delta > 0 {
                folder.total_size += delta as u64;
            } else {
                folder.total_size = folder.total_size.saturating_sub((-delta) as u64);
            }
            folder.updated_at = Utc::now();
        }
    }

    /// Search folders by name
    pub async fn search(&self, query: &str) -> Vec<MediaFolder> {
        let folders = self.folders.read().await;
        let query_lower = query.to_lowercase();

        folders.values()
            .filter(|f| f.name.to_lowercase().contains(&query_lower))
            .cloned()
            .collect()
    }
}

impl Default for FolderService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_folder() {
        let service = FolderService::new();

        let folder = service.create("Test Folder", None, None).await.unwrap();
        assert_eq!(folder.name, "Test Folder");
        assert_eq!(folder.slug, "test-folder");
        assert!(folder.parent_id.is_none());
    }

    #[tokio::test]
    async fn test_create_nested_folder() {
        let service = FolderService::new();

        let parent = service.create("Parent", None, None).await.unwrap();
        let child = service.create("Child", Some(parent.id), None).await.unwrap();

        assert_eq!(child.parent_id, Some(parent.id));
        assert!(child.path.contains("parent/child") || child.depth == 1);
    }

    #[tokio::test]
    async fn test_get_children() {
        let service = FolderService::new();

        let parent = service.create("Parent", None, None).await.unwrap();
        service.create("Child 1", Some(parent.id), None).await.unwrap();
        service.create("Child 2", Some(parent.id), None).await.unwrap();

        let children = service.get_children(parent.id).await;
        assert_eq!(children.len(), 2);
    }
}
