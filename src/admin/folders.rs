//! Folders Admin View

use std::sync::Arc;
use serde::Serialize;

use crate::models::{FolderTreeNode, FolderBreadcrumb};
use crate::services::FolderService;

/// Folders page data
#[derive(Debug, Serialize)]
pub struct FoldersPageData {
    pub tree: Vec<FolderTreeNode>,
    pub total_folders: usize,
}

/// Single folder view data
#[derive(Debug, Serialize)]
pub struct FolderViewData {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub path: String,
    pub item_count: u32,
    pub total_size: String,
    pub created_at: String,
    pub children: Vec<ChildFolder>,
    pub breadcrumbs: Vec<FolderBreadcrumb>,
}

#[derive(Debug, Serialize)]
pub struct ChildFolder {
    pub id: String,
    pub name: String,
    pub item_count: u32,
}

/// Folders view
pub struct FoldersView {
    folder_service: Arc<FolderService>,
}

impl FoldersView {
    pub fn new(folder_service: Arc<FolderService>) -> Self {
        Self { folder_service }
    }

    /// Get folders page data
    pub async fn get_data(&self) -> FoldersPageData {
        let tree = self.folder_service.get_tree().await;
        let all = self.folder_service.get_all().await;

        FoldersPageData {
            tree,
            total_folders: all.len(),
        }
    }

    /// Get single folder data
    pub async fn get_folder_data(&self, folder_id: &str) -> Option<FolderViewData> {
        let uuid = uuid::Uuid::parse_str(folder_id).ok()?;
        let folder = self.folder_service.get(uuid).await?;
        let children = self.folder_service.get_children(uuid).await;
        let breadcrumbs = self.folder_service.get_breadcrumbs(uuid).await;

        Some(FolderViewData {
            id: folder.id.to_string(),
            name: folder.name,
            description: folder.description,
            path: folder.path,
            item_count: folder.item_count,
            total_size: folder.formatted_size(),
            created_at: folder.created_at.to_rfc3339(),
            children: children.into_iter().map(|c| ChildFolder {
                id: c.id.to_string(),
                name: c.name,
                item_count: c.item_count,
            }).collect(),
            breadcrumbs,
        })
    }

