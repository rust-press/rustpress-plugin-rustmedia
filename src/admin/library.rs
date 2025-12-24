//! Media Library Admin View

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{MediaFilter, MediaType, SortBy, SortOrder};
use crate::services::{MediaService, FolderService};
use crate::handlers::{MediaHandler, MediaItemResponse};

/// Library filter query params
#[derive(Debug, Deserialize)]
pub struct LibraryQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub folder_id: Option<String>,
    pub media_type: Option<String>,
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub view: Option<String>, // grid or list
}

/// Library page data
#[derive(Debug, Serialize)]
pub struct LibraryData {
    pub items: Vec<MediaItemResponse>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
    pub folders: Vec<FolderOption>,
    pub current_folder: Option<FolderOption>,
    pub breadcrumbs: Vec<BreadcrumbItem>,
    pub filters: AppliedFilters,
}

#[derive(Debug, Serialize)]
pub struct FolderOption {
    pub id: String,
    pub name: String,
    pub path: String,
    pub item_count: u32,
}

#[derive(Debug, Serialize)]
pub struct BreadcrumbItem {
    pub id: Option<String>,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct AppliedFilters {
    pub folder_id: Option<String>,
    pub media_type: Option<String>,
    pub search: Option<String>,
    pub sort_by: String,
    pub sort_order: String,
    pub view: String,
}

/// Library view
pub struct LibraryView {
    media_service: Arc<MediaService>,
    folder_service: Arc<FolderService>,
    media_handler: Arc<MediaHandler>,
}

impl LibraryView {
    pub fn new(
        media_service: Arc<MediaService>,
        folder_service: Arc<FolderService>,
        media_handler: Arc<MediaHandler>,
    ) -> Self {
        Self {
            media_service,
            folder_service,
            media_handler,
        }
    }

    /// Get library data
    pub async fn get_data(&self, query: LibraryQuery) -> LibraryData {
        let page = query.page.unwrap_or(1);
        let per_page = query.per_page.unwrap_or(24);

        // Parse filters
        let folder_id = query.folder_id.as_ref()
            .and_then(|f| Uuid::parse_str(f).ok());

        let media_type = query.media_type.as_ref()
            .and_then(|t| match t.as_str() {
                "image" => Some(MediaType::Image),
                "video" => Some(MediaType::Video),
                "audio" => Some(MediaType::Audio),
                "document" => Some(MediaType::Document),
                _ => None,
            });

        let sort_by = query.sort_by.as_ref()
            .map(|s| match s.as_str() {
                "name" => SortBy::Name,
                "size" => SortBy::Size,
                "type" => SortBy::Type,
                _ => SortBy::Date,
            })
            .unwrap_or(SortBy::Date);

        let sort_order = query.sort_order.as_ref()
            .map(|s| match s.as_str() {
                "asc" => SortOrder::Asc,
                _ => SortOrder::Desc,
            })
            .unwrap_or(SortOrder::Desc);

        // Build filter
        let filter = MediaFilter {
            folder_id,
            media_type,
            search: query.search.clone(),
            tags: None,
            uploaded_by: None,
            date_from: None,
            date_to: None,
            sort_by,
            sort_order,
            page,
            per_page,
        };

        // Get media
        let result = self.media_handler.list(filter).await;

        // Convert to response items
        let items: Vec<MediaItemResponse> = result.items.into_iter().map(|m| {
            MediaItemResponse {
                id: m.id.to_string(),
                filename: m.filename,
                title: m.title,
                description: m.description,
                alt_text: m.alt_text,
                mime_type: m.mime_type,
                media_type: format!("{}", m.media_type),
                size: m.size,
                size_formatted: m.formatted_size(),
                url: m.url,
                dimensions: m.dimensions.map(|d| crate::handlers::media::DimensionsResponse {
                    width: d.width,
                    height: d.height,
                }),
                thumbnails: m.thumbnails.iter().map(|t| crate::handlers::media::ThumbnailResponse {
                    name: t.size_name.clone(),
                    url: t.url.clone(),
                    width: t.width,
                    height: t.height,
                }).collect(),
                uploaded_at: m.uploaded_at.to_rfc3339(),
                tags: m.tags,
            }
        }).collect();

        // Get folders
        let all_folders = self.folder_service.get_all().await;
        let folders: Vec<FolderOption> = all_folders.iter().map(|f| FolderOption {
            id: f.id.to_string(),
            name: f.name.clone(),
            path: f.path.clone(),
            item_count: f.item_count,
        }).collect();

        // Current folder
        let current_folder = folder_id.and_then(|fid| {
            all_folders.iter().find(|f| f.id == fid).map(|f| FolderOption {
                id: f.id.to_string(),
                name: f.name.clone(),
                path: f.path.clone(),
                item_count: f.item_count,
            })
        });

        // Breadcrumbs
        let mut breadcrumbs = vec![
            BreadcrumbItem {
                id: None,
                name: "All Media".to_string(),
                url: "/admin/media/library".to_string(),
            }
        ];

        if let Some(fid) = folder_id {
            let folder_breadcrumbs = self.folder_service.get_breadcrumbs(fid).await;
            for bc in folder_breadcrumbs {
                breadcrumbs.push(BreadcrumbItem {
                    id: Some(bc.id.to_string()),
                    name: bc.name,
                    url: format!("/admin/media/library?folder_id={}", bc.id),
                });
            }
        }

        let total_pages = ((result.total as f64) / (per_page as f64)).ceil() as u32;

        LibraryData {
            items,
            total: result.total,
            page,
            per_page,
            total_pages,
            folders,
            current_folder,
            breadcrumbs,
            filters: AppliedFilters {
                folder_id: query.folder_id,
                media_type: query.media_type,
                search: query.search,
                sort_by: query.sort_by.unwrap_or_else(|| "date".to_string()),
                sort_order: query.sort_order.unwrap_or_else(|| "desc".to_string()),
                view: query.view.unwrap_or_else(|| "grid".to_string()),
            },
        }
    }

