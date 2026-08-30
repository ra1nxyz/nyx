use exif::{In, Reader, Tag};
use std::io::Cursor;

pub struct ExifInfo {
    pub make: Option<String>,
    pub model: Option<String>,
    pub lens: Option<String>,

    pub date_time: Option<String>,

    pub iso: Option<String>,
    pub exposure_time: Option<String>,
    pub f_number: Option<String>,
    pub focal_length: Option<String>,

    pub software: Option<String>,

    pub latitude: Option<String>,
    pub longitude: Option<String>,
}

pub fn extract_exif(bytes: &[u8]) -> Option<ExifInfo> {
    let mut cursor = Cursor::new(bytes);

    let exif = Reader::new()
        .read_from_container(&mut cursor)
        .ok()?;

    let get = |tag: Tag| {
        exif.get_field(tag, In::PRIMARY)
            .map(|field| field.display_value().with_unit(&exif).to_string())
    };

    Some(ExifInfo {
        make: get(Tag::Make),
        model: get(Tag::Model),
        lens: get(Tag::LensModel),

        date_time: get(Tag::DateTimeOriginal),

        iso: get(Tag::ISOSpeed),
        exposure_time: get(Tag::ExposureTime),
        f_number: get(Tag::FNumber),
        focal_length: get(Tag::FocalLength),

        software: get(Tag::Software),

        latitude: None,
        longitude: None,
    })
}