    /// Render folders page HTML
    pub async fn render(&self) -> String {
        let data = self.get_data().await;

        format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Folders - RustMedia</title>
    <link rel="stylesheet" href="/plugins/rustmedia/assets/css/admin.css">
</head>
<body>
    <div class="rustmedia-admin">
        <header class="admin-header">
            <h1>Media Folders</h1>
            <nav class="admin-nav">
                <a href="/admin/media">Dashboard</a>
                <a href="/admin/media/library">Library</a>
                <a href="/admin/media/upload">Upload</a>
                <a href="/admin/media/folders" class="active">Folders</a>
                <a href="/admin/media/settings">Settings</a>
            </nav>
        </header>

        <main class="admin-content">
            <div class="folders-toolbar">
                <button class="btn btn-primary" id="create-folder-btn">
                    + Create Folder
                </button>
                <span class="folder-count">{} folders</span>
            </div>

            <div class="folders-container">
                <div class="folder-tree">
                    <h3>Folder Structure</h3>
                    <ul class="tree-root">
                        {}
                    </ul>
                </div>

                <div class="folder-details" id="folder-details">
                    <p class="placeholder">Select a folder to view details</p>
                </div>
            </div>
        </main>
    </div>

    <div id="create-folder-modal" class="modal hidden">
        <div class="modal-content">
            <div class="modal-header">
                <h2>Create Folder</h2>
                <button class="modal-close">&times;</button>
            </div>
            <form id="create-folder-form">
                <div class="form-group">
                    <label for="folder-name">Folder Name</label>
                    <input type="text" id="folder-name" name="name" required>
                </div>
                <div class="form-group">
                    <label for="parent-folder">Parent Folder</label>
                    <select id="parent-folder" name="parent_id">
                        <option value="">Root (No parent)</option>
                        {}
                    </select>
                </div>
                <div class="form-group">
                    <label for="folder-description">Description</label>
                    <textarea id="folder-description" name="description"></textarea>
                </div>
                <div class="form-actions">
                    <button type="button" class="btn" onclick="closeModal()">Cancel</button>
                    <button type="submit" class="btn btn-primary">Create</button>
                </div>
            </form>
        </div>
    </div>

    <script src="/plugins/rustmedia/assets/js/admin.js"></script>
    <script src="/plugins/rustmedia/assets/js/folders.js"></script>
</body>
</html>
"#,
            data.total_folders,
            self.render_tree(&data.tree),
            self.render_parent_options(&data.tree, ""),
        )
    }

    fn render_tree(&self, nodes: &[FolderTreeNode]) -> String {
        if nodes.is_empty() {
            return "<li class=\"empty\">No folders</li>".to_string();
        }

        nodes.iter().map(|node| {
            let children_html = if !node.children.is_empty() {
                format!(r#"<ul class="tree-children">{}</ul>"#, self.render_tree(&node.children))
            } else {
                String::new()
            };

            format!(r#"
                <li class="tree-item" data-id="{}">
                    <div class="tree-item-content" onclick="selectFolder('{}')">
                        <span class="tree-icon">{}</span>
                        <span class="tree-name">{}</span>
                        <span class="tree-count">{}</span>
                    </div>
                    {}
                </li>
            "#,
                node.folder.id,
                node.folder.id,
                if node.children.is_empty() { "📁" } else { "📂" },
                node.folder.name,
                node.folder.item_count,
                children_html,
            )
        }).collect::<Vec<_>>().join("\n")
    }

    fn render_parent_options(&self, nodes: &[FolderTreeNode], prefix: &str) -> String {
        nodes.iter().map(|node| {
            let indent = format!("{}{}", prefix, &node.folder.name);
            let mut html = format!(r#"<option value="{}">{}</option>"#, node.folder.id, indent);

            if !node.children.is_empty() {
                let child_prefix = format!("{}  ", prefix);
                html.push_str(&self.render_parent_options(&node.children, &child_prefix));
            }

            html
        }).collect::<Vec<_>>().join("\n")
    }

    /// Render single folder view
    pub async fn render_folder(&self, folder_id: &str) -> String {
        match self.get_folder_data(folder_id).await {
            Some(data) => {
                format!(r#"
                    <div class="folder-info">
                        <div class="folder-header">
                            <h2>{}</h2>
                            <div class="folder-actions">
                                <button class="btn btn-small" onclick="editFolder('{}')">Edit</button>
                                <button class="btn btn-small btn-danger" onclick="deleteFolder('{}')">Delete</button>
                            </div>
                        </div>

                        <div class="folder-meta">
                            <p><strong>Path:</strong> /{}</p>
                            <p><strong>Items:</strong> {}</p>
                            <p><strong>Size:</strong> {}</p>
                            <p><strong>Created:</strong> {}</p>
                        </div>

                        {}

                        {}

                        <div class="folder-quick-actions">
                            <a href="/admin/media/library?folder_id={}" class="btn">View Contents</a>
                            <a href="/admin/media/upload?folder_id={}" class="btn">Upload Here</a>
                        </div>
                    </div>
                "#,
                    data.name,
                    data.id,
                    data.id,
                    data.path,
                    data.item_count,
                    data.total_size,
                    data.created_at,
                    data.description.map(|d| format!("<p><strong>Description:</strong> {}</p>", d)).unwrap_or_default(),
                    self.render_children(&data.children),
                    data.id,
                    data.id,
                )
            }
            None => {
                "<p class=\"error\">Folder not found</p>".to_string()
            }
        }
    }

    fn render_children(&self, children: &[ChildFolder]) -> String {
        if children.is_empty() {
            return String::new();
        }

        let items: String = children.iter().map(|c| {
            format!(r#"
                <li>
                    <a href="#" onclick="selectFolder('{}'); return false;">
                        📁 {} <span class="count">({})</span>
                    </a>
                </li>
            "#, c.id, c.name, c.item_count)
        }).collect::<Vec<_>>().join("\n");

        format!(r#"
            <div class="subfolders">
                <h4>Subfolders</h4>
                <ul>{}</ul>
            </div>
        "#, items)
    }
}
