use exif::{In, Reader, Tag};
use std::io::Cursor;

pub struct ExifInfo {
    pub field_count: usize,

    pub make: Option<String>,
    pub model: Option<String>,
    pub lens: Option<String>,

    pub date_time_original: Option<String>,
    pub date_time_digitized: Option<String>,

    pub iso: Option<String>,
    pub exposure_time: Option<String>,
    pub f_number: Option<String>,
    pub focal_length: Option<String>,
    pub orientation: Option<String>,

    pub software: Option<String>,

    pub gps: Option<GpsCoordinates>,
}

#[derive(Debug)]
pub struct GpsCoordinates {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
}

impl ExifInfo {
    pub fn to_text(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!(
            "**Fields:** `{}`",
            self.field_count
        ));

        if let Some(value) = &self.make {
            lines.push(format!("**Camera:** {}", value));
        }

        if let Some(value) = &self.model {
            lines.push(format!("**Model:** {}", value));
        }

        if let Some(value) = &self.lens {
            lines.push(format!("**Lens:** {}", value));
        }

        if let Some(value) = &self.date_time_original {
            lines.push(format!("**Date taken:** {}", value));
        }

        if let Some(value) = &self.iso {
            lines.push(format!("**ISO:** {}", value));
        }

        if let Some(value) = &self.exposure_time {
            lines.push(format!("**Exposure:** {}", value));
        }

        if let Some(value) = &self.f_number {
            lines.push(format!("**Aperture:** {}", value));
        }

        if let Some(value) = &self.focal_length {
            lines.push(format!("**Focal length:** {}", value));
        }

        if let Some(value) = &self.orientation {
            lines.push(format!("**Orientation:** {}", value));
        }

        if let Some(value) = &self.software {
            lines.push(format!("**Software:** {}", value));
        }

        if let Some(gps) = &self.gps {
            lines.push(format!(
                "**GPS:** `{:.6}, {:.6}`",
                gps.latitude,
                gps.longitude
            ));

            if let Some(altitude) = gps.altitude {
                lines.push(format!(
                    "**GPS altitude:** `{:.1} m`",
                    altitude
                ));
            }
        }

        lines.join("\n")
    }
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

    let orientation = exif
        .get_field(Tag::Orientation, In::PRIMARY)
        .and_then(|field| orientation_name(&field.value));

    Some(ExifInfo {
        field_count: exif.fields().len(),

        make: get(Tag::Make),
        model: get(Tag::Model),
        lens: get(Tag::LensModel),

        date_time_original: get(Tag::DateTimeOriginal),
        date_time_digitized: get(Tag::DateTimeDigitized),

        iso: get(Tag::ISOSpeed),
        exposure_time: get(Tag::ExposureTime),
        f_number: get(Tag::FNumber),
        focal_length: get(Tag::FocalLength),
        orientation,

        software: get(Tag::Software),

        gps: extract_gps(&exif),
    })
}

fn extract_gps(exif: &exif::Exif) -> Option<GpsCoordinates> {
    let latitude = exif.get_field(
        exif::Tag::GPSLatitude,
        exif::In::PRIMARY,
    )?;

    let latitude_ref = exif.get_field(
        exif::Tag::GPSLatitudeRef,
        exif::In::PRIMARY,
    )?;

    let longitude = exif.get_field(
        exif::Tag::GPSLongitude,
        exif::In::PRIMARY,
    )?;

    let longitude_ref = exif.get_field(
        exif::Tag::GPSLongitudeRef,
        exif::In::PRIMARY,
    )?;

    let latitude_ref =
        latitude_ref.display_value().to_string();

    let longitude_ref =
        longitude_ref.display_value().to_string();

    let latitude = gps_to_decimal(
        &latitude.value,
        latitude_ref.trim_matches('"'),
    )?;

    let longitude = gps_to_decimal(
        &longitude.value,
        longitude_ref.trim_matches('"'),
    )?;

    let altitude = exif
        .get_field(
            exif::Tag::GPSAltitude,
            exif::In::PRIMARY,
        )
        .and_then(|field| match &field.value {
            exif::Value::Rational(values) => {
                values.first().map(|v| v.to_f64())
            }
            _ => None,
        });

    Some(GpsCoordinates {
        latitude,
        longitude,
        altitude,
    })
}

fn gps_to_decimal(
    value: &exif::Value,
    reference: &str,
) -> Option<f64> {
    let components = match value {
        exif::Value::Rational(values) if values.len() >= 3 => values,
        _ => return None,
    };

    let degrees = components[0].to_f64();
    let minutes = components[1].to_f64();
    let seconds = components[2].to_f64();

    let mut decimal =
        degrees + minutes / 60.0 + seconds / 3600.0;

    if reference == "S" || reference == "W" {
        decimal = -decimal;
    }

    Some(decimal)
}

fn orientation_name(value: &exif::Value) -> Option<String> {
    let number = match value {
        exif::Value::Short(values) => *values.first()?,
        _ => return None,
    };

    Some(match number {
        1 => "Normal",
        2 => "Mirrored horizontally",
        3 => "Rotated 180°",
        4 => "Mirrored vertically",
        5 => "Mirrored horizontally, rotated 270° CW",
        6 => "Rotated 90° CW",
        7 => "Mirrored horizontally, rotated 90° CW",
        8 => "Rotated 270° CW",
        _ => "Unknown",
    }.to_string())
}