//! Folder Models
//!
//! Media folder/collection structures.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Media folder for organizing files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFolder {
    /// Unique ID
    pub id: Uuid,
    /// Folder name
    pub name: String,
    /// URL-safe slug
    pub slug: String,
    /// Description
    pub description: Option<String>,
    /// Parent folder ID (None = root)
    pub parent_id: Option<Uuid>,
    /// Full path from root
    pub path: String,
    /// Depth level (0 = root)
    pub depth: u32,
    /// Cover image ID
    pub cover_image_id: Option<Uuid>,
    /// Number of items in folder
    pub item_count: u32,
    /// Total size of items in bytes
    pub total_size: u64,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Created by user ID
    pub created_by: Option<Uuid>,
    /// Custom metadata
    pub metadata: FolderMetadata,
}

impl MediaFolder {
    /// Create a new folder
    pub fn new(name: impl Into<String>, parent_id: Option<Uuid>) -> Self {
        let name = name.into();
        let slug = slugify(&name);
        let now = Utc::now();

        Self {
            id: Uuid::now_v7(),
            name,
            slug,
            description: None,
            parent_id,
            path: String::new(),
            depth: if parent_id.is_some() { 1 } else { 0 },
            cover_image_id: None,
            item_count: 0,
            total_size: 0,
            created_at: now,
            updated_at: now,
            created_by: None,
            metadata: FolderMetadata::default(),
        }
    }

    /// Build full path from ancestors
    pub fn build_path(&mut self, ancestors: &[&MediaFolder]) {
        let mut parts: Vec<&str> = ancestors.iter().map(|f| f.slug.as_str()).collect();
        parts.push(&self.slug);
        self.path = parts.join("/");
        self.depth = ancestors.len() as u32;
    }

    /// Get formatted total size
    pub fn formatted_size(&self) -> String {
        super::media::format_bytes(self.total_size)
    }
}

/// Folder metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FolderMetadata {
    /// Color for UI
    pub color: Option<String>,
    /// Icon name
    pub icon: Option<String>,
    /// Sort order
    pub sort_order: Option<i32>,
    /// Is system folder (cannot delete)
    pub is_system: bool,
    /// Access permissions
    pub permissions: Option<FolderPermissions>,
}

/// Folder permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderPermissions {
    /// Public access
    pub is_public: bool,
    /// Allowed user IDs
    pub allowed_users: Vec<Uuid>,
    /// Allowed role names
    pub allowed_roles: Vec<String>,
}

impl Default for FolderPermissions {
    fn default() -> Self {
        Self {
            is_public: false,
            allowed_users: Vec::new(),
            allowed_roles: Vec::new(),
        }
    }
}

/// Folder tree node for hierarchical display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderTreeNode {
    pub folder: MediaFolder,
    pub children: Vec<FolderTreeNode>,
}

impl FolderTreeNode {
    /// Create a leaf node
    pub fn leaf(folder: MediaFolder) -> Self {
        Self {
            folder,
            children: Vec::new(),
        }
    }

    /// Add a child node
    pub fn add_child(&mut self, child: FolderTreeNode) {
        self.children.push(child);
    }

    /// Get total item count including children
    pub fn total_items(&self) -> u32 {
        self.folder.item_count + self.children.iter().map(|c| c.total_items()).sum::<u32>()
    }

    /// Get total size including children
    pub fn total_size(&self) -> u64 {
        self.folder.total_size + self.children.iter().map(|c| c.total_size()).sum::<u64>()
    }
}

/// Create folder request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateFolderRequest {
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub description: Option<String>,
}

/// Update folder request
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateFolderRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub cover_image_id: Option<Uuid>,
}

/// Move items request
#[derive(Debug, Clone, Deserialize)]
pub struct MoveItemsRequest {
    pub item_ids: Vec<Uuid>,
    pub target_folder_id: Option<Uuid>,
}

/// Folder breadcrumb
#[derive(Debug, Clone, Serialize)]
pub struct FolderBreadcrumb {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
}

/// Create URL-safe slug from name
pub fn slugify(name: &str) -> String {
    let re = regex::Regex::new(r"[^a-zA-Z0-9-]").unwrap();
    re.replace_all(&name.to_lowercase(), "-")
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("My Folder"), "my-folder");
        assert_eq!(slugify("Test@#$%Folder"), "test----folder");
        assert_eq!(slugify("  spaces  "), "--spaces--");
    }

    #[test]
    fn test_folder_creation() {
        let folder = MediaFolder::new("Test Folder", None);
        assert_eq!(folder.name, "Test Folder");
        assert_eq!(folder.slug, "test-folder");
        assert_eq!(folder.depth, 0);
        assert!(folder.parent_id.is_none());
    }

    #[test]
    fn test_folder_with_parent() {
        let parent_id = Uuid::now_v7();
        let folder = MediaFolder::new("Child Folder", Some(parent_id));
        assert_eq!(folder.parent_id, Some(parent_id));
        assert_eq!(folder.depth, 1);
    }
}
