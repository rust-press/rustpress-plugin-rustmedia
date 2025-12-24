//! RustMedia Plugin Entry Point

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::settings::MediaSettings;
use crate::services::{
    StorageService, ImageService, MediaService,
    FolderService, OptimizerService, UploadService,
};
use crate::handlers::{MediaHandler, FolderHandler, UploadHandler};
use crate::admin::{DashboardView, LibraryView, UploadView, FoldersView, SettingsView};

/// RustMedia Plugin
pub struct RustMediaPlugin {
    /// Plugin settings
    settings: Arc<RwLock<MediaSettings>>,

    /// Services
    storage_service: Arc<StorageService>,
    image_service: Arc<ImageService>,
    media_service: Arc<MediaService>,
    folder_service: Arc<FolderService>,
    optimizer_service: Arc<OptimizerService>,
    upload_service: Arc<UploadService>,

    /// Handlers
    media_handler: Arc<MediaHandler>,
    folder_handler: Arc<FolderHandler>,
    upload_handler: Arc<UploadHandler>,

    /// Admin views
    dashboard_view: DashboardView,
    library_view: LibraryView,
    upload_view: UploadView,
    folders_view: FoldersView,
    settings_view: SettingsView,
}

impl RustMediaPlugin {
    /// Create a new RustMedia plugin instance
    pub fn new() -> Self {
        let settings = Arc::new(RwLock::new(MediaSettings::default()));

        // Create services
        let storage_service = Arc::new(StorageService::new("uploads/media"));
        let image_service = Arc::new(ImageService::new());
        let folder_service = Arc::new(FolderService::new());
        let optimizer_service = Arc::new(OptimizerService::new(
            Arc::clone(&image_service),
            Arc::clone(&storage_service),
        ));
        let media_service = Arc::new(MediaService::new(
            Arc::clone(&storage_service),
            Arc::clone(&image_service),
            Arc::clone(&folder_service),
        ));
        let upload_service = Arc::new(UploadService::new(
            Arc::clone(&storage_service),
            Arc::clone(&image_service),
            Arc::clone(&media_service),
            Arc::clone(&optimizer_service),
        ));

        // Create handlers
        let media_handler = Arc::new(MediaHandler::new(Arc::clone(&media_service)));
        let folder_handler = Arc::new(FolderHandler::new(Arc::clone(&folder_service)));
        let upload_handler = Arc::new(UploadHandler::new(
            Arc::clone(&upload_service),
            Arc::clone(&media_service),
        ));

        // Create admin views
        let dashboard_view = DashboardView::new(
            Arc::clone(&media_service),
            Arc::clone(&folder_service),
        );
        let library_view = LibraryView::new(
            Arc::clone(&media_service),
            Arc::clone(&folder_service),
            Arc::clone(&media_handler),
        );
        let upload_view = UploadView::new(
            Arc::clone(&folder_service),
            Arc::clone(&upload_service),
        );
        let folders_view = FoldersView::new(Arc::clone(&folder_service));
        let settings_view = SettingsView::new(Arc::clone(&settings));

        Self {
            settings,
            storage_service,
            image_service,
            media_service,
            folder_service,
            optimizer_service,
            upload_service,
            media_handler,
            folder_handler,
            upload_handler,
            dashboard_view,
            library_view,
            upload_view,
            folders_view,
            settings_view,
        }
    }

    /// Create with custom settings
    pub fn with_settings(settings: MediaSettings) -> Self {
        let mut plugin = Self::new();
        plugin.settings = Arc::new(RwLock::new(settings));
        plugin
    }

    /// Initialize the plugin
    pub async fn initialize(&self) -> Result<(), String> {
        // Create upload directory
        self.storage_service.create_directory("").await
            .map_err(|e| e.to_string())?;

        // Create thumbnail directories
        self.storage_service.create_directory("thumbnails").await
            .map_err(|e| e.to_string())?;

        // Create temp directory for chunked uploads
        self.storage_service.create_directory("temp/chunks").await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Get plugin name
    pub fn name(&self) -> &'static str {
        "RustMedia"
    }

