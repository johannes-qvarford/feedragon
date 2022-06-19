use anyhow::{Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use reqwest::Url;

#[async_trait(?Send)]
pub trait HttpClient {
    async fn get_bytes(&self, url: &Url) -> Result<Bytes>;
}

pub struct ReqwestHttpClient {}

#[async_trait(?Send)]
impl HttpClient for ReqwestHttpClient {
    async fn get_bytes(&self, url: &Url) -> Result<Bytes> {
        let body = reqwest::get(url.clone())
            .await
            .with_context(|| format!("Failed to download resource {}", url))?
            .bytes()
            .await
            .context("Failed to extract byte request body")?;
        Ok(body)
    }
}
