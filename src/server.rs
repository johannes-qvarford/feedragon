use crate::feed_provider::FeedProvider;
use actix_web::web::ServiceConfig;
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
        HttpServer::new(move || App::new().configure(config_app(self.provider.clone())))
            .bind(("0.0.0.0", self.port))?
            .run()
            .await
    }
}

fn config_app(provider: FeedProvider) -> Box<dyn Fn(&mut ServiceConfig)> {
    Box::new(move |cfg: &mut ServiceConfig| {
        cfg.app_data(web::Data::new(AppState {
            provider: provider.clone(),
        }))
        .service(feed_category);
        ()
    })
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, sync::Arc};

    use crate::http_client::HttpClient;

    use super::*;
    use actix_files::Files;
    use actix_http::body::{self, BoxBody};
    use actix_web::{
        dev::{Service, ServiceResponse},
        test::{init_service, TestRequest},
    };
    use async_trait::async_trait;
    use bytes::Bytes;
    use reqwest::Url;

    fn config_static_files() -> Box<dyn Fn(&mut ServiceConfig)> {
        Box::new(move |cfg: &mut ServiceConfig| {
            cfg.service(Files::new("/static", "./res/static").prefer_utf8(true));
            ()
        })
    }

    struct HashMapHttpClient {
        hash_map: HashMap<String, Bytes>,
    }

    #[async_trait]
    impl HttpClient for HashMapHttpClient {
        async fn get_bytes(&self, url: &Url) -> Result<Bytes> {
            Ok(self.hash_map.get(url.as_str()).unwrap().clone())
        }
    }

    fn bytes(filename: &str) -> Bytes {
        std::fs::read(filename).unwrap().into()
    }

    async fn start(
        categories: HashMap<String, Vec<String>>,
    ) -> impl Service<actix_http::Request, Response = ServiceResponse<BoxBody>, Error = actix_web::Error>
    {
        let map = [
            (
                "https://nitter.privacy.qvarford.net/PhilJamesson/rss".into(),
                bytes("./src/res/static/feed1.xml"),
            ),
            (
                "https://nitter.privacy.qvarford.net/HardDriveMag/rss".into(),
                bytes("./src/res/static/feed2.xml"),
            ),
        ]
        .into();
        let http_client = Arc::new(HashMapHttpClient { hash_map: map });
        let provider =
            FeedProvider::from_categories_and_http_client(categories, http_client).unwrap();
        let app = init_service(
            App::new()
                .configure(config_app(provider))
                .configure(config_static_files()),
        )
        .await;
        app
    }

    #[actix_rt::test]
    pub async fn items_from_all_urls_in_category_are_merged() {
        let app = start(
            [(
                "comedy".into(),
                vec![
                    "https://nitter.privacy.qvarford.net/PhilJamesson/rss".into(),
                    "https://nitter.privacy.qvarford.net/HardDriveMag/rss".into(),
                ],
            )]
            .into(),
        )
        .await;

        let request = TestRequest::get()
            .uri("/feeds/comedy/atom.xml")
            .to_request();
        let response = app.call(request).await.unwrap();

        assert!(
            response.status().is_success(),
            "Should be possible to fetch valid feeds."
        );
        let body: BoxBody = response.into_body();
        let bytes = body::to_bytes(body).await.unwrap();
        let string = String::from_utf8(bytes[..].into()).unwrap();
        assert!(
            string.contains("max one jared leto role per month please. between morbius and the wework thing ive forgotten what other people look like"),
            "Expected to find an item from the first feed"
        );
        assert!(
            string.contains("three anime articles in a row????"),
            "Expected to find an item from the second feed"
        )
    }
}
