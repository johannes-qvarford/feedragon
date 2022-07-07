use std::time::Duration;

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
        let client = reqwest::ClientBuilder::new()
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .connect_timeout(Duration::from_secs(60))
            .build()
            .context("Failed to create client")?;

        let body = client
            .get(url.clone())
            .send()
            .await
            .with_context(|| format!("Failed to download resource {}", url))?
            .bytes()
            .await
            .context("Failed to extract byte request body")?;
        Ok(body)
    }
}
