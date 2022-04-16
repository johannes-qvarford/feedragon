mod atom;
mod atom_serialization;
mod category;
mod model;
mod rss_serialization;
mod serialization;

extern crate serde_derive;

use actix_web::ResponseError;
use actix_web::{get, web, App, HttpServer};
use anyhow;
use anyhow::{Context, Error, Result};
use category::FeedProvider;
use derive_more::Display;

use serialization::FeedDeserializer;

#[derive(Display, Debug)]
struct LoggingError {
    err: Error,
}

impl ResponseError for LoggingError {}

impl From<Error> for LoggingError {
    fn from(err: Error) -> LoggingError {
        log::error!("{:#?}", err);
        LoggingError { err }
    }
}

#[get("/feeds/{name}/atom.xml")]
async fn feed_category(
    info: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<String, LoggingError> {
    let category_name = &info.into_inner();
    let feed = state.provider.feed_by_category(category_name).await?;
    let response_body = feed.serialize_to_string().with_context(move || {
        format!(
            "Failed to convert feed category {} to string",
            category_name
        )
    })?;
    Ok(response_body)
}

struct AppState {
    provider: FeedProvider,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    HttpServer::new(|| {
        let provider = FeedProvider::from_file("feedragon.toml").unwrap();
        App::new()
            .app_data(web::Data::new(AppState { provider }))
            .service(feed_category)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
