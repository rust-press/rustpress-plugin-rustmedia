//! Image Models
//!
//! Image processing and manipulation structures.

use serde::{Deserialize, Serialize};

/// Image size preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSize {
    /// Size name (e.g., "thumbnail", "medium", "large")
    pub name: String,
    /// Width in pixels (0 = auto)
    pub width: u32,
    /// Height in pixels (0 = auto)
    pub height: u32,
    /// Resize mode
    pub mode: ResizeMode,
    /// Quality (1-100)
    pub quality: u8,
    /// Whether this size is enabled
    pub enabled: bool,
}

impl ImageSize {
    pub fn new(name: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            name: name.into(),
            width,
            height,
            mode: ResizeMode::Fit,
            quality: 85,
            enabled: true,
        }
    }

    /// Get target dimensions maintaining aspect ratio
    pub fn calculate_dimensions(&self, original_width: u32, original_height: u32) -> (u32, u32) {
        match self.mode {
            ResizeMode::Exact => (self.width, self.height),
            ResizeMode::Fit => {
                if self.width == 0 && self.height == 0 {
                    return (original_width, original_height);
                }

                let ratio = original_width as f64 / original_height as f64;

                if self.width == 0 {
                    let new_width = (self.height as f64 * ratio).round() as u32;
                    (new_width, self.height)
                } else if self.height == 0 {
                    let new_height = (self.width as f64 / ratio).round() as u32;
                    (self.width, new_height)
                } else {
                    let width_ratio = self.width as f64 / original_width as f64;
                    let height_ratio = self.height as f64 / original_height as f64;
                    let ratio = width_ratio.min(height_ratio);

                    let new_width = (original_width as f64 * ratio).round() as u32;
                    let new_height = (original_height as f64 * ratio).round() as u32;
                    (new_width, new_height)
                }
            }
            ResizeMode::Fill => (self.width, self.height),
            ResizeMode::Cover => (self.width, self.height),
        }
    }
}

/// Image resize mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResizeMode {
    /// Exact dimensions (may distort)
    Exact,
    /// Fit within dimensions (maintain aspect ratio)
    Fit,
    /// Fill dimensions with cropping
    Fill,
    /// Cover dimensions (crop to fit)
    Cover,
}

impl Default for ResizeMode {
    fn default() -> Self {
        Self::Fit
    }
}

/// Default image sizes
pub fn default_image_sizes() -> Vec<ImageSize> {
    vec![
        ImageSize {
            name: "thumbnail".to_string(),
            width: 150,
            height: 150,
            mode: ResizeMode::Fill,
            quality: 80,
            enabled: true,
        },
        ImageSize {
            name: "small".to_string(),
            width: 300,
            height: 0,
            mode: ResizeMode::Fit,
            quality: 85,
            enabled: true,
        },
        ImageSize {
            name: "medium".to_string(),
            width: 600,
            height: 0,
            mode: ResizeMode::Fit,
            quality: 85,
            enabled: true,
        },
        ImageSize {
            name: "large".to_string(),
            width: 1200,
            height: 0,
            mode: ResizeMode::Fit,
            quality: 85,
            enabled: true,
        },
        ImageSize {
            name: "full".to_string(),
            width: 2048,
            height: 0,
            mode: ResizeMode::Fit,
            quality: 90,
            enabled: true,
        },
    ]
}

/// Image transformation request
#[derive(Debug, Clone, Deserialize)]
pub struct ImageTransformRequest {
    /// Resize width (0 = auto)
    pub width: Option<u32>,
    /// Resize height (0 = auto)
    pub height: Option<u32>,
    /// Resize mode
    pub mode: Option<ResizeMode>,
    /// Output quality (1-100)
    pub quality: Option<u8>,
    /// Output format
    pub format: Option<ImageFormat>,
    /// Rotation in degrees (90, 180, 270)
    pub rotate: Option<i32>,
    /// Flip horizontal
    pub flip_h: Option<bool>,
    /// Flip vertical
    pub flip_v: Option<bool>,
    /// Crop parameters
    pub crop: Option<CropParams>,
    /// Watermark
    pub watermark: Option<WatermarkParams>,
    /// Filters
    pub filters: Option<Vec<ImageFilter>>,
}

