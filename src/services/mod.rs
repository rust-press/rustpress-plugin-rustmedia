//! RustMedia Services
//!
//! Core media management services.

pub mod media;
pub mod folder;
pub mod image;
pub mod storage;
pub mod optimizer;
pub mod upload;

pub use media::MediaService;
pub use folder::FolderService;
pub use image::ImageService;
pub use storage::StorageService;
pub use optimizer::OptimizerService;
pub use upload::UploadService;
