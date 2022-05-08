use std::{fs::read_to_string, sync::Arc};

mod cache;
mod caching_http_client;
mod config;
mod feed;
mod feed_provider;
mod http_client;
mod server;

use caching_http_client::CachingHttpClient;
use config::Config;
use feed::default_feed_deserializer;
use feed_provider::FeedProvider;
use http_client::ReqwestHttpClient;
use reqwest::Url;

extern crate serde_derive;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let config = Config::from_toml_str(&read_to_string("feedragon.toml").unwrap()).unwrap();
    let http_client = ReqwestHttpClient {};
    let feed_deserializer = Arc::new(default_feed_deserializer());
    let feed_urls = config
        .categories
        .iter()
        .flat_map(|(_, url_strings)| url_strings)
        .map(|s| Url::parse(s).unwrap());
    let http_client =
        CachingHttpClient::new(Arc::new(http_client), chrono::Duration::hours(1), feed_urls);
    let http_client = Arc::new(http_client);
    let provider = FeedProvider::from_categories_and_http_client_and_feed_deserializer(
        config.categories,
        http_client,
        feed_deserializer,
    )
    .unwrap();
    let starter = server::Starter {
        port: 8080,
        provider,
    };
    starter.start_server().await
}