/// Image format
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Gif,
    WebP,
    Avif,
}

impl ImageFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "png" => Some(Self::Png),
            "gif" => Some(Self::Gif),
            "webp" => Some(Self::WebP),
            "avif" => Some(Self::Avif),
            _ => None,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Gif => "gif",
            Self::WebP => "webp",
            Self::Avif => "avif",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Gif => "image/gif",
            Self::WebP => "image/webp",
            Self::Avif => "image/avif",
        }
    }
}

impl Default for ImageFormat {
    fn default() -> Self {
        Self::Jpeg
    }
}

/// Crop parameters
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CropParams {
    /// X offset
    pub x: u32,
    /// Y offset
    pub y: u32,
    /// Crop width
    pub width: u32,
    /// Crop height
    pub height: u32,
}

/// Watermark parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatermarkParams {
    /// Watermark image path or URL
    pub image: String,
    /// Position
    pub position: WatermarkPosition,
    /// Opacity (0.0 - 1.0)
    pub opacity: f32,
    /// Scale relative to main image
    pub scale: f32,
    /// Margin in pixels
    pub margin: u32,
}

/// Watermark position
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum WatermarkPosition {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    Tile,
}

impl Default for WatermarkPosition {
    fn default() -> Self {
        Self::BottomRight
    }
}

/// Image filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageFilter {
    /// Brightness adjustment (-100 to 100)
    Brightness(i32),
    /// Contrast adjustment (-100 to 100)
    Contrast(i32),
    /// Saturation adjustment (-100 to 100)
    Saturation(i32),
    /// Hue rotation (0 to 360)
    HueRotate(i32),
    /// Blur amount
    Blur(f32),
    /// Sharpen amount
    Sharpen(f32),
    /// Grayscale
    Grayscale,
    /// Sepia
    Sepia,
    /// Invert colors
    Invert,
}

/// Image optimization result
#[derive(Debug, Clone, Serialize)]
pub struct OptimizationResult {
    /// Original size in bytes
    pub original_size: u64,
    /// Optimized size in bytes
    pub optimized_size: u64,
    /// Size reduction percentage
    pub savings_percent: f64,
    /// New dimensions
    pub dimensions: Option<(u32, u32)>,
    /// Format used
    pub format: ImageFormat,
}

impl OptimizationResult {
    pub fn new(original_size: u64, optimized_size: u64) -> Self {
        let savings_percent = if original_size > 0 {
            ((original_size - optimized_size) as f64 / original_size as f64) * 100.0
        } else {
            0.0
        };

        Self {
            original_size,
            optimized_size,
            savings_percent,
            dimensions: None,
            format: ImageFormat::default(),
        }
    }
}

/// Focal point for smart cropping
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FocalPoint {
    /// X position (0.0 to 1.0)
    pub x: f32,
    /// Y position (0.0 to 1.0)
    pub y: f32,
}

impl Default for FocalPoint {
    fn default() -> Self {
        Self { x: 0.5, y: 0.5 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_dimensions_fit() {
        let size = ImageSize::new("test", 200, 100);
        let (w, h) = size.calculate_dimensions(1000, 500);
        assert_eq!(w, 200);
        assert_eq!(h, 100);

        // Different aspect ratio
        let (w, h) = size.calculate_dimensions(1000, 1000);
        assert_eq!(w, 100);
        assert_eq!(h, 100);
    }

    #[test]
    fn test_image_format() {
        assert_eq!(ImageFormat::from_extension("jpg"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::from_extension("PNG"), Some(ImageFormat::Png));
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
    }
}
