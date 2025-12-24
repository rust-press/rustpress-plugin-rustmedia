//! Storage Service
//!
//! File storage operations.

use std::path::{Path, PathBuf};
use tokio::fs;
use chrono::Utc;
use sha2::{Sha256, Digest};

/// Storage error
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("File not found: {0}")]
    NotFound(String),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("File too large: {0} bytes")]
    FileTooLarge(u64),
    #[error("Invalid file type: {0}")]
    InvalidType(String),
}

/// Storage service for file operations
pub struct StorageService {
    /// Base uploads directory
    uploads_dir: PathBuf,
    /// Base URL for uploads
    base_url: String,
    /// Maximum file size in bytes
    max_file_size: u64,
    /// Allowed MIME types (empty = all)
    allowed_types: Vec<String>,
    /// Organize by date
    organize_by_date: bool,
}

impl StorageService {
    /// Create a new storage service
    pub fn new(uploads_dir: PathBuf, base_url: impl Into<String>) -> Self {
        Self {
            uploads_dir,
            base_url: base_url.into(),
            max_file_size: 50 * 1024 * 1024, // 50MB default
            allowed_types: Vec::new(),
            organize_by_date: true,
        }
    }

    /// Initialize storage (create directories)
    pub async fn init(&self) -> Result<(), StorageError> {
        fs::create_dir_all(&self.uploads_dir).await?;
        Ok(())
    }

    /// Set maximum file size
    pub fn set_max_size(&mut self, size: u64) {
        self.max_file_size = size;
    }

    /// Set allowed MIME types
    pub fn set_allowed_types(&mut self, types: Vec<String>) {
        self.allowed_types = types;
    }

