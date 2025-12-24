//! Image Service
//!
//! Image processing and manipulation.

use std::path::Path;
use image::{DynamicImage, ImageFormat as ImgFormat, imageops::FilterType};

use crate::models::{
    ImageSize, ImageFormat, ImageDimensions, ResizeMode,
    CropParams, ImageTransformRequest, OptimizationResult,
    Thumbnail, default_image_sizes,
};
use super::storage::{StorageService, StorageError};

/// Image processing error
#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("Image processing error: {0}")]
    Processing(String),
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
}

/// Image service for processing
pub struct ImageService {
    /// Storage service
    storage: std::sync::Arc<StorageService>,
    /// Configured image sizes
    sizes: Vec<ImageSize>,
    /// Default quality
    default_quality: u8,
    /// Convert to WebP
    convert_to_webp: bool,
    /// Strip metadata
    strip_metadata: bool,
}

impl ImageService {
    /// Create a new image service
    pub fn new(storage: std::sync::Arc<StorageService>) -> Self {
        Self {
            storage,
            sizes: default_image_sizes(),
            default_quality: 85,
            convert_to_webp: false,
            strip_metadata: true,
        }
    }

    /// Set image sizes
    pub fn set_sizes(&mut self, sizes: Vec<ImageSize>) {
        self.sizes = sizes;
    }

    /// Set default quality
    pub fn set_quality(&mut self, quality: u8) {
        self.default_quality = quality.clamp(1, 100);
    }

    /// Enable WebP conversion
    pub fn enable_webp(&mut self, enabled: bool) {
        self.convert_to_webp = enabled;
    }

    /// Get image dimensions
    pub fn get_dimensions(&self, data: &[u8]) -> Result<ImageDimensions, ImageError> {
        let img = image::load_from_memory(data)?;
        Ok(ImageDimensions::new(img.width(), img.height()))
    }

    /// Resize image
    pub fn resize(
        &self,
        data: &[u8],
        size: &ImageSize,
    ) -> Result<Vec<u8>, ImageError> {
        let img = image::load_from_memory(data)?;
        let (width, height) = size.calculate_dimensions(img.width(), img.height());

        let resized = match size.mode {
            ResizeMode::Exact => img.resize_exact(width, height, FilterType::Lanczos3),
            ResizeMode::Fit => img.resize(width, height, FilterType::Lanczos3),
            ResizeMode::Fill | ResizeMode::Cover => {
                img.resize_to_fill(width, height, FilterType::Lanczos3)
            }
        };

        self.encode_image(&resized, ImageFormat::Jpeg, size.quality)
    }

    /// Generate all thumbnails for an image
    pub async fn generate_thumbnails(
        &self,
        data: &[u8],
        original_path: &str,
    ) -> Result<Vec<Thumbnail>, ImageError> {
        let img = image::load_from_memory(data)?;
        let mut thumbnails = Vec::new();

        for size in &self.sizes {
            if !size.enabled {
                continue;
            }

            // Skip if image is smaller than target
            if img.width() <= size.width && img.height() <= size.height {
                continue;
            }

            let (width, height) = size.calculate_dimensions(img.width(), img.height());

            let resized = match size.mode {
                ResizeMode::Exact => img.resize_exact(width, height, FilterType::Lanczos3),
                ResizeMode::Fit => img.resize(width, height, FilterType::Lanczos3),
                ResizeMode::Fill | ResizeMode::Cover => {
                    img.resize_to_fill(width, height, FilterType::Lanczos3)
                }
            };

            // Determine output format
            let format = if self.convert_to_webp {
                ImageFormat::WebP
            } else {
                ImageFormat::Jpeg
            };

            let thumb_data = self.encode_image(&resized, format, size.quality)?;

            // Generate thumbnail path
            let thumb_path = self.generate_thumbnail_path(original_path, &size.name, format);

            // Store thumbnail
            let stored = self.storage.store(
                &thumb_data,
                &thumb_path,
                format.mime_type(),
            ).await?;

            thumbnails.push(Thumbnail {
                size_name: size.name.clone(),
                width: resized.width(),
                height: resized.height(),
                path: stored.path,
                url: stored.url,
                size: stored.size,
            });
        }

        Ok(thumbnails)
    }

