mod serialization;
mod atom_serialization;
mod rss_serialization;
mod atom;

#[macro_use] extern crate serde_derive;
use crate::rss_serialization::RssDeserializer;
use crate::atom_serialization::AtomDeserializer;
use actix_web::{get, web, App, HttpServer, Responder};
use reqwest;
use std::error::Error;
use serialization::FeedDeserializer;
use anyhow;

#[get("/{id}/{name}/index.html")]
async fn index(params: web::Path<(u32, String)>) -> impl Responder {
    let (id, name) = params.into_inner();
    format!("Hello {}! id:{}", name, id)
}

#[derive(Deserialize)]
struct ProxyQuery {
    proxied: String
}

#[get("/feeds/atom-proxy")]
async fn atom_proxy(q: web::Query<ProxyQuery>) -> Result<String, Box<dyn Error>> {
    let body = reqwest::get(q.proxied.clone())
        .await?
        .bytes()
        .await?;

    let parser = AtomDeserializer{};
    let feed = parser.parse_feed_from_bytes(body.as_ref()).map_err(anyhow::Error::msg)?;

    let response_body = feed.serialize_to_string()?;
    Ok(response_body)
}

#[get("/feeds/rss-proxy")]
async fn rss_proxy(q: web::Query<ProxyQuery>) -> Result<String, Box<dyn Error>> {
    let body = reqwest::get(q.proxied.clone())
        .await?
        .bytes()
        .await?;

    let parser = RssDeserializer{};
    let feed = parser.parse_feed_from_bytes(body.as_ref()).map_err(anyhow::Error::msg)?;

    let response_body = feed.serialize_to_string()?;
    Ok(response_body)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(index).service(atom_proxy).service(rss_proxy))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}