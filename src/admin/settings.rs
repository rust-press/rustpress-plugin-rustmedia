//! Media Settings Admin View

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::models::ImageSize;
use crate::settings::MediaSettings;

/// Settings form data
#[derive(Debug, Deserialize)]
pub struct SettingsForm {
    // Storage
    pub storage_backend: Option<String>,
    pub storage_path: Option<String>,
    pub base_url: Option<String>,

    // Upload limits
    pub max_file_size: Option<u64>,
    pub allowed_extensions: Option<String>,

    // Image processing
    pub jpeg_quality: Option<u8>,
    pub png_compression: Option<u8>,
    pub webp_quality: Option<u8>,
    pub auto_optimize: Option<bool>,
    pub strip_metadata: Option<bool>,
    pub convert_to_webp: Option<bool>,

    // Thumbnails
    pub generate_thumbnails: Option<bool>,

    // Organization
    pub organize_by_date: Option<bool>,
    pub date_format: Option<String>,
    pub slugify_filenames: Option<bool>,
    pub deduplicate: Option<bool>,
}

/// Settings page data
#[derive(Debug, Serialize)]
pub struct SettingsPageData {
    pub settings: MediaSettings,
    pub storage_backends: Vec<StorageBackendOption>,
    pub image_sizes: Vec<ImageSizeConfig>,
}

#[derive(Debug, Serialize)]
pub struct StorageBackendOption {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct ImageSizeConfig {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub mode: String,
    pub quality: u8,
    pub enabled: bool,
}

/// Settings view
pub struct SettingsView {
    settings: Arc<RwLock<MediaSettings>>,
}

impl SettingsView {
    pub fn new(settings: Arc<RwLock<MediaSettings>>) -> Self {
        Self { settings }
    }

    /// Get settings page data
    pub async fn get_data(&self) -> SettingsPageData {
        let settings = self.settings.read().await;

        let storage_backends = vec![
            StorageBackendOption {
                id: "local".to_string(),
                name: "Local Filesystem".to_string(),
                description: "Store files on the local server".to_string(),
            },
            StorageBackendOption {
                id: "s3".to_string(),
                name: "Amazon S3".to_string(),
                description: "Store files in Amazon S3 or compatible storage".to_string(),
            },
        ];

        let image_sizes: Vec<ImageSizeConfig> = settings.image_sizes.iter().map(|s| {
            ImageSizeConfig {
                name: s.name.clone(),
                width: s.width,
                height: s.height,
                mode: format!("{:?}", s.mode),
                quality: s.quality,
                enabled: s.enabled,
            }
        }).collect();

        SettingsPageData {
            settings: settings.clone(),
            storage_backends,
            image_sizes,
        }
    }

    /// Update settings
    pub async fn update(&self, form: SettingsForm) -> Result<(), String> {
        let mut settings = self.settings.write().await;

        // Storage settings
        if let Some(backend) = form.storage_backend {
            settings.storage_backend = backend;
        }
        if let Some(path) = form.storage_path {
            settings.storage_path = path;
        }
        if let Some(url) = form.base_url {
            settings.base_url = url;
        }

        // Upload limits
        if let Some(size) = form.max_file_size {
            settings.max_file_size = size;
        }
        if let Some(extensions) = form.allowed_extensions {
            settings.allowed_extensions = extensions.split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
        }

        // Image processing
        if let Some(q) = form.jpeg_quality {
            settings.jpeg_quality = q.clamp(1, 100);
        }
        if let Some(q) = form.png_compression {
            settings.png_compression = q.clamp(0, 9);
        }
        if let Some(q) = form.webp_quality {
            settings.webp_quality = q.clamp(1, 100);
        }
        if let Some(v) = form.auto_optimize {
            settings.auto_optimize = v;
        }
        if let Some(v) = form.strip_metadata {
            settings.strip_metadata = v;
        }
        if let Some(v) = form.convert_to_webp {
            settings.convert_to_webp = v;
        }

        // Thumbnails
        if let Some(v) = form.generate_thumbnails {
            settings.generate_thumbnails = v;
        }

        // Organization
        if let Some(v) = form.organize_by_date {
            settings.organize_by_date = v;
        }
        if let Some(format) = form.date_format {
            settings.date_format = format;
        }
        if let Some(v) = form.slugify_filenames {
            settings.slugify_filenames = v;
        }
        if let Some(v) = form.deduplicate {
            settings.deduplicate = v;
        }

        Ok(())
    }