    /// Crop image
    pub fn crop(&self, data: &[u8], params: &CropParams) -> Result<Vec<u8>, ImageError> {
        let img = image::load_from_memory(data)?;

        let cropped = img.crop_imm(params.x, params.y, params.width, params.height);

        self.encode_image(&cropped, ImageFormat::Jpeg, self.default_quality)
    }

    /// Rotate image
    pub fn rotate(&self, data: &[u8], degrees: i32) -> Result<Vec<u8>, ImageError> {
        let img = image::load_from_memory(data)?;

        let rotated = match degrees {
            90 | -270 => img.rotate90(),
            180 | -180 => img.rotate180(),
            270 | -90 => img.rotate270(),
            _ => img,
        };

        self.encode_image(&rotated, ImageFormat::Jpeg, self.default_quality)
    }

    /// Flip image horizontally
    pub fn flip_horizontal(&self, data: &[u8]) -> Result<Vec<u8>, ImageError> {
        let img = image::load_from_memory(data)?;
        let flipped = img.fliph();
        self.encode_image(&flipped, ImageFormat::Jpeg, self.default_quality)
    }

    /// Flip image vertically
    pub fn flip_vertical(&self, data: &[u8]) -> Result<Vec<u8>, ImageError> {
        let img = image::load_from_memory(data)?;
        let flipped = img.flipv();
        self.encode_image(&flipped, ImageFormat::Jpeg, self.default_quality)
    }

    /// Convert to grayscale
    pub fn grayscale(&self, data: &[u8]) -> Result<Vec<u8>, ImageError> {
        let img = image::load_from_memory(data)?;
        let gray = img.grayscale();
        self.encode_image(&gray, ImageFormat::Jpeg, self.default_quality)
    }

    /// Apply blur
    pub fn blur(&self, data: &[u8], sigma: f32) -> Result<Vec<u8>, ImageError> {
        let img = image::load_from_memory(data)?;
        let blurred = img.blur(sigma);
        self.encode_image(&blurred, ImageFormat::Jpeg, self.default_quality)
    }

    /// Optimize image
    pub fn optimize(&self, data: &[u8], quality: u8) -> Result<OptimizationResult, ImageError> {
        let original_size = data.len() as u64;
        let img = image::load_from_memory(data)?;

        let format = if self.convert_to_webp {
            ImageFormat::WebP
        } else {
            ImageFormat::Jpeg
        };

        let optimized = self.encode_image(&img, format, quality)?;
        let optimized_size = optimized.len() as u64;

        let mut result = OptimizationResult::new(original_size, optimized_size);
        result.dimensions = Some((img.width(), img.height()));
        result.format = format;

        Ok(result)
    }

