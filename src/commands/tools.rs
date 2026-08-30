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
    let bytes = if let Some(image) = image {
        fetch::from_url(&image.url).await?
    } else {
        fetch::from_url(&url.unwrap().as_str()).await?
    };

    let info = analyze::analyze(&bytes)?;

    let mut embed = serenity::CreateEmbed::default()
        .title("Image Information")
        .color(0x5865F2)
        .field("Image Format", format!("`{:?}`", info.format.extensions_str()), false)
        .field("Size", format!("`{:.2}` MB", info.size as f64 / 1000000.0), false)
        .field("Resolution", format!("{} x {}", info.width, info.height), false)
        .field("Megapixels", format!("{:.2} MP", info.megapixels), false);

    if let Some(exif) = info.exif {
        embed = embed.field("EXIF", exif, false);
    } else {
        embed = embed.field("EXIF", "No EXIF found on this image", false);
    }
    
    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}