    /// Render library HTML
    pub async fn render(&self, query: LibraryQuery) -> String {
        let data = self.get_data(query).await;

        format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Media Library - RustMedia</title>
    <link rel="stylesheet" href="/plugins/rustmedia/assets/css/admin.css">
</head>
<body>
    <div class="rustmedia-admin">
        <header class="admin-header">
            <h1>Media Library</h1>
            <nav class="admin-nav">
                <a href="/admin/media">Dashboard</a>
                <a href="/admin/media/library" class="active">Library</a>
                <a href="/admin/media/upload">Upload</a>
                <a href="/admin/media/folders">Folders</a>
                <a href="/admin/media/settings">Settings</a>
            </nav>
        </header>

        <main class="admin-content">
            <div class="library-toolbar">
                <div class="breadcrumbs">
                    {}
                </div>

                <div class="filters">
                    <form method="get" class="filter-form">
                        <input type="text" name="search" placeholder="Search..." value="{}">

                        <select name="folder_id">
                            <option value="">All Folders</option>
                            {}
                        </select>

                        <select name="media_type">
                            <option value="">All Types</option>
                            <option value="image" {}>Images</option>
                            <option value="video" {}>Videos</option>
                            <option value="audio" {}>Audio</option>
                            <option value="document" {}>Documents</option>
                        </select>

                        <select name="sort_by">
                            <option value="date" {}>Date</option>
                            <option value="name" {}>Name</option>
                            <option value="size" {}>Size</option>
                            <option value="type" {}>Type</option>
                        </select>

                        <button type="submit">Filter</button>
                    </form>

                    <div class="view-toggle">
                        <button class="view-btn {}" data-view="grid">Grid</button>
                        <button class="view-btn {}" data-view="list">List</button>
                    </div>
                </div>
            </div>

            <div class="media-grid view-{}">
                {}
            </div>

            <div class="pagination">
                {}
            </div>
        </main>
    </div>

    <div id="media-modal" class="modal hidden">
        <div class="modal-content">
            <button class="modal-close">&times;</button>
            <div class="modal-body"></div>
        </div>
    </div>

    <script src="/plugins/rustmedia/assets/js/admin.js"></script>
</body>
</html>
"#,
            self.render_breadcrumbs(&data.breadcrumbs),
            data.filters.search.as_deref().unwrap_or(""),
            self.render_folder_options(&data.folders, data.filters.folder_id.as_deref()),
            if data.filters.media_type.as_deref() == Some("image") { "selected" } else { "" },
            if data.filters.media_type.as_deref() == Some("video") { "selected" } else { "" },
            if data.filters.media_type.as_deref() == Some("audio") { "selected" } else { "" },
            if data.filters.media_type.as_deref() == Some("document") { "selected" } else { "" },
            if data.filters.sort_by == "date" { "selected" } else { "" },
            if data.filters.sort_by == "name" { "selected" } else { "" },
            if data.filters.sort_by == "size" { "selected" } else { "" },
            if data.filters.sort_by == "type" { "selected" } else { "" },
            if data.filters.view == "grid" { "active" } else { "" },
            if data.filters.view == "list" { "active" } else { "" },
            data.filters.view,
            self.render_media_items(&data.items, &data.filters.view),
            self.render_pagination(data.page, data.total_pages),
        )
    }

