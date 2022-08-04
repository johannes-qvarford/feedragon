use crate::feed_provider::FeedProvider;
use crate::feed_transformer::FeedTransformer;
use actix_web::http::header;
use actix_web::web::ServiceConfig;
use actix_web::{get, web, App, HttpResponse, HttpServer};
use actix_web::{Responder, ResponseError};
use anyhow;
use anyhow::{Context, Error, Result};
use derive_more::Display;
use serde_derive::Deserialize;

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

struct AppState {
    provider: FeedProvider,
}

#[derive(Deserialize)]
struct Query {
    extract: Option<String>,
}

#[get("/feeds/{name}/atom.xml")]
async fn feed_category(
    info: web::Path<String>,
    state: web::Data<AppState>,
    query: web::Query<Query>,
) -> Result<String, LoggingError> {
    let category_name = &info.into_inner();
    let feed = state.provider.feed_by_category(category_name).await?;
    let feed = if query.extract.as_ref().filter(|e| **e == "media").is_some() {
        let transformer = FeedTransformer {
            http_client: state.provider.http_client.clone(),
        };
        transformer.extract_images_from_feed(feed).await
    } else {
        feed
    };
    let response_body = feed.serialize_to_string().with_context(|| {
        format!(
            "Failed to convert feed category {} to string",
            category_name
        )
    })?;
    Ok(response_body)
}

#[derive(Deserialize)]
struct ExternalPreviewPath {
    query_base64: String,
    tail: String,
}

#[get("/libreddit/ep/{query_base64}/{tail:.*}")]
async fn libreddit_redirect(info: web::Path<ExternalPreviewPath>) -> impl Responder {
    let tail = &info.tail;
    let query = String::from_utf8(base64::decode(&info.query_base64).unwrap()).unwrap();
    let value = format!("https://libreddit.privacy.qvarford.net/{tail}?{query}");
    HttpResponse::SeeOther()
        .append_header((header::LOCATION, value))
        .finish()
}

pub async fn start_server<F: Clone + Send + 'static + Fn() -> FeedProvider>(
    port: u16,
    factory: F,
) -> std::io::Result<()> {
    HttpServer::new(move || App::new().configure(config_app(factory())))
        .bind(("0.0.0.0", port))?
        .workers(1)
        .run()
        .await
}

fn config_app(provider: FeedProvider) -> Box<dyn Fn(&mut ServiceConfig)> {
    Box::new(move |cfg: &mut ServiceConfig| {
        cfg.app_data(web::Data::new(AppState {
            provider: provider.clone(),
        }))
        .service(feed_category)
        .service(libreddit_redirect);
        ()
    })
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use std::{collections::HashMap, rc::Rc};

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

    #[async_trait(?Send)]
    impl HttpClient for HashMapHttpClient {
        async fn get_bytes(&self, url: &Url) -> Result<Bytes> {
            let feed_short_name = self.hash_map.get(url.as_str()).unwrap();
            bytes(&format!("./src/res/static/{}.xml", feed_short_name.value))
        }
    }

    fn bytes(filename: &str) -> Result<Bytes> {
        let bytes = std::fs::read(filename)?.into();
        Ok(bytes)
    }

    #[derive(Clone)]
    struct FeedShortName {
        value: String,
        feed_type: FeedType,
    }

    #[derive(Clone)]
    enum FeedType {
        Nitter,
        TwitchRss,
        Invidious,
    }

    impl FeedShortName {
        fn url_string(&self) -> String {
            match self.feed_type {
                FeedType::Nitter => {
                    format!("https://nitter.privacy.qvarford.net/{}/rss", self.value)
                }
                FeedType::TwitchRss => {
                    format!("https://twitchrss.appspot.com/vodonly/{}", self.value)
                }
                FeedType::Invidious => "https://invidious.privacy.qvarford.net/feed/private".into(),
            }
        }
    }

    async fn start(
        category_to_short_names: HashMap<String, Vec<FeedShortName>>,
    ) -> impl Service<actix_http::Request, Response = ServiceResponse<BoxBody>, Error = actix_web::Error>
    {
        let url_to_content: HashMap<String, FeedShortName> = category_to_short_names
            .iter()
            .flat_map(|(_, short_names)| short_names)
            .map(|short_name| (short_name.url_string(), short_name.clone()))
            .collect();
        let http_client = Rc::new(HashMapHttpClient {
            hash_map: url_to_content,
        });
        let categories: HashMap<String, Vec<String>> = category_to_short_names
            .into_iter()
            .map(|(category, short_names)| {
                (
                    category,
                    short_names
                        .into_iter()
                        .map(|short_name| short_name.url_string())
                        .collect(),
                )
            })
            .collect();
        let provider = FeedProvider::from_categories_and_http_client_and_feed_deserializer(
            categories,
            http_client,
            Rc::new(default_feed_deserializer()),
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
                FeedShortName {
                    value: "PhilJamesson".into(),
                    feed_type: FeedType::Nitter,
                },
                FeedShortName {
                    value: "HardDriveMag".into(),
                    feed_type: FeedType::Nitter,
                },
                FeedShortName {
                    value: "tietuesday".into(),
                    feed_type: FeedType::TwitchRss,
                },
                FeedShortName {
                    value: "invidious".into(),
                    feed_type: FeedType::Invidious,
                },
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
            "Expected to find an item from the first nitter feed"
        );
        assert!(
            string.contains("three anime articles in a row????"),
            "Expected to find an item from the second nitter feed"
        );
        assert!(
            string.contains("Last Stream for a week! Rogue Legacy 2!"),
            "Expected to find an item from the first twitchrss feed"
        );
        assert!(
            string.contains("SmallAnt joined the discord call at the worst time"),
            "Expected to find an item from the first invidious feed"
        )
    }

    #[actix_rt::test]
    pub async fn feeds_that_cannot_be_fetched_are_ignored() {
        env_logger::init();

        let category_to_short_names = [(
            "comedy".into(),
            vec![
                FeedShortName {
                    value: "PhilJamesson".into(),
                    feed_type: FeedType::Nitter,
                },
                FeedShortName {
                    value: "HardDriveMag".into(),
                    feed_type: FeedType::Nitter,
                },
                FeedShortName {
                    value: "ThisFeedDoesNotExist".into(),
                    feed_type: FeedType::Nitter,
                },
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
