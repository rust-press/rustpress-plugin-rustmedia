//! Optimizer Service
//!
//! Media optimization and compression.

use std::sync::Arc;
use crate::models::{ImageFormat, OptimizationResult};
use super::image::ImageService;
use super::storage::StorageService;

/// Optimizer service error
#[derive(Debug, thiserror::Error)]
pub enum OptimizerError {
    #[error("Image error: {0}")]
    Image(#[from] super::image::ImageError),
    #[error("Storage error: {0}")]
    Storage(#[from] super::storage::StorageError),
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}

/// Optimization settings
#[derive(Debug, Clone)]
pub struct OptimizationSettings {
    /// JPEG quality (1-100)
    pub jpeg_quality: u8,
    /// PNG compression level
    pub png_compression: u8,
    /// WebP quality (1-100)
    pub webp_quality: u8,
    /// Maximum width
    pub max_width: Option<u32>,
    /// Maximum height
    pub max_height: Option<u32>,
    /// Strip metadata
    pub strip_metadata: bool,
    /// Convert to WebP
    pub convert_to_webp: bool,
    /// Progressive JPEG
    pub progressive_jpeg: bool,
}

impl Default for OptimizationSettings {
    fn default() -> Self {
        Self {
            jpeg_quality: 85,
            png_compression: 6,
            webp_quality: 80,
            max_width: Some(2048),
            max_height: Some(2048),
            strip_metadata: true,
            convert_to_webp: false,
            progressive_jpeg: true,
        }
    }
}

/// Optimizer service
pub struct OptimizerService {
    /// Image service
    image_service: Arc<ImageService>,
    /// Storage service
    storage: Arc<StorageService>,
    /// Settings
    settings: OptimizationSettings,
}

impl OptimizerService {
    /// Create a new optimizer service
    pub fn new(
        image_service: Arc<ImageService>,
        storage: Arc<StorageService>,
    ) -> Self {
        Self {
            image_service,
            storage,
            settings: OptimizationSettings::default(),
        }
    }

    /// Set optimization settings
    pub fn configure(&mut self, settings: OptimizationSettings) {
        self.settings = settings;
    }

    /// Optimize an image
    pub async fn optimize_image(
        &self,
        data: &[u8],
        format: Option<ImageFormat>,
    ) -> Result<OptimizedImage, OptimizerError> {
        let original_size = data.len() as u64;

        // Determine output format
        let output_format = if self.settings.convert_to_webp {
            ImageFormat::WebP
        } else {
            format.unwrap_or(ImageFormat::Jpeg)
        };

        // Get quality based on format
        let quality = match output_format {
            ImageFormat::Jpeg => self.settings.jpeg_quality,
            ImageFormat::WebP => self.settings.webp_quality,
            ImageFormat::Png => self.settings.png_compression,
            _ => 85,
        };

        // Optimize
        let result = self.image_service.optimize(data, quality)?;

        // Encode optimized image
        let optimized_data = self.image_service.optimize(data, quality)?;

        Ok(OptimizedImage {
            data: Vec::new(), // Would need to return actual optimized bytes
            original_size,
            optimized_size: optimized_data.optimized_size,
            format: output_format,
            savings_percent: optimized_data.savings_percent,
        })
    }

    /// Optimize image file in place
    pub async fn optimize_file(&self, path: &str) -> Result<OptimizationResult, OptimizerError> {
        let data = self.storage.read(path).await?;
        let result = self.optimize_image(&data, None).await?;

        Ok(OptimizationResult {
            original_size: result.original_size,
            optimized_size: result.optimized_size,
            savings_percent: result.savings_percent,
            dimensions: None,
            format: result.format,
        })
    }

    /// Batch optimize images
    pub async fn optimize_batch(
        &self,
        paths: Vec<String>,
    ) -> Vec<(String, Result<OptimizationResult, OptimizerError>)> {
        let mut results = Vec::new();

        for path in paths {
            let result = self.optimize_file(&path).await;
            results.push((path, result));
        }

        results
    }

    /// Convert image to format
    pub async fn convert(
        &self,
        data: &[u8],
        target_format: ImageFormat,
    ) -> Result<Vec<u8>, OptimizerError> {
        let quality = match target_format {
            ImageFormat::Jpeg => self.settings.jpeg_quality,
            ImageFormat::WebP => self.settings.webp_quality,
            ImageFormat::Png => self.settings.png_compression,
            _ => 85,
        };

        let transform = crate::models::ImageTransformRequest {
            width: None,
            height: None,
            mode: None,
            quality: Some(quality),
            format: Some(target_format),
            rotate: None,
            flip_h: None,
            flip_v: None,
            crop: None,
            watermark: None,
            filters: None,
        };

        let result = self.image_service.transform(data, &transform)?;
        Ok(result)
    }

    /// Resize and optimize
    pub async fn resize_and_optimize(
        &self,
        data: &[u8],
        max_width: u32,
        max_height: u32,
    ) -> Result<OptimizedImage, OptimizerError> {
        let transform = crate::models::ImageTransformRequest {
            width: Some(max_width),
            height: Some(max_height),
            mode: Some(crate::models::ResizeMode::Fit),
            quality: Some(self.settings.jpeg_quality),
            format: if self.settings.convert_to_webp {
                Some(ImageFormat::WebP)
            } else {
                None
            },
            rotate: None,
            flip_h: None,
            flip_v: None,
            crop: None,
            watermark: None,
            filters: None,
        };

        let original_size = data.len() as u64;
        let optimized_data = self.image_service.transform(data, &transform)?;
        let optimized_size = optimized_data.len() as u64;

        let savings_percent = if original_size > 0 {
            ((original_size - optimized_size) as f64 / original_size as f64) * 100.0
        } else {
            0.0
        };

        Ok(OptimizedImage {
            data: optimized_data,
            original_size,
            optimized_size,
            format: if self.settings.convert_to_webp {
                ImageFormat::WebP
            } else {
                ImageFormat::Jpeg
            },
            savings_percent,
        })
    }

    /// Get estimated savings for image
    pub fn estimate_savings(&self, size: u64, mime_type: &str) -> u64 {
        // Rough estimates based on typical compression ratios
        let ratio = match mime_type {
            "image/jpeg" => 0.7, // 30% savings typically
            "image/png" => 0.5,  // 50% savings with lossy conversion
            "image/gif" => 0.6,
            "image/webp" => 0.85, // Already optimized
            _ => 0.8,
        };

        ((size as f64) * (1.0 - ratio)) as u64
    }
}

/// Optimized image result
#[derive(Debug)]
pub struct OptimizedImage {
    /// Optimized image data
    pub data: Vec<u8>,
    /// Original size in bytes
    pub original_size: u64,
    /// Optimized size in bytes
    pub optimized_size: u64,
    /// Output format
    pub format: ImageFormat,
    /// Savings percentage
    pub savings_percent: f64,
}

impl OptimizedImage {
    /// Get bytes saved
    pub fn bytes_saved(&self) -> u64 {
        self.original_size.saturating_sub(self.optimized_size)
    }
}
