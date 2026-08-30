use crate::{serenity};
pub(crate) use crate::types::{Context, Data, Error};
use exif::{In, Reader, Tag};
use reqwest::{get};
use crate::commands::fun::{rcstatus, rolecolours, roleset};
use crate::tooling::image::{analyze, fetch};

pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        inspectimage(),
        // add more here
    ]
}

#[poise::command(slash_command)]
pub async fn inspectimage(
    ctx: Context<'_>,
    image: Option<serenity::all::Attachment>,
    url: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?; // defer to close short time window for response

    let image_url = if let Some(image) = &image {
        image.url.clone()
    } else if let Some(url) = &url {
        url.clone()
    } else { return Err("Provide image/URL for inspection".into()) };

    let bytes = fetch::from_url(&image_url).await?;

    let info = analyze(&bytes)?;

    let mut embed = serenity::CreateEmbed::default()
        .title("Image Information")
        .color(0x5865F2)
        .image(&image_url)
        .field("Image Format", format!("`{}`", info.format.extensions_str().join(", ")), true)
        .field("Size", format!("`{:.2}` MB", info.size as f64 / 1000000.0), true)
        .field("Resolution", format!("{} x {}", info.width, info.height), true)
        .field("Megapixels", format!("`{:.2} MP`", info.megapixels), true)
        .field("Aspect Ratio", format!("`{:.2}:1`", info.aspect_ratio), true)
        .field("Colour", format!("`{:?}`", info.color_type), true);

    if let Some(exif) = info.exif {  // i need to fix this lmao its gross
        let mut exif_text = String::new();

        if let Some(make) = exif.make {
            exif_text.push_str(&format!("**Camera:** {}\n", make));
        }
        if let Some(model) = exif.model {
            exif_text.push_str(&format!("**Model:** {}\n", model));
        }
        if let Some(lens) = exif.lens {
            exif_text.push_str(&format!("**Lens:** {}\n", lens));
        }
        if let Some(date) = exif.date_time {
            exif_text.push_str(&format!("**Date:** {}\n", date));
        }
        if let Some(iso) = exif.iso {
            exif_text.push_str(&format!("**ISO:** {}\n", iso));
        }
        if let Some(exposure) = exif.exposure_time {
            exif_text.push_str(&format!("**Exposure:** {}\n", exposure));
        }
        if let Some(aperture) = exif.f_number {
            exif_text.push_str(&format!("**Aperture:** {}\n", aperture));
        }
        if let Some(focal_length) = exif.focal_length {
            exif_text.push_str(&format!("**Focal length:** {}\n", focal_length));
        }
        if let Some(software) = exif.software {
            exif_text.push_str(&format!("**Software:** {}\n", software));
        }
        if !exif_text.is_empty() {
            embed = embed.field("EXIF", exif_text, false);
        }
    } else {
        embed = embed.field("EXIF", "No EXIF metadata found.", false);
    }

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}





