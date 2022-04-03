mod parsing;
mod atom_parser;
mod xml_tree;

#[macro_use] extern crate serde_derive;
use actix_web::web::Buf;
use actix_web::{get, web, App, HttpServer, Responder, HttpResponse};
use reqwest;
use std::error::Error;
use parsing::Parser;
use anyhow;
use bytes::BufMut;
use tempfile::{tempdir};
use md5::{Md5, Digest};
use std::fs::{read_to_string, File};
use std::str;
use std::path::{Path, PathBuf};
use hex;

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
    println!("{:?}", response_tree);

    let dir = tempdir()?;
    let mut hasher = Md5::new();
    hasher.update(q.proxied.as_bytes());
    let v: Vec<_> = hasher.finalize().into_iter().collect();
    let filename = hex::encode(&v[..]);
    let path_buf: PathBuf = dir.path().join(filename);
    response_tree.write(File::create(path_buf.as_path())?)?;
    let response_body = read_to_string(path_buf.as_path())?;

    Ok(response_body)
    //format!("hello {}", q.proxied)
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(index).service(invidious_proxy))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}