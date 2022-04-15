mod atom;
mod atom_serialization;
mod rss_serialization;
mod serialization;

#[macro_use]
extern crate serde_derive;
use crate::atom_serialization::AtomDeserializer;
use crate::rss_serialization::RssDeserializer;
use actix_web::ResponseError;
use actix_web::{get, web, App, HttpServer, Responder};
use anyhow;
use anyhow::{Context, Error, Result};
use derive_more::Display;
use reqwest;
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

#[get("/{id}/{name}/index.html")]
async fn index(params: web::Path<(u32, String)>) -> impl Responder {
    let (id, name) = params.into_inner();
    format!("Hello {}! id:{}", name, id)
}

#[derive(Deserialize)]
struct ProxyQuery {
    proxied: String,
}

#[get("/feeds/atom-proxy")]
async fn atom_proxy(q: web::Query<ProxyQuery>) -> Result<String, LoggingError> {
    let body = reqwest::get(q.proxied.clone())
        .await
        .with_context(|| format!("Failed to fetch atom feed {}", q.proxied.clone()))?
        .bytes()
        .await
        .context("Failed to extract byte request body")?;

    let parser = AtomDeserializer {};
    let feed = parser
        .parse_feed_from_bytes(body.as_ref())
        .with_context(|| format!("Failed to parse atom feed {}", q.proxied.clone()))?;
    let response_body = feed.serialize_to_string().with_context(|| {
        format!(
            "Failed to convert parsed atom feed to string {}",
            q.proxied.clone()
        )
    })?;
    Ok(response_body)
}

#[get("/feeds/rss-proxy")]
async fn rss_proxy(q: web::Query<ProxyQuery>) -> Result<String, LoggingError> {
    let body = reqwest::get(q.proxied.clone())
        .await
        .with_context(|| format!("Failed to fetch atom feed {}", q.proxied.clone()))?
        .bytes()
        .await
        .context("Failed to extract byte request body")?;

    let parser = RssDeserializer {};
    let feed = parser
        .parse_feed_from_bytes(body.as_ref())
        .with_context(|| format!("Failed to parse atom feed {}", q.proxied.clone()))?;

    let response_body = feed.serialize_to_string().with_context(|| {
        format!(
            "Failed to convert parsed atom feed to string {}",
            q.proxied.clone()
        )
    })?;
    Ok(response_body)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    HttpServer::new(|| {
        App::new()
            .service(index)
            .service(atom_proxy)
            .service(rss_proxy)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
