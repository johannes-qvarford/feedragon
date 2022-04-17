use crate::feed_provider::FeedProvider;
use actix_web::ResponseError;
use actix_web::{get, web, App, HttpServer};
use anyhow;
use anyhow::{Context, Error, Result};
use derive_more::Display;

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
    let response_body = feed.serialize_to_string().with_context(|| {
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

#[derive(Clone)]
pub struct Starter {
    pub provider: FeedProvider,
    pub port: u16,
}

impl Starter {
    pub async fn start_server(self) -> std::io::Result<()> {
        HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(AppState {
                    provider: self.provider.clone(),
                }))
                .service(feed_category)
        })
        .bind(("0.0.0.0", self.port))?
        .run()
        .await
    }
}
