mod parsing;
mod atom_parser;
mod xml_tree;

#[macro_use] extern crate serde_derive;
use crate::xml_tree::write_element_to_string;
use actix_web::web::Buf;
use actix_web::{get, web, App, HttpServer, Responder};
use reqwest;
use std::error::Error;
use parsing::Parser;
use anyhow;

#[get("/{id}/{name}/index.html")]
async fn index(params: web::Path<(u32, String)>) -> impl Responder {
    let (id, name) = params.into_inner();
    format!("Hello {}! id:{}", name, id)
}

#[derive(Deserialize)]
struct InvidiousProxyQuery {
    proxied: String
}

#[get("/feeds/invidious-proxy")]
async fn invidious_proxy(q: web::Query<InvidiousProxyQuery>) -> Result<String, Box<dyn Error>> {
    let body = reqwest::get(q.proxied.clone())
        .await?
        .bytes()
        .await?;

    let tree = xmltree::Element::parse(body.reader())?;

    let parser = atom_parser::AtomParser{};
    let feed = parser.parse_feed(tree).map_err(anyhow::Error::msg)?;
    let response_tree = parser.serialize_feed(feed);

    let response_body = write_element_to_string(&response_tree, &q.proxied)?;
    Ok(response_body)
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(index).service(invidious_proxy))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}