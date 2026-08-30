use reqwest::{Client, Url};
use std::time::Duration;

const MAX_IMAGE_SIZE: usize = 150 * 1024 * 1024; // 150 MB

pub async fn from_url(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let url = Url::parse(url)?;

    match url.scheme() {
        "http" | "https" => {}
        _ => return Err("URL provided doesnt use hypertext".into()),
    }

    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()?;

    let response = client.get(url).send().await?.error_for_status()?;

    if let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE) {
        let content_type = content_type.to_str()?.split(';').next().unwrap_or("");

        if !content_type.starts_with("image/") {
            return Err(format!("URL does not point to an image: {}", content_type).into());
        }
    }

    if let Some(length) = response.content_length() {
        if length > MAX_IMAGE_SIZE as u64 {
            return Err(format!("Image is over maximum size of {}MB", MAX_IMAGE_SIZE/1024/1024).into());
        }
    }

    let bytes = response.bytes().await?;

    if bytes.len() > MAX_IMAGE_SIZE {
        return Err(format!("Image is over maximum size of {}MB", MAX_IMAGE_SIZE/1024/1024).into());
    }

    Ok(bytes.to_vec())
}
