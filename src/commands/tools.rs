use crate::{serenity};
pub(crate) use crate::types::{Context, Data, Error};
use exif::{In, Reader, Tag};
use reqwest::{get};
use crate::commands::fun::{rcstatus, rolecolours, roleset};
use crate::tooling::image::{analyze, fetch};
use crate::tooling::image::fetch::FetchError;

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

    let bytes = match fetch::from_url(&image_url).await {
        Ok(bytes) => bytes,

        Err(error) => {
            let message = match error {
                fetch::FetchError::InvalidUrl(_) => {
                    "The URL provided is invalid"
                }
                fetch::FetchError::InvalidScheme => {
                    "The URL provided is not hypertext"
                }
                fetch::FetchError::TooLarge => {
                    "The image provided is over the 150MB maximum"
                }
                fetch::FetchError::Timeout => {
                    "The remote server took too long to respond"
                }
                fetch::FetchError::Request(_) => {
                    "Image download for inspection failed"
                }
            };
            ctx.send(poise::CreateReply::default().content(message)).await?;

            return Ok(());
        }
    };

    let info = match analyze(&bytes) {
        Ok(info) => info,
        Err(error) => {
            ctx.send(poise::CreateReply::default()
                .content("File is not a supported image")).await?;
            return Ok(());
        }
    };

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

    if let Some(exif) = info.exif {
        embed = embed.field("EXIF Metadata", exif.to_text(), false);
    } else {
        embed = embed.field("EXIF", "No EXIF metadata found", false);
    }

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}





