//! Upload Admin View

use std::sync::Arc;
use serde::Serialize;

use crate::services::{FolderService, UploadService};

/// Upload page data
#[derive(Debug, Serialize)]
pub struct UploadPageData {
    pub folders: Vec<FolderOption>,
    pub max_file_size: u64,
    pub max_file_size_formatted: String,
    pub allowed_types: Vec<String>,
    pub allowed_extensions: Vec<String>,
    pub chunk_size: usize,
}

#[derive(Debug, Serialize)]
pub struct FolderOption {
    pub id: String,
    pub name: String,
    pub path: String,
}

/// Upload view
pub struct UploadView {
    folder_service: Arc<FolderService>,
    upload_service: Arc<UploadService>,
}

impl UploadView {
    pub fn new(
        folder_service: Arc<FolderService>,
        upload_service: Arc<UploadService>,
    ) -> Self {
        Self {
            folder_service,
            upload_service,
        }
    }

    /// Get upload page data
    pub async fn get_data(&self) -> UploadPageData {
        let folders = self.folder_service.get_all().await;
        let folder_options: Vec<FolderOption> = folders.into_iter().map(|f| FolderOption {
            id: f.id.to_string(),
            name: f.name,
            path: f.path,
        }).collect();

        let max_size = self.upload_service.get_max_file_size();

        UploadPageData {
            folders: folder_options,
            max_file_size: max_size,
            max_file_size_formatted: Self::format_size(max_size),
            allowed_types: self.upload_service.get_allowed_types(),
            allowed_extensions: self.upload_service.get_allowed_extensions(),
            chunk_size: 5 * 1024 * 1024, // 5MB
        }
    }

    /// Render upload page HTML
    pub async fn render(&self) -> String {
        let data = self.get_data().await;

        format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Upload Media - RustMedia</title>
    <link rel="stylesheet" href="/plugins/rustmedia/assets/css/admin.css">
</head>
<body>
    <div class="rustmedia-admin">
        <header class="admin-header">
            <h1>Upload Media</h1>
            <nav class="admin-nav">
                <a href="/admin/media">Dashboard</a>
                <a href="/admin/media/library">Library</a>
                <a href="/admin/media/upload" class="active">Upload</a>
                <a href="/admin/media/folders">Folders</a>
                <a href="/admin/media/settings">Settings</a>
            </nav>
        </header>

        <main class="admin-content">
            <div class="upload-container">
                <div class="upload-dropzone" id="dropzone">
                    <div class="dropzone-content">
                        <div class="dropzone-icon">📤</div>
                        <h3>Drop files here to upload</h3>
                        <p>or click to select files</p>
                        <p class="dropzone-info">
                            Max file size: {} <br>
                            Allowed: {}
                        </p>
                    </div>
                    <input type="file" id="file-input" multiple hidden>
                </div>

                <div class="upload-options">
                    <div class="form-group">
                        <label for="folder-select">Upload to folder:</label>
                        <select id="folder-select">
                            <option value="">Root (No folder)</option>
                            {}
                        </select>
                    </div>

                    <div class="form-group">
                        <label>
                            <input type="checkbox" id="optimize-images" checked>
                            Optimize images automatically
                        </label>
                    </div>

                    <div class="form-group">
                        <label>
                            <input type="checkbox" id="generate-thumbnails" checked>
                            Generate thumbnails
                        </label>
                    </div>
                </div>

                <div class="upload-queue" id="upload-queue">
                    <h3>Upload Queue</h3>
                    <div class="queue-list" id="queue-list"></div>
                </div>

                <div class="upload-complete" id="upload-complete" style="display: none;">
                    <h3>Completed Uploads</h3>
                    <div class="complete-list" id="complete-list"></div>
                </div>
            </div>
        </main>
    </div>

    <script>
        window.RUSTMEDIA_CONFIG = {{
            maxFileSize: {},
            chunkSize: {},
            allowedTypes: {},
            allowedExtensions: {}
        }};
    </script>
    <script src="/plugins/rustmedia/assets/js/admin.js"></script>
    <script src="/plugins/rustmedia/assets/js/upload.js"></script>
</body>
</html>
"#,
            data.max_file_size_formatted,
            data.allowed_extensions.join(", "),
            self.render_folder_options(&data.folders),
            data.max_file_size,
            data.chunk_size,
            serde_json::to_string(&data.allowed_types).unwrap_or_else(|_| "[]".to_string()),
            serde_json::to_string(&data.allowed_extensions).unwrap_or_else(|_| "[]".to_string()),
        )
    }

    fn render_folder_options(&self, folders: &[FolderOption]) -> String {
        folders.iter().map(|f| {
            format!(r#"<option value="{}">{}</option>"#, f.id, f.name)
        }).collect::<Vec<_>>().join("\n")
    }

    fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.0} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.0} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.0} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
}
