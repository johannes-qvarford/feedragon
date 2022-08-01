use std::{fs::read_to_string, rc::Rc, sync::Arc};

mod cache;
mod caching_http_client;
mod config;
mod feed;
mod feed_provider;
mod feed_transformer;
mod http_client;
mod server;

use caching_http_client::CachingHttpClient;
use config::Config;
use feed::default_feed_deserializer;
use feed_provider::FeedProvider;
use http_client::ReqwestHttpClient;
use reqwest::Url;
use server::start_server;

extern crate serde_derive;

fn thread_local_feed_provider(config: Arc<Config>) -> FeedProvider {
    let http_client = ReqwestHttpClient {};
    let feed_deserializer = Rc::new(default_feed_deserializer());
    let feed_urls = config
        .categories
        .iter()
        .flat_map(|(_, url_strings)| url_strings)
        .map(|s| Url::parse(s).unwrap());
    let http_client =
        CachingHttpClient::new(Rc::new(http_client), chrono::Duration::hours(1), feed_urls);
    let http_client = Rc::new(http_client);
    let provider = FeedProvider::from_categories_and_http_client_and_feed_deserializer(
        config.categories.clone(),
        http_client,
        feed_deserializer,
    )
    .unwrap();
    provider
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let config =
        Arc::new(Config::from_toml_str(&read_to_string("feedragon.toml").unwrap()).unwrap());
    start_server(8080, move || thread_local_feed_provider(config.clone())).await
}
