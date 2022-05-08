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
    use anyhow::Result;
    use std::{collections::HashMap, sync::Arc};

    use crate::{feed::default_feed_deserializer, http_client::HttpClient};

    use super::*;
    use actix_http::{
        body::{self, BoxBody},
        StatusCode,
    };
    use actix_web::{
        dev::{Service, ServiceResponse},
        test::{init_service, TestRequest},
    };
    use async_trait::async_trait;
    use bytes::Bytes;
    use reqwest::Url;

    struct HashMapHttpClient {
        hash_map: HashMap<String, FeedShortName>,
    }

    #[async_trait]
    impl HttpClient for HashMapHttpClient {
        async fn get_bytes(&self, url: &Url) -> Result<Bytes> {
            let feed_short_name = self.hash_map.get(url.as_str()).unwrap();
            bytes(&format!("./src/res/static/{}.xml", feed_short_name.0))
        }
    }

    fn bytes(filename: &str) -> Result<Bytes> {
        let bytes = std::fs::read(filename)?.into();
        Ok(bytes)
    }

    #[derive(Clone)]
    struct FeedShortName(String);

    async fn start(
        category_to_short_names: HashMap<String, Vec<FeedShortName>>,
    ) -> impl Service<actix_http::Request, Response = ServiceResponse<BoxBody>, Error = actix_web::Error>
    {
        let url_to_content: HashMap<String, FeedShortName> = category_to_short_names
            .iter()
            .flat_map(|(_, short_names)| short_names)
            .map(|short_name| {
                (
                    format!("https://nitter.privacy.qvarford.net/{}/rss", &short_name.0),
                    short_name.clone(),
                )
            })
            .collect();
        let http_client = Arc::new(HashMapHttpClient {
            hash_map: url_to_content,
        });
        let categories: HashMap<String, Vec<String>> = category_to_short_names
            .into_iter()
            .map(|(category, short_names)| {
                (
                    category,
                    short_names
                        .into_iter()
                        .map(|short_name| {
                            format!("https://nitter.privacy.qvarford.net/{}/rss", short_name.0)
                        })
                        .collect(),
                )
            })
            .collect();
        let provider = FeedProvider::from_categories_and_http_client_and_feed_deserializer(
            categories,
            http_client,
            Arc::new(default_feed_deserializer()),
        )
        .unwrap();
        let app = init_service(App::new().configure(config_app(provider))).await;
        app
    }

    async fn fetch_category(
        category: &str,
        category_to_short_names: HashMap<String, Vec<FeedShortName>>,
    ) -> (StatusCode, String) {
        let app = start(category_to_short_names).await;

        let request = TestRequest::get()
            .uri(&format!("/feeds/{}/atom.xml", category))
            .to_request();
        let response = app.call(request).await.unwrap();

        let status_code = response.status();
        let body: BoxBody = response.into_body();
        let bytes = body::to_bytes(body).await.unwrap();
        let string = String::from_utf8(bytes[..].into()).unwrap();

        (status_code, string)
    }

    #[actix_rt::test]
    pub async fn items_from_all_urls_in_category_are_merged() {
        let category_to_short_names = [(
            "comedy".into(),
            vec![
                FeedShortName("PhilJamesson".into()),
                FeedShortName("HardDriveMag".into()),
            ],
        )]
        .into();

        let (status_code, string) = fetch_category("comedy", category_to_short_names).await;

        assert!(
            status_code.is_success(),
            "Should be possible to fetch valid feeds."
        );
        assert!(
            string.contains("max one jared leto role per month please. between morbius and the wework thing ive forgotten what other people look like"),
            "Expected to find an item from the first feed"
        );
        assert!(
            string.contains("three anime articles in a row????"),
            "Expected to find an item from the second feed"
        )
    }

    #[actix_rt::test]
    pub async fn feeds_that_cannot_be_fetched_are_ignored() {
        let category_to_short_names = [(
            "comedy".into(),
            vec![
                FeedShortName("PhilJamesson".into()),
                FeedShortName("HardDriveMag".into()),
                FeedShortName("ThisFeedDoesNotExist".into()),
            ],
        )]
        .into();

        let (status_code, string) = fetch_category("comedy", category_to_short_names).await;

        assert!(
            status_code.is_success(),
            "Should be possible to fetch valid feeds."
        );
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
