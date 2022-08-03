use std::rc::Rc;
use std::str;

use crate::{
    feed::{model::Entry, Feed},
    http_client::HttpClient,
};

use anyhow::{Context, Error, Result};
use futures::{stream, FutureExt, Stream, StreamExt, TryStreamExt};
use reqwest::Url;
use scraper::{Html, Selector};

struct FeedTransformer {
    http_client: Rc<dyn HttpClient>,
}

impl FeedTransformer {
    pub async fn extract_images_from_feed(&self, feed: Feed) -> Result<Feed> {
        let feed = feed;
        let entry_results = stream::iter(feed.entries).flat_map(|e| self.convert_to_image_entry(e));

        let results: Vec<Result<Vec<Entry>>> = entry_results.collect().await;
        let results: Result<Vec<Vec<Entry>>> = results.into_iter().collect();
        let entries: Vec<Entry> = results
            .with_context(|| format!("Could not extract images from feed"))?
            .into_iter()
            .flatten()
            .collect();

        Ok(Feed {
            author_name: feed.author_name,
            id: feed.id,
            link: feed.link,
            title: feed.title,
            entries,
        })
    }

    fn convert_to_image_entry(&self, e: Entry) -> impl Stream<Item = Result<Vec<Entry>>> + '_ {
        let links = self.extract_images_from_page(e.id).into_stream();

        let entries = links.map_ok(move |links| {
            links
                .into_iter()
                .map(|link| Entry {
                    id: link.to_string(),
                    link: link.to_string(),
                    summary: e.summary.clone(),
                    title: e.title.clone(),
                    updated: e.updated,
                })
                .collect::<Vec<_>>()
        });
        entries
    }

    async fn extract_images_from_page(&self, url: String) -> Result<Vec<Url>> {
        let url: Url = Url::try_from(url.as_str())
            .with_context(|| format!("Invalid link {url} during image extraction."))?;
        let bytes = self.http_client.get_bytes(&url).await?;
        let content: &str =
            str::from_utf8(&bytes).with_context(|| format!("Page at {url} is not valid utf8"))?;
        let html = Html::parse_document(content);

        // TODO: Nitter specific, do different things for libreddit
        // TODO: Nitter videos are kinda bad, maybe skip them to begin with?
        // TODO: save-to-mega can't really handle hls videos either way it seems like. webm works.
        // TODO: Don't use og:image for libreddit since it's only a thumb.

        let selector = Selector::parse(r#"meta[property="og:image"]"#)
            .or_else(|e| Err(Error::msg(format!("Could not parse selector {e:?}"))))?;
        let image_links = html.select(&selector).map(|element_ref| {
            element_ref
                .value()
                .attr("content")
                .ok_or_else(|| Error::msg("Missing content attribute for og:image property"))
        });
        let r: Result<Vec<Url>> = image_links
            .map(|s| -> Result<Url> {
                Url::try_from(s?)
                    .with_context(|| format!("Invalid link {url} during image extraction."))
            })
            .collect();
        r
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, rc::Rc};

    use anyhow::Result;
    use async_trait::async_trait;
    use bytes::Bytes;
    use chrono::DateTime;
    use reqwest::Url;

    use crate::{
        feed::{model::Entry, Feed},
        http_client::HttpClient,
    };

    use super::FeedTransformer;

    struct Page(&'static str);

    struct HashMapHttpClient {
        hash_map: HashMap<String, Page>,
    }

    #[async_trait(?Send)]
    impl HttpClient for HashMapHttpClient {
        async fn get_bytes(&self, url: &Url) -> Result<Bytes> {
            let page = self.hash_map.get(url.as_str()).unwrap();
            bytes(&format!("./src/res/static/pages/{}.html", page.0))
        }
    }

    fn bytes(filename: &str) -> Result<Bytes> {
        let bytes = std::fs::read(filename)?.into();
        Ok(bytes)
    }

    #[actix_rt::test]
    async fn nitter_single_image_is_extracted() {
        // map a serebii page
        let url = "https://nitter.privacy.qvarford.net/SerebiiNet/status/1554195261253046272";
        let transformer = transformer([(url.into(), Page("nitter_single_image"))].into());
        let feed = feed(vec![url]);
        let expected_url = "https://nitter.privacy.qvarford.net/pic/media%2FFY_ABU8XoAAoLX6.jpg";
        let mut expected_feed = feed.clone();

        let transformed_feed = transformer.extract_images_from_feed(feed).await.unwrap();

        expected_feed.entries[0].id = expected_url.into();
        expected_feed.entries[0].link = expected_url.into();
        assert_eq!(expected_feed, transformed_feed)
    }

    #[actix_rt::test]
    async fn empty_if_there_are_no_images_to_extract() {
        let url = "https://nitter.privacy.qvarford.net/jeremysmiles/status/1554270809509748737";
        let transformer = transformer([(url.into(), Page("nitter_no_image"))].into());
        let feed = feed(vec![url]);
        let mut expected_feed = feed.clone();

        let transformed_feed = transformer.extract_images_from_feed(feed).await.unwrap();

        expected_feed.entries = vec![];
        assert_eq!(expected_feed, transformed_feed,)
    }

    #[actix_rt::test]
    async fn empty_if_the_page_could_not_be_downloaded() {}

    // empty_if_the_page_was_empty

    fn transformer(page_map: HashMap<String, Page>) -> FeedTransformer {
        let http_client = HashMapHttpClient { hash_map: page_map };
        FeedTransformer {
            http_client: Rc::new(http_client),
        }
    }

    fn feed(ids: Vec<&'static str>) -> Feed {
        Feed {
            author_name: "".into(),
            id: "".into(),
            link: "https://google.com".try_into().unwrap(),
            title: "".into(),
            entries: ids
                .into_iter()
                .map(|id| Entry {
                    id: id.into(),
                    summary: "".into(),
                    title: "".into(),
                    link: id.into(),
                    updated: DateTime::parse_from_rfc3339("2022-03-22T07:26:01+00:00")
                        .unwrap()
                        .into(),
                })
                .collect(),
        }
    }
}
