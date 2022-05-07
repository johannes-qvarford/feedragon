use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use reqwest::Url;

use crate::cache::TimedCache;
use crate::http_client::HttpClient;

pub struct CachingHttpClient {
    cache: TimedCache<Url, Bytes>,
    delegate: Arc<dyn HttpClient>,
}

impl CachingHttpClient {
    pub fn new<I: Iterator<Item = Url>>(
        delegate: Arc<dyn HttpClient>,
        duration: chrono::Duration,
        feed_urls: I,
    ) -> CachingHttpClient {
        CachingHttpClient {
            delegate,
            cache: TimedCache::from_expiration_duration_and_keys(duration, feed_urls),
        }
    }
}

#[async_trait]
impl HttpClient for CachingHttpClient {
    async fn get_bytes(&self, url: &Url) -> Result<Bytes> {
        let result = self
            .cache
            .get_or_compute(url.clone(), || self.delegate.get_bytes(url))
            .await
            .with_context(|| format!("Failed get cache http request {}, or compute it", url));
        result
    }
}