    fn render_breadcrumbs(&self, breadcrumbs: &[BreadcrumbItem]) -> String {
        breadcrumbs.iter().enumerate().map(|(i, bc)| {
            if i == breadcrumbs.len() - 1 {
                format!(r#"<span class="breadcrumb-current">{}</span>"#, bc.name)
            } else {
                format!(r#"<a href="{}" class="breadcrumb-link">{}</a> / "#, bc.url, bc.name)
            }
        }).collect::<Vec<_>>().join("")
    }

    fn render_folder_options(&self, folders: &[FolderOption], selected: Option<&str>) -> String {
        folders.iter().map(|f| {
            let selected_attr = if selected == Some(&f.id) { "selected" } else { "" };
            format!(r#"<option value="{}" {}>{} ({})</option>"#, f.id, selected_attr, f.name, f.item_count)
        }).collect::<Vec<_>>().join("\n")
    }

    fn render_media_items(&self, items: &[MediaItemResponse], view: &str) -> String {
        if items.is_empty() {
            return r#"<div class="empty-state">
                <p>No media found</p>
                <a href="/admin/media/upload" class="btn btn-primary">Upload Files</a>
            </div>"#.to_string();
        }

        items.iter().map(|item| {
            let thumbnail = item.thumbnails.first()
                .map(|t| format!(r#"<img src="{}" alt="{}">"#, t.url, item.filename))
                .unwrap_or_else(|| format!(r#"<div class="file-icon">{}</div>"#, self.get_type_icon(&item.media_type)));

            if view == "list" {
                format!(r#"
                    <div class="media-item list-item" data-id="{}">
                        <div class="item-thumb">{}</div>
                        <div class="item-name">{}</div>
                        <div class="item-type">{}</div>
                        <div class="item-size">{}</div>
                        <div class="item-date">{}</div>
                        <div class="item-actions">
                            <button class="btn-icon btn-view" title="View">👁️</button>
                            <button class="btn-icon btn-edit" title="Edit">✏️</button>
                            <button class="btn-icon btn-delete" title="Delete">🗑️</button>
                        </div>
                    </div>
                "#, item.id, thumbnail, item.filename, item.media_type, item.size_formatted, item.uploaded_at)
            } else {
                format!(r#"
                    <div class="media-item grid-item" data-id="{}">
                        <div class="item-thumb">{}</div>
                        <div class="item-overlay">
                            <span class="item-name">{}</span>
                            <span class="item-size">{}</span>
                        </div>
                        <input type="checkbox" class="item-select">
                    </div>
                "#, item.id, thumbnail, item.filename, item.size_formatted)
            }
        }).collect::<Vec<_>>().join("\n")
    }

    fn render_pagination(&self, current: u32, total: u32) -> String {
        if total <= 1 {
            return String::new();
        }

        let mut html = String::from("<div class=\"pagination-controls\">");

        if current > 1 {
            html.push_str(&format!(r#"<a href="?page={}" class="page-link">« Previous</a>"#, current - 1));
        }

        for page in 1..=total {
            if page == current {
                html.push_str(&format!(r#"<span class="page-current">{}</span>"#, page));
            } else if (page as i32 - current as i32).abs() <= 2 || page == 1 || page == total {
                html.push_str(&format!(r#"<a href="?page={}" class="page-link">{}</a>"#, page, page));
            } else if page == 2 || page == total - 1 {
                html.push_str(r#"<span class="page-ellipsis">...</span>"#);
            }
        }

        if current < total {
            html.push_str(&format!(r#"<a href="?page={}" class="page-link">Next »</a>"#, current + 1));
        }

        html.push_str("</div>");
        html
    }

    fn get_type_icon(&self, media_type: &str) -> &'static str {
        match media_type {
            "Image" => "🖼️",
            "Video" => "🎬",
            "Audio" => "🎵",
            "Document" => "📄",
            _ => "📁",
        }
    }
}
