use exif::Reader;
use image::ImageFormat;
use std::io::Cursor;

pub struct ImageInfo {
    pub format: ImageFormat,
    pub size: usize,
    pub width: u32,
    pub height: u32,
    pub megapixels: f64,
    pub exif: Option<String>,
}

pub fn analyze(bytes: &[u8]) -> Result<ImageInfo, Box<dyn std::error::Error + Send + Sync>> {
    let img = image::load_from_memory(bytes)?;

    let width = img.width();
    let height = img.height();

    let megapixels = (width as f64 * height as f64) / 1_000_000.0;

    let format = image::guess_format(bytes)?;

    let mut cursor = Cursor::new(bytes);

    let exif = Reader::new()
        .read_from_container(&mut cursor)
        .ok()
        .map(|exif| {
            exif.fields()
                .map(|field| {
                    format!(
                        "**{}**: {}",
                        field.tag,
                        field.display_value().with_unit(&exif)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|text| !text.is_empty());

    Ok(ImageInfo {
        format,
        size: bytes.len(),
        width,
        height,
        megapixels,
        exif,
    })
}