    /// Render settings page HTML
    pub async fn render(&self) -> String {
        let data = self.get_data().await;

        format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Media Settings - RustMedia</title>
    <link rel="stylesheet" href="/plugins/rustmedia/assets/css/admin.css">
</head>
<body>
    <div class="rustmedia-admin">
        <header class="admin-header">
            <h1>Media Settings</h1>
            <nav class="admin-nav">
                <a href="/admin/media">Dashboard</a>
                <a href="/admin/media/library">Library</a>
                <a href="/admin/media/upload">Upload</a>
                <a href="/admin/media/folders">Folders</a>
                <a href="/admin/media/settings" class="active">Settings</a>
            </nav>
        </header>

        <main class="admin-content">
            <form id="settings-form" method="post" class="settings-form">
                <div class="settings-section">
                    <h2>Storage</h2>

                    <div class="form-group">
                        <label for="storage-backend">Storage Backend</label>
                        <select id="storage-backend" name="storage_backend">
                            {}
                        </select>
                    </div>

                    <div class="form-group">
                        <label for="storage-path">Storage Path</label>
                        <input type="text" id="storage-path" name="storage_path" value="{}">
                        <p class="help-text">Path where media files will be stored</p>
                    </div>

                    <div class="form-group">
                        <label for="base-url">Base URL</label>
                        <input type="text" id="base-url" name="base_url" value="{}">
                        <p class="help-text">Base URL for serving media files</p>
                    </div>
                </div>

                <div class="settings-section">
                    <h2>Upload Limits</h2>

                    <div class="form-group">
                        <label for="max-file-size">Maximum File Size (MB)</label>
                        <input type="number" id="max-file-size" name="max_file_size"
                               value="{}" min="1" max="1000">
                    </div>

                    <div class="form-group">
                        <label for="allowed-extensions">Allowed Extensions</label>
                        <input type="text" id="allowed-extensions" name="allowed_extensions"
                               value="{}">
                        <p class="help-text">Comma-separated list of allowed file extensions</p>
                    </div>
                </div>

                <div class="settings-section">
                    <h2>Image Processing</h2>

                    <div class="form-row">
                        <div class="form-group">
                            <label for="jpeg-quality">JPEG Quality (1-100)</label>
                            <input type="number" id="jpeg-quality" name="jpeg_quality"
                                   value="{}" min="1" max="100">
                        </div>

                        <div class="form-group">
                            <label for="png-compression">PNG Compression (0-9)</label>
                            <input type="number" id="png-compression" name="png_compression"
                                   value="{}" min="0" max="9">
                        </div>

                        <div class="form-group">
                            <label for="webp-quality">WebP Quality (1-100)</label>
                            <input type="number" id="webp-quality" name="webp_quality"
                                   value="{}" min="1" max="100">
                        </div>
                    </div>

                    <div class="form-group checkbox-group">
                        <label>
                            <input type="checkbox" name="auto_optimize" {}>
                            Automatically optimize uploaded images
                        </label>
                    </div>

                    <div class="form-group checkbox-group">
                        <label>
                            <input type="checkbox" name="strip_metadata" {}>
                            Strip EXIF and metadata from images
                        </label>
                    </div>

                    <div class="form-group checkbox-group">
                        <label>
                            <input type="checkbox" name="convert_to_webp" {}>
                            Convert images to WebP format
                        </label>
                    </div>
                </div>

                <div class="settings-section">
                    <h2>Thumbnails</h2>

                    <div class="form-group checkbox-group">
                        <label>
                            <input type="checkbox" name="generate_thumbnails" {}>
                            Automatically generate thumbnails
                        </label>
                    </div>

                    <div class="thumbnail-sizes">
                        <h3>Thumbnail Sizes</h3>
                        <table class="sizes-table">
                            <thead>
                                <tr>
                                    <th>Name</th>
                                    <th>Width</th>
                                    <th>Height</th>
                                    <th>Mode</th>
                                    <th>Quality</th>
                                    <th>Enabled</th>
                                </tr>
                            </thead>
                            <tbody>
                                {}
                            </tbody>
                        </table>
                        <button type="button" class="btn btn-small" onclick="addThumbnailSize()">
                            + Add Size
                        </button>
                    </div>
                </div>

                <div class="settings-section">
                    <h2>Organization</h2>

                    <div class="form-group checkbox-group">
                        <label>
                            <input type="checkbox" name="organize_by_date" {}>
                            Organize uploads by date (year/month)
                        </label>
                    </div>

                    <div class="form-group">
                        <label for="date-format">Date Format</label>
                        <select id="date-format" name="date_format">
                            <option value="%Y/%m" {}>YYYY/MM (2024/01)</option>
                            <option value="%Y/%m/%d" {}>YYYY/MM/DD (2024/01/15)</option>
                            <option value="%Y" {}>YYYY (2024)</option>
                        </select>
                    </div>

                    <div class="form-group checkbox-group">
                        <label>
                            <input type="checkbox" name="slugify_filenames" {}>
                            Slugify filenames (remove special characters)
                        </label>
                    </div>

                    <div class="form-group checkbox-group">
                        <label>
                            <input type="checkbox" name="deduplicate" {}>
                            Detect and prevent duplicate uploads
                        </label>
                    </div>
                </div>

                <div class="form-actions">
                    <button type="submit" class="btn btn-primary">Save Settings</button>
                    <button type="reset" class="btn">Reset</button>
                </div>
            </form>
        </main>
    </div>

    <script src="/plugins/rustmedia/assets/js/admin.js"></script>
    <script src="/plugins/rustmedia/assets/js/settings.js"></script>
</body>
</html>
"#,
            self.render_storage_options(&data.storage_backends, &data.settings.storage_backend),
            data.settings.storage_path,
            data.settings.base_url,
            data.settings.max_file_size / (1024 * 1024),
            data.settings.allowed_extensions.join(", "),
            data.settings.jpeg_quality,
            data.settings.png_compression,
            data.settings.webp_quality,
            if data.settings.auto_optimize { "checked" } else { "" },
            if data.settings.strip_metadata { "checked" } else { "" },
            if data.settings.convert_to_webp { "checked" } else { "" },
            if data.settings.generate_thumbnails { "checked" } else { "" },
            self.render_thumbnail_sizes(&data.image_sizes),
            if data.settings.organize_by_date { "checked" } else { "" },
            if data.settings.date_format == "%Y/%m" { "selected" } else { "" },
            if data.settings.date_format == "%Y/%m/%d" { "selected" } else { "" },
            if data.settings.date_format == "%Y" { "selected" } else { "" },
            if data.settings.slugify_filenames { "checked" } else { "" },
            if data.settings.deduplicate { "checked" } else { "" },
        )
    }

