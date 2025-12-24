//! RustMedia Admin Module

pub mod dashboard;
pub mod library;
pub mod upload;
pub mod folders;
pub mod settings;

pub use dashboard::DashboardView;
pub use library::LibraryView;
pub use upload::UploadView;
pub use folders::FoldersView;
pub use settings::SettingsView;