    /// Store a file
    pub async fn store(
        &self,
        data: &[u8],
        filename: &str,
        mime_type: &str,
    ) -> Result<StoredFile, StorageError> {
        // Check file size
        let size = data.len() as u64;
        if size > self.max_file_size {
            return Err(StorageError::FileTooLarge(size));
        }

        // Check MIME type
        if !self.allowed_types.is_empty() && !self.allowed_types.contains(&mime_type.to_string()) {
            return Err(StorageError::InvalidType(mime_type.to_string()));
        }

        // Calculate content hash
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hex::encode(hasher.finalize());

        // Generate path
        let relative_path = self.generate_path(filename);
        let full_path = self.uploads_dir.join(&relative_path);

        // Create directory if needed
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Write file
        fs::write(&full_path, data).await?;

        // Generate URL
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), relative_path);

        Ok(StoredFile {
            path: relative_path,
            url,
            size,
            hash,
        })
    }

    /// Store file from path (move or copy)
    pub async fn store_from_path(
        &self,
        source: &Path,
        filename: &str,
        copy: bool,
    ) -> Result<StoredFile, StorageError> {
        let data = fs::read(source).await?;
        let mime_type = mime_guess::from_path(source)
            .first_or_octet_stream()
            .to_string();

        let result = self.store(&data, filename, &mime_type).await?;

        if !copy {
            let _ = fs::remove_file(source).await;
        }

        Ok(result)
    }

    /// Read file contents
    pub async fn read(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        let full_path = self.uploads_dir.join(path);

        if !full_path.exists() {
            return Err(StorageError::NotFound(path.to_string()));
        }

        Ok(fs::read(&full_path).await?)
    }

    /// Delete a file
    pub async fn delete(&self, path: &str) -> Result<(), StorageError> {
        let full_path = self.uploads_dir.join(path);

        if full_path.exists() {
            fs::remove_file(&full_path).await?;
        }

        Ok(())
    }

    /// Check if file exists
    pub async fn exists(&self, path: &str) -> bool {
        let full_path = self.uploads_dir.join(path);
        full_path.exists()
    }

    /// Get file size
    pub async fn size(&self, path: &str) -> Result<u64, StorageError> {
        let full_path = self.uploads_dir.join(path);
        let metadata = fs::metadata(&full_path).await?;
        Ok(metadata.len())
    }

    /// Move file to new location
    pub async fn move_file(&self, from: &str, to: &str) -> Result<(), StorageError> {
        let from_path = self.uploads_dir.join(from);
        let to_path = self.uploads_dir.join(to);

        if let Some(parent) = to_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::rename(&from_path, &to_path).await?;
        Ok(())
    }

    /// Copy file
    pub async fn copy_file(&self, from: &str, to: &str) -> Result<(), StorageError> {
        let from_path = self.uploads_dir.join(from);
        let to_path = self.uploads_dir.join(to);

        if let Some(parent) = to_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::copy(&from_path, &to_path).await?;
        Ok(())
    }

    /// Generate unique filename
    pub fn generate_unique_filename(&self, original: &str) -> String {
        let timestamp = Utc::now().format("%Y%m%d%H%M%S");
        let random: u32 = rand_simple();

        let ext = Path::new(original)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let stem = Path::new(original)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file");

        let sanitized = crate::models::sanitize_filename(stem);

        if ext.is_empty() {
            format!("{}-{}-{:08x}", sanitized, timestamp, random)
        } else {
            format!("{}-{}-{:08x}.{}", sanitized, timestamp, random, ext)
        }
    }

    /// Generate path based on organization settings
    fn generate_path(&self, filename: &str) -> String {
        let unique_name = self.generate_unique_filename(filename);

        if self.organize_by_date {
            let now = Utc::now();
            format!("{}/{}/{}", now.format("%Y"), now.format("%m"), unique_name)
        } else {
            unique_name
        }
    }

    /// Get full filesystem path
    pub fn full_path(&self, relative: &str) -> PathBuf {
        self.uploads_dir.join(relative)
    }

    /// Get URL for a path
    pub fn url_for(&self, path: &str) -> String {
        format!("{}/{}", self.base_url.trim_end_matches('/'), path)
    }

    /// Get uploads directory
    pub fn uploads_dir(&self) -> &Path {
        &self.uploads_dir
    }

    /// Calculate directory size
    pub async fn directory_size(&self, path: Option<&str>) -> Result<u64, StorageError> {
        let target = match path {
            Some(p) => self.uploads_dir.join(p),
            None => self.uploads_dir.clone(),
        };

        let mut total = 0u64;

        let mut entries = fs::read_dir(&target).await?;
        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            if metadata.is_file() {
                total += metadata.len();
            } else if metadata.is_dir() {
                if let Ok(sub_size) = Box::pin(self.directory_size(
                    Some(entry.path().strip_prefix(&self.uploads_dir).unwrap().to_str().unwrap())
                )).await {
                    total += sub_size;
                }
            }
        }

        Ok(total)
    }

    /// List files in directory
    pub async fn list_files(&self, path: Option<&str>) -> Result<Vec<FileInfo>, StorageError> {
        let target = match path {
            Some(p) => self.uploads_dir.join(p),
            None => self.uploads_dir.clone(),
        };

        let mut files = Vec::new();

        let mut entries = fs::read_dir(&target).await?;
        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            let name = entry.file_name().to_string_lossy().to_string();
            let relative_path = entry.path()
                .strip_prefix(&self.uploads_dir)
                .unwrap()
                .to_string_lossy()
                .to_string();

            files.push(FileInfo {
                name,
                path: relative_path,
                size: metadata.len(),
                is_directory: metadata.is_dir(),
                modified: metadata.modified().ok(),
            });
        }

        Ok(files)
    }
}

/// Stored file information
#[derive(Debug, Clone)]
pub struct StoredFile {
    /// Relative path
    pub path: String,
    /// Public URL
    pub url: String,
    /// File size in bytes
    pub size: u64,
    /// Content hash
    pub hash: String,
}

/// File info for listing
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_directory: bool,
    pub modified: Option<std::time::SystemTime>,
}

/// Simple random number (no external crate)
fn rand_simple() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    ((duration.as_nanos() % u32::MAX as u128) as u32).wrapping_mul(1103515245).wrapping_add(12345)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_storage_store_and_read() {
        let dir = tempdir().unwrap();
        let storage = StorageService::new(dir.path().to_path_buf(), "/uploads");

        storage.init().await.unwrap();

        let data = b"Hello, World!";
        let result = storage.store(data, "test.txt", "text/plain").await.unwrap();

        assert!(!result.path.is_empty());
        assert!(result.url.starts_with("/uploads"));

        let read_data = storage.read(&result.path).await.unwrap();
        assert_eq!(read_data, data);
    }

    #[tokio::test]
    async fn test_storage_delete() {
        let dir = tempdir().unwrap();
        let storage = StorageService::new(dir.path().to_path_buf(), "/uploads");

        storage.init().await.unwrap();

        let data = b"Test data";
        let result = storage.store(data, "delete-me.txt", "text/plain").await.unwrap();

        assert!(storage.exists(&result.path).await);

        storage.delete(&result.path).await.unwrap();

        assert!(!storage.exists(&result.path).await);
    }
}