    /// Get plugin version
    pub fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// Get plugin description
    pub fn description(&self) -> &'static str {
        "Complete media management for RustPress"
    }

    // Service accessors
    pub fn storage_service(&self) -> &Arc<StorageService> {
        &self.storage_service
    }

    pub fn image_service(&self) -> &Arc<ImageService> {
        &self.image_service
    }

    pub fn media_service(&self) -> &Arc<MediaService> {
        &self.media_service
    }

    pub fn folder_service(&self) -> &Arc<FolderService> {
        &self.folder_service
    }

    pub fn optimizer_service(&self) -> &Arc<OptimizerService> {
        &self.optimizer_service
    }

    pub fn upload_service(&self) -> &Arc<UploadService> {
        &self.upload_service
    }

    // Handler accessors
    pub fn media_handler(&self) -> &Arc<MediaHandler> {
        &self.media_handler
    }

    pub fn folder_handler(&self) -> &Arc<FolderHandler> {
        &self.folder_handler
    }

    pub fn upload_handler(&self) -> &Arc<UploadHandler> {
        &self.upload_handler
    }

    // Admin view accessors
    pub fn dashboard_view(&self) -> &DashboardView {
        &self.dashboard_view
    }

    pub fn library_view(&self) -> &LibraryView {
        &self.library_view
    }

    pub fn upload_view(&self) -> &UploadView {
        &self.upload_view
    }

    pub fn folders_view(&self) -> &FoldersView {
        &self.folders_view
    }

    pub fn settings_view(&self) -> &SettingsView {
        &self.settings_view
    }

    /// Get current settings
    pub async fn get_settings(&self) -> MediaSettings {
        self.settings.read().await.clone()
    }

    /// Update settings
    pub async fn update_settings(&self, settings: MediaSettings) {
        let mut current = self.settings.write().await;
        *current = settings;
    }

    // Convenience methods

    /// Quick upload a file
    pub async fn upload_file(
        &self,
        data: Vec<u8>,
        filename: &str,
    ) -> Result<crate::models::MediaItem, String> {
        let options = crate::models::UploadOptions::default();
        self.media_service.upload(data, filename, options, None)
            .await
            .map_err(|e| e.to_string())
    }

    /// Get media by ID
    pub async fn get_media(&self, id: uuid::Uuid) -> Option<crate::models::MediaItem> {
        self.media_service.get(id).await
    }

    /// Search media
    pub async fn search_media(&self, query: &str, limit: usize) -> Vec<crate::models::MediaItem> {
        self.media_service.search(query, limit).await
    }

    /// Create a folder
    pub async fn create_folder(
        &self,
        name: &str,
        parent_id: Option<uuid::Uuid>,
    ) -> Result<crate::models::MediaFolder, String> {
        self.folder_service.create(name, parent_id, None)
            .await
            .map_err(|e| e.to_string())
    }

    /// Get folder tree
    pub async fn get_folder_tree(&self) -> Vec<crate::models::FolderTreeNode> {
        self.folder_service.get_tree().await
    }

    /// Get media statistics
    pub async fn get_stats(&self) -> crate::services::media::MediaStats {
        self.media_service.get_stats().await
    }

    /// Optimize an image
    pub async fn optimize_image(
        &self,
        data: &[u8],
    ) -> Result<crate::services::optimizer::OptimizedImage, String> {
        self.optimizer_service.optimize_image(data, None)
            .await
            .map_err(|e| e.to_string())
    }

    /// Generate thumbnails for an image
    pub async fn generate_thumbnails(
        &self,
        data: &[u8],
    ) -> Result<Vec<crate::models::Thumbnail>, String> {
        let settings = self.settings.read().await;
        let sizes = settings.get_enabled_sizes();

        let mut thumbnails = Vec::new();
        for size in sizes {
            if let Ok(thumb_data) = self.image_service.resize(
                data,
                size.width,
                size.height,
                size.mode,
            ) {
                thumbnails.push(crate::models::Thumbnail {
                    size_name: size.name.clone(),
                    url: String::new(), // Would be set when saved
                    path: String::new(),
                    width: size.width,
                    height: size.height,
                });
            }
        }

        Ok(thumbnails)
    }

    /// Cleanup expired chunked uploads
    pub async fn cleanup_expired_uploads(&self) -> usize {
        self.upload_service.cleanup_expired().await
    }

    /// Get allowed file types for upload
    pub fn get_allowed_types(&self) -> Vec<String> {
        self.upload_service.get_allowed_types()
    }

    /// Get maximum file size
    pub fn get_max_file_size(&self) -> u64 {
        self.upload_service.get_max_file_size()
    }

    // CLI commands for maintenance

    /// Run storage cleanup
    pub async fn cleanup_storage(&self) -> Result<CleanupResult, String> {
        // Would implement orphan file cleanup, etc.
        Ok(CleanupResult {
            files_removed: 0,
            bytes_freed: 0,
            errors: vec![],
        })
    }

    /// Regenerate all thumbnails
    pub async fn regenerate_thumbnails(&self) -> Result<RegenerationResult, String> {
        // Would implement thumbnail regeneration
        Ok(RegenerationResult {
            processed: 0,
            skipped: 0,
            errors: vec![],
        })
    }

    /// Rebuild media index
    pub async fn rebuild_index(&self) -> Result<(), String> {
        // Would scan storage and rebuild database
        Ok(())
    }
}

impl Default for RustMediaPlugin {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of storage cleanup
#[derive(Debug)]
pub struct CleanupResult {
    pub files_removed: usize,
    pub bytes_freed: u64,
    pub errors: Vec<String>,
}

/// Result of thumbnail regeneration
#[derive(Debug)]
pub struct RegenerationResult {
    pub processed: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

/// Plugin metadata for registration
pub fn plugin_info() -> PluginInfo {
    PluginInfo {
        name: "RustMedia",
        version: env!("CARGO_PKG_VERSION"),
        description: "Complete media management for RustPress",
        author: "RustPress Team",
        homepage: "https://rustpress.dev/plugins/rustmedia",
        license: "MIT",
        dependencies: vec![],
        hooks: vec![
            "media.upload",
            "media.delete",
            "media.optimize",
            "folder.create",
            "folder.delete",
        ],
        routes: vec![
            "/admin/media",
            "/admin/media/library",
            "/admin/media/upload",
            "/admin/media/folders",
            "/admin/media/settings",
            "/api/media",
            "/api/media/folders",
            "/api/media/upload",
        ],
    }
}

/// Plugin information
#[derive(Debug)]
pub struct PluginInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub author: &'static str,
    pub homepage: &'static str,
    pub license: &'static str,
    pub dependencies: Vec<&'static str>,
    pub hooks: Vec<&'static str>,
    pub routes: Vec<&'static str>,
}
