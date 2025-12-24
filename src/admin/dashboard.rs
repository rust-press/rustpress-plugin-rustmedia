//! Media Dashboard Admin View

use std::sync::Arc;
use serde::Serialize;
use crate::services::{MediaService, FolderService, media::MediaStats};

/// Dashboard view data
#[derive(Debug, Serialize)]
pub struct DashboardData {
    pub stats: MediaStats,
    pub recent_uploads: Vec<RecentUpload>,
    pub storage_usage: StorageUsage,
    pub media_by_type: Vec<MediaTypeCount>,
    pub top_folders: Vec<TopFolder>,
}

#[derive(Debug, Serialize)]
pub struct RecentUpload {
    pub id: String,
    pub filename: String,
    pub thumbnail_url: Option<String>,
    pub media_type: String,
    pub size_formatted: String,
    pub uploaded_at: String,
}

#[derive(Debug, Serialize)]
pub struct StorageUsage {
    pub used: u64,
    pub used_formatted: String,
    pub limit: Option<u64>,
    pub limit_formatted: Option<String>,
    pub percent_used: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct MediaTypeCount {
    pub media_type: String,
    pub count: u64,
    pub size: u64,
    pub size_formatted: String,
    pub percent: f64,
}

#[derive(Debug, Serialize)]
pub struct TopFolder {
    pub id: String,
    pub name: String,
    pub item_count: u32,
    pub total_size: String,
}

/// Dashboard view
pub struct DashboardView {
    media_service: Arc<MediaService>,
    folder_service: Arc<FolderService>,
    storage_limit: Option<u64>,
}

impl DashboardView {
    pub fn new(
        media_service: Arc<MediaService>,
        folder_service: Arc<FolderService>,
    ) -> Self {
        Self {
            media_service,
            folder_service,
            storage_limit: None,
        }
    }

    /// Set storage limit
    pub fn set_storage_limit(&mut self, limit: u64) {
        self.storage_limit = Some(limit);
    }

    /// Get dashboard data
    pub async fn get_data(&self) -> DashboardData {
        let stats = self.media_service.get_stats().await;

        // Get recent uploads
        let recent = self.media_service.get_recent(10).await;
        let recent_uploads: Vec<RecentUpload> = recent.into_iter().map(|m| {
            RecentUpload {
                id: m.id.to_string(),
                filename: m.filename.clone(),
                thumbnail_url: m.thumbnails.first().map(|t| t.url.clone()),
                media_type: format!("{}", m.media_type),
                size_formatted: m.formatted_size(),
                uploaded_at: m.uploaded_at.to_rfc3339(),
            }
        }).collect();

        // Storage usage
        let storage_usage = StorageUsage {
            used: stats.total_size,
            used_formatted: Self::format_size(stats.total_size),
            limit: self.storage_limit,
            limit_formatted: self.storage_limit.map(Self::format_size),
            percent_used: self.storage_limit.map(|l| (stats.total_size as f64 / l as f64) * 100.0),
        };

        // Media by type
        let total_size = stats.total_size as f64;
        let media_by_type = vec![
            MediaTypeCount {
                media_type: "Images".to_string(),
                count: stats.images,
                size: stats.images * 500_000, // Estimate
                size_formatted: Self::format_size(stats.images * 500_000),
                percent: if total_size > 0.0 { (stats.images as f64 * 500_000.0 / total_size) * 100.0 } else { 0.0 },
            },
            MediaTypeCount {
                media_type: "Videos".to_string(),
                count: stats.videos,
                size: stats.videos * 10_000_000, // Estimate
                size_formatted: Self::format_size(stats.videos * 10_000_000),
                percent: if total_size > 0.0 { (stats.videos as f64 * 10_000_000.0 / total_size) * 100.0 } else { 0.0 },
            },
            MediaTypeCount {
                media_type: "Audio".to_string(),
                count: stats.audio,
                size: stats.audio * 5_000_000, // Estimate
                size_formatted: Self::format_size(stats.audio * 5_000_000),
                percent: if total_size > 0.0 { (stats.audio as f64 * 5_000_000.0 / total_size) * 100.0 } else { 0.0 },
            },
            MediaTypeCount {
                media_type: "Documents".to_string(),
                count: stats.documents,
                size: stats.documents * 200_000, // Estimate
                size_formatted: Self::format_size(stats.documents * 200_000),
                percent: if total_size > 0.0 { (stats.documents as f64 * 200_000.0 / total_size) * 100.0 } else { 0.0 },
            },
            MediaTypeCount {
                media_type: "Other".to_string(),
                count: stats.other,
                size: stats.other * 100_000, // Estimate
                size_formatted: Self::format_size(stats.other * 100_000),
                percent: if total_size > 0.0 { (stats.other as f64 * 100_000.0 / total_size) * 100.0 } else { 0.0 },
            },
        ];

        // Top folders
        let folders = self.folder_service.get_all().await;
        let mut sorted_folders = folders;
        sorted_folders.sort_by(|a, b| b.item_count.cmp(&a.item_count));
        let top_folders: Vec<TopFolder> = sorted_folders.into_iter().take(5).map(|f| {
            TopFolder {
                id: f.id.to_string(),
                name: f.name,
                item_count: f.item_count,
                total_size: f.formatted_size(),
            }
        }).collect();

        DashboardData {
            stats,
            recent_uploads,
            storage_usage,
            media_by_type,
            top_folders,
        }
    }

