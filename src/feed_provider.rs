use crate::feed::{merge_feeds, Feed, FeedDeserializer};
use crate::http_client::HttpClient;
use anyhow::{Context, Error, Result};

use futures::future::join_all;
use std::collections::HashMap;
use std::rc::Rc;
use std::vec::IntoIter;
use tokio::task::{self, JoinHandle};
use url::Url;

// TODO: We need to support dynamic serializers, so that we don't have to keep track of which feeds are
// atom feeds and which are rss feeds. For now, only support rss feeds.

#[derive(Clone)]
pub struct FeedProvider {
    categories: HashMap<String, Category>,
    pub http_client: Rc<dyn HttpClient>,
    feed_deserializer: Rc<dyn FeedDeserializer>,
}

impl FeedProvider {
    pub fn from_categories_and_http_client_and_feed_deserializer(
        categories: HashMap<String, Vec<String>>,
        http_client: Rc<dyn HttpClient>,
        feed_deserializer: Rc<dyn FeedDeserializer>,
    ) -> Result<FeedProvider> {
        let categories =
            categories
                .into_iter()
                .map(|name_and_urls| -> Result<(String, Category)> {
                    let feed_urls: Vec<Result<Url>> = name_and_urls
                        .1
                        .into_iter()
                        .map(|url_string| {
                            Url::parse(&url_string).with_context(|| {
                                format!("Failed to parse url {} in category", &url_string)
                            })
                        })
                        .collect();

                    let feed_urls: Vec<Url> = try_all(feed_urls.into_iter())
                        .with_context(|| {
                            format!("Failed to parse url in category {}", name_and_urls.0)
                        })?
                        .collect();

                    Ok((name_and_urls.0, Category { feed_urls }))
                });
        let categories: HashMap<_, _> = try_all(categories)
            .with_context(|| format!("Failed to parse categories due to url conversion issues.",))?
            .collect();
        Ok(FeedProvider {
            categories,
            http_client,
            feed_deserializer,
        })
    }

    pub async fn feed_by_category(&self, category_name: &str) -> Result<Feed> {
        let category = self
            .categories
            .get(category_name)
            .ok_or_else(|| Error::msg(format!("Failed to find feed category {}", category_name)))?;

        let feed_results = self.category_feeds(category).await;
        let feeds = FeedProvider::discard_err_feeds(feed_results, category_name);

        Ok(merge_feeds(
            category_name.into(),
            format!(
                "https://feedragon.privacy.qvarford.net/feeds/{}/atom.xml",
                category_name
            )[..]
                .try_into()?,
            feeds,
        ))
    }

    async fn category_feeds<'a>(
        &'a self,
        category: &'a Category,
    ) -> impl Iterator<Item = Result<Feed>> {
        type Handle = JoinHandle<Result<Feed>>;
        let mut feed_results: Vec<Handle> = vec![];
        for url in category.feed_urls.iter() {
            let future = FeedProvider::get_feed(
                self.http_client.clone(),
                self.feed_deserializer.clone(),
                url.clone(),
            );
            feed_results.push(task::spawn_local(future));
        }

        let results = join_all(feed_results).await;
        let flattened_results = results.into_iter().map(|rr| Ok(rr.map_err(Error::new)??));
        flattened_results
    }

    async fn get_feed(
        http_client: Rc<dyn HttpClient>,
        deserializer: Rc<dyn FeedDeserializer>,
        url: Url,
    ) -> Result<Feed> {
        let bytes = http_client
            .get_bytes(&url)
            .await
            .with_context(|| format!("Failed downloading feed {} as part of category", url))?;
        let mut feed = deserializer
            .parse_feed_from_bytes(bytes.as_ref())
            .with_context(|| format!("Failed to parse feed {} as part of category", url))?;
        for entry in feed.entries.iter_mut() {
            entry.id = entry
                .id
                .replace("www.reddit.com", "libreddit.privacy.qvarford.net");
        }
        Ok(feed)
    }

    fn discard_err_feeds<I: Iterator<Item = Result<Feed>>>(
        feed_results: I,
        category_name: &str,
    ) -> Vec<Feed> {
        let feeds = feed_results.flat_map(|feed_result| {
            match feed_result {
                Ok(feed) => Some(feed),
                Err(err) => {
                    log::warn!("Failed to fetch feed as part of category {}. It will not be part of the next category feed.\n{:#?}", category_name, err);
                    None
                }
            }
        }).collect();
        feeds
    }
}

#[derive(Clone)]
struct Category {
    feed_urls: Vec<Url>,
}

fn try_all<T: Sized, E, I: Iterator<Item = Result<T, E>> + Sized>(it: I) -> Result<IntoIter<T>, E> {
    let mut items: Vec<T> = vec![];
    for item in it {
        items.push(item?);
    }
    Ok(items.into_iter())
}
