use crate::serenity;
pub(crate) use crate::types::{Context, Data, Error};
use exif::{In, Reader, Tag};
use reqwest::{get};
use crate::commands::fun::{rcstatus, rolecolours, roleset};

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
        reqwest::get(&image.url).await?.bytes().await?
    } else {
        reqwest::get(url.unwrap()).await?.bytes().await?
    };

    let img = image::load_from_memory(bytes.as_ref())?;
    let width = img.width();
    let height = img.height();

    let mp = (width as f64 * height as f64) / 1000000.0;
    let img_type = image::guess_format(&bytes)?;
    let size = bytes.len();

    let mut cursor = std::io::Cursor::new(bytes.as_ref());
    let exif = Reader::new().read_from_container(&mut cursor).ok();

    let mut embed = serenity::CreateEmbed::default()
        .title("Image Information")
        .color(0x5865F2)
        .field("Image Format", format!("`{:?}`", img_type.extensions_str()), false)
        .field("Size", format!("`{:.2}` MB", size as f64 / 1000000.0), false)
        .field("Resolution", format!("{} x {}", width, height), false)
        .field("Megapixels", format!("{:.2} MP", mp), false);

    if let Some(exif) = exif {
        let exif_text = exif.fields().map(|field| {
            format!("**{}**: {}", field.tag, field.display_value().with_unit(&exif)
            )
        })
        .collect::<Vec<_>>().join("\n");

        if !exif_text.is_empty() {
            embed = embed.field("EXIF", exif_text, false);
        }
    }


    //test run
    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}