    fn render_storage_options(&self, backends: &[StorageBackendOption], selected: &str) -> String {
        backends.iter().map(|b| {
            let selected_attr = if b.id == selected { "selected" } else { "" };
            format!(r#"<option value="{}" {}>{}</option>"#, b.id, selected_attr, b.name)
        }).collect::<Vec<_>>().join("\n")
    }

    fn render_thumbnail_sizes(&self, sizes: &[ImageSizeConfig]) -> String {
        sizes.iter().map(|s| {
            format!(r#"
                <tr data-size="{}">
                    <td><input type="text" value="{}" name="size_name[]"></td>
                    <td><input type="number" value="{}" name="size_width[]" min="1"></td>
                    <td><input type="number" value="{}" name="size_height[]" min="1"></td>
                    <td>
                        <select name="size_mode[]">
                            <option value="Fit" {}>Fit</option>
                            <option value="Fill" {}>Fill</option>
                            <option value="Exact" {}>Exact</option>
                        </select>
                    </td>
                    <td><input type="number" value="{}" name="size_quality[]" min="1" max="100"></td>
                    <td><input type="checkbox" name="size_enabled[]" {}></td>
                </tr>
            "#,
                s.name,
                s.name,
                s.width,
                s.height,
                if s.mode == "Fit" { "selected" } else { "" },
                if s.mode == "Fill" { "selected" } else { "" },
                if s.mode == "Exact" { "selected" } else { "" },
                s.quality,
                if s.enabled { "checked" } else { "" },
            )
        }).collect::<Vec<_>>().join("\n")
    }
}
