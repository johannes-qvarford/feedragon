use std::fs::read_to_string;

mod config;
mod feed;
mod feed_provider;
mod server;

use config::Config;
use feed_provider::FeedProvider;

extern crate serde_derive;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let config = Config::from_toml_str(&read_to_string("feedragon.toml").unwrap()).unwrap();
    let provider = FeedProvider::from_categories(config.categories).unwrap();
    let starter = server::Starter {
        port: 8080,
        provider,
    };
    starter.start_server().await
}
