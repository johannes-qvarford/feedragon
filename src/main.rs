mod parsing;
mod atom_parser;
mod xml_tree;

#[macro_use] extern crate serde_derive;
use actix_web::{get, web, App, HttpServer, Responder};
use reqwest;
use std::error::Error;

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
        .text()
        .await?;
    Ok(body)
    //format!("hello {}", q.proxied)
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(index).service(invidious_proxy))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}