    /// Render dashboard HTML
    pub async fn render(&self) -> String {
        let data = self.get_data().await;

        format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Media Dashboard - RustMedia</title>
    <link rel="stylesheet" href="/plugins/rustmedia/assets/css/admin.css">
</head>
<body>
    <div class="rustmedia-admin">
        <header class="admin-header">
            <h1>Media Dashboard</h1>
            <nav class="admin-nav">
                <a href="/admin/media" class="active">Dashboard</a>
                <a href="/admin/media/library">Library</a>
                <a href="/admin/media/upload">Upload</a>
                <a href="/admin/media/folders">Folders</a>
                <a href="/admin/media/settings">Settings</a>
            </nav>
        </header>

        <main class="admin-content">
            <div class="stats-grid">
                <div class="stat-card">
                    <div class="stat-icon">📁</div>
                    <div class="stat-value">{}</div>
                    <div class="stat-label">Total Files</div>
                </div>
                <div class="stat-card">
                    <div class="stat-icon">🖼️</div>
                    <div class="stat-value">{}</div>
                    <div class="stat-label">Images</div>
                </div>
                <div class="stat-card">
                    <div class="stat-icon">🎬</div>
                    <div class="stat-value">{}</div>
                    <div class="stat-label">Videos</div>
                </div>
                <div class="stat-card">
                    <div class="stat-icon">💾</div>
                    <div class="stat-value">{}</div>
                    <div class="stat-label">Storage Used</div>
                </div>
            </div>

            <div class="dashboard-grid">
                <div class="panel recent-uploads">
                    <h2>Recent Uploads</h2>
                    <div class="upload-list">
                        {}
                    </div>
                </div>

                <div class="panel storage-breakdown">
                    <h2>Storage by Type</h2>
                    <div class="storage-chart">
                        {}
                    </div>
                </div>

                <div class="panel top-folders">
                    <h2>Top Folders</h2>
                    <ul class="folder-list">
                        {}
                    </ul>
                </div>
            </div>
        </main>
    </div>
    <script src="/plugins/rustmedia/assets/js/admin.js"></script>
</body>
</html>
"#,
            data.stats.total_count,
            data.stats.images,
            data.stats.videos,
            data.storage_usage.used_formatted,
            self.render_recent_uploads(&data.recent_uploads),
            self.render_storage_chart(&data.media_by_type),
            self.render_top_folders(&data.top_folders),
        )
    }

    fn render_recent_uploads(&self, uploads: &[RecentUpload]) -> String {
        if uploads.is_empty() {
            return "<p class=\"empty\">No uploads yet</p>".to_string();
        }

        uploads.iter().map(|u| {
            let thumbnail = u.thumbnail_url.as_ref()
                .map(|url| format!(r#"<img src="{}" alt="{}">"#, url, u.filename))
                .unwrap_or_else(|| format!(r#"<div class="file-icon">{}</div>"#, self.get_type_icon(&u.media_type)));

            format!(r#"
                <div class="upload-item">
                    <div class="upload-thumb">{}</div>
                    <div class="upload-info">
                        <span class="filename">{}</span>
                        <span class="meta">{} • {}</span>
                    </div>
                </div>
            "#, thumbnail, u.filename, u.size_formatted, u.uploaded_at)
        }).collect::<Vec<_>>().join("\n")
    }

    fn render_storage_chart(&self, types: &[MediaTypeCount]) -> String {
        types.iter().map(|t| {
            format!(r#"
                <div class="storage-bar">
                    <div class="bar-label">{}</div>
                    <div class="bar-track">
                        <div class="bar-fill" style="width: {:.1}%"></div>
                    </div>
                    <div class="bar-value">{} ({} files)</div>
                </div>
            "#, t.media_type, t.percent, t.size_formatted, t.count)
        }).collect::<Vec<_>>().join("\n")
    }

    fn render_top_folders(&self, folders: &[TopFolder]) -> String {
        if folders.is_empty() {
            return "<li class=\"empty\">No folders</li>".to_string();
        }

        folders.iter().map(|f| {
            format!(r#"
                <li>
                    <a href="/admin/media/folders/{}">
                        <span class="folder-name">📁 {}</span>
                        <span class="folder-meta">{} items • {}</span>
                    </a>
                </li>
            "#, f.id, f.name, f.item_count, f.total_size)
        }).collect::<Vec<_>>().join("\n")
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

    fn format_size(bytes: u64) -> String {
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
}
