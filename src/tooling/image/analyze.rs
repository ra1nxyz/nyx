use exif::Reader;
use std::io::Cursor;

use image::{ColorType, ImageFormat};

use super::metadata::{extract_exif, ExifInfo};


pub struct ImageInfo {
    pub format: ImageFormat,
    pub size: usize,

    pub width: u32,
    pub height: u32,
    pub megapixels: f64,
    pub aspect_ratio: f64,

    pub color_type: ColorType,

    pub exif: Option<ExifInfo>,
}

pub fn analyze(bytes: &[u8]) -> Result<ImageInfo, Box<dyn std::error::Error + Send + Sync>> {
    let img = image::load_from_memory(bytes)?;

    let width = img.width();
    let height = img.height();

    let megapixels = (width as f64 * height as f64) / 1_000_000.0;

    let aspect_ratio = width as f64 / height as f64;

    let format = image::guess_format(bytes)?;

    let color_type = img.color();

    let mut cursor = Cursor::new(bytes);

    let exif = extract_exif(bytes);

    Ok(ImageInfo {
        format,
        size: bytes.len(),
        width,
        height,
        megapixels,
        aspect_ratio,
        color_type,
        exif,
    })
}