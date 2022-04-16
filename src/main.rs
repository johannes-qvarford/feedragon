use category::FeedProvider;

mod atom;
mod atom_serialization;
mod category;
mod model;
mod rss_serialization;
mod serialization;
mod server;

extern crate serde_derive;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let provider = FeedProvider::from_file("feedragon.toml").unwrap();
    let starter = server::Starter {
        port: 8080,
        provider,
    };
    starter.start_server().await
}