    /// Transform image with multiple operations
    pub fn transform(&self, data: &[u8], request: &ImageTransformRequest) -> Result<Vec<u8>, ImageError> {
        let mut img = image::load_from_memory(data)?;

        // Crop first
        if let Some(ref crop) = request.crop {
            img = img.crop_imm(crop.x, crop.y, crop.width, crop.height);
        }

        // Resize
        if request.width.is_some() || request.height.is_some() {
            let width = request.width.unwrap_or(0);
            let height = request.height.unwrap_or(0);
            let mode = request.mode.unwrap_or(ResizeMode::Fit);

            img = match mode {
                ResizeMode::Exact if width > 0 && height > 0 => {
                    img.resize_exact(width, height, FilterType::Lanczos3)
                }
                ResizeMode::Fill | ResizeMode::Cover if width > 0 && height > 0 => {
                    img.resize_to_fill(width, height, FilterType::Lanczos3)
                }
                _ => {
                    if width > 0 && height > 0 {
                        img.resize(width, height, FilterType::Lanczos3)
                    } else if width > 0 {
                        let ratio = width as f64 / img.width() as f64;
                        let new_height = (img.height() as f64 * ratio) as u32;
                        img.resize(width, new_height, FilterType::Lanczos3)
                    } else if height > 0 {
                        let ratio = height as f64 / img.height() as f64;
                        let new_width = (img.width() as f64 * ratio) as u32;
                        img.resize(new_width, height, FilterType::Lanczos3)
                    } else {
                        img
                    }
                }
            };
        }

        // Rotate
        if let Some(degrees) = request.rotate {
            img = match degrees {
                90 | -270 => img.rotate90(),
                180 | -180 => img.rotate180(),
                270 | -90 => img.rotate270(),
                _ => img,
            };
        }

        // Flip
        if request.flip_h == Some(true) {
            img = img.fliph();
        }
        if request.flip_v == Some(true) {
            img = img.flipv();
        }

        // Apply filters
        if let Some(ref filters) = request.filters {
            for filter in filters {
                img = self.apply_filter(img, filter);
            }
        }

        // Encode
        let format = request.format.unwrap_or(ImageFormat::Jpeg);
        let quality = request.quality.unwrap_or(self.default_quality);

        self.encode_image(&img, format, quality)
    }

    /// Apply a filter to image
    fn apply_filter(&self, img: DynamicImage, filter: &crate::models::ImageFilter) -> DynamicImage {
        use crate::models::ImageFilter;

        match filter {
            ImageFilter::Brightness(value) => {
                img.brighten(*value)
            }
            ImageFilter::Contrast(value) => {
                img.adjust_contrast(*value as f32)
            }
            ImageFilter::Blur(sigma) => {
                img.blur(*sigma)
            }
            ImageFilter::Grayscale => {
                img.grayscale()
            }
            ImageFilter::Invert => {
                let mut inverted = img;
                inverted.invert();
                inverted
            }
            // Other filters would need custom implementation
            _ => img,
        }
    }

    /// Encode image to bytes
    fn encode_image(
        &self,
        img: &DynamicImage,
        format: ImageFormat,
        quality: u8,
    ) -> Result<Vec<u8>, ImageError> {
        let mut buffer = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buffer);

        match format {
            ImageFormat::Jpeg => {
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, quality);
                img.write_with_encoder(encoder)?;
            }
            ImageFormat::Png => {
                img.write_to(&mut cursor, ImgFormat::Png)?;
            }
            ImageFormat::WebP => {
                img.write_to(&mut cursor, ImgFormat::WebP)?;
            }
            ImageFormat::Gif => {
                img.write_to(&mut cursor, ImgFormat::Gif)?;
            }
            _ => {
                return Err(ImageError::UnsupportedFormat(format!("{:?}", format)));
            }
        }

        Ok(buffer)
    }

    /// Generate thumbnail path
    fn generate_thumbnail_path(&self, original: &str, size_name: &str, format: ImageFormat) -> String {
        let path = Path::new(original);
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
        let parent = path.parent().and_then(|p| p.to_str()).unwrap_or("");

        if parent.is_empty() {
            format!("{}-{}.{}", stem, size_name, format.extension())
        } else {
            format!("{}/{}-{}.{}", parent, stem, size_name, format.extension())
        }
    }

    /// Check if file is an image
    pub fn is_image(mime_type: &str) -> bool {
        mime_type.starts_with("image/")
    }

    /// Get supported image extensions
    pub fn supported_extensions() -> Vec<&'static str> {
        vec!["jpg", "jpeg", "png", "gif", "webp", "bmp", "ico", "tiff", "tif"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_image() {
        assert!(ImageService::is_image("image/jpeg"));
        assert!(ImageService::is_image("image/png"));
        assert!(!ImageService::is_image("text/plain"));
        assert!(!ImageService::is_image("application/pdf"));
    }
}
