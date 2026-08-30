use reqwest::{Client, Url};
use std::time::Duration;
use thiserror::Error;

const MAX_IMAGE_SIZE: usize = 150 * 1024 * 1024; // 150 MB

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("invalid URL")]
    InvalidUrl(#[from] url::ParseError),

    #[error("URL must use hypertext")]
    InvalidScheme,

    #[error("image is too large (maximum size is 150 MB)")]
    TooLarge,

    #[error("request timed out")]
    Timeout,

    #[error("HTTP request failed")]
    Request(#[from] reqwest::Error),
}

pub async fn from_url(url: &str) -> Result<Vec<u8>, FetchError> {
    let url = Url::parse(url)?;

    match url.scheme() {
        "http" | "https" => {}
        _ => return Err(FetchError::InvalidScheme),
    }

    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()?;

    let response = match client.get(url).send().await {
        Ok(response) => response,
        Err(error) if error.is_timeout() => {
            return Err(FetchError::Timeout);
        }
        Err(error) => return Err(FetchError::Request(error)),
    };

    let response = response.error_for_status()?;

    if let Some(length) = response.content_length() {
        if length > MAX_IMAGE_SIZE as u64 {
            return Err(FetchError::TooLarge);
        }
    }

    let bytes = response.bytes().await?;

    if bytes.len() > MAX_IMAGE_SIZE {
        return Err(FetchError::TooLarge);
    }

    Ok(bytes.to_vec())
}
