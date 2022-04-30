use std::{fs::read_to_string, sync::Arc};

mod cache;
mod caching_http_client;
mod config;
mod feed;
mod feed_provider;
mod http_client;
mod server;

use config::Config;
use feed_provider::FeedProvider;
use http_client::{HttpClient, ReqwestHttpClient};

extern crate serde_derive;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let config = Config::from_toml_str(&read_to_string("feedragon.toml").unwrap()).unwrap();
    let http_client: Arc<dyn HttpClient> = Arc::new(ReqwestHttpClient {});
    let provider =
        FeedProvider::from_categories_and_http_client(config.categories, http_client).unwrap();
    let starter = server::Starter {
        port: 8080,
        provider,
    };
    starter.start_server().await
}
