use std::rc::Rc;
use std::str;

use crate::{
    feed::{model::Entry, Feed},
    http_client::HttpClient,
};

use anyhow::{Context, Error, Result};
use futures::{stream, FutureExt, Stream, StreamExt};
use log::warn;
use reqwest::Url;
use scraper::{Html, Selector};

pub struct FeedTransformer {
    pub http_client: Rc<dyn HttpClient>,
}

impl FeedTransformer {
    pub async fn extract_images_from_feed(&self, feed: Feed) -> Feed {
        let feed = feed;
        let stream = stream::iter(feed.entries).flat_map(|e| self.extract_images_from_entry(e));

        let entries: Vec<Entry> = stream
            .collect::<Vec<Vec<_>>>()
            .await
            .into_iter()
            .flatten()
            .collect();

        Feed {
            author_name: feed.author_name,
            id: feed.id,
            link: feed.link,
            title: feed.title,
            entries,
        }
    }

    fn extract_images_from_entry(&self, e: Entry) -> impl Stream<Item = Vec<Entry>> + '_ {
        let id = e.id.clone();
        let links = self.extract_images_from_page(e.id).into_stream();

        let entries = links.map(move |links_result| match links_result {
            Err(error) => {
                warn!("Could not extract images from {id}. Error: {error}");
                vec![]
            }
            Ok(links) => links
                .into_iter()
                .map(|mut url| {
                    url.set_query(None);
                    url
                })
                .map(|link| Entry {
                    id: link.to_string(),
                    link: link.to_string(),
                    summary: e.summary.clone(),
                    title: e.title.clone(),
                    updated: e.updated,
                })
                .collect::<Vec<_>>(),
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

        let image_links = FeedTransformer::scrape_images_from_html(&url, html)?;
        let r: Result<Vec<Url>> = image_links
            .into_iter()
            .map(|s| -> Result<Url> {
                Url::try_from(s?.as_str())
                    .with_context(|| format!("Invalid link {url} during image extraction."))
            })
            .collect();
        r
    }

    fn scrape_images_from_html(url: &Url, html: Html) -> Result<Vec<Result<String>>> {
        // TODO: Nitter videos are kinda bad, maybe skip them to begin with?
        // TODO: save-to-mega can't really handle hls videos either way it seems like. webm works.
        // TODO: Don't use og:image for libreddit since it's only a thumb.
        // TODO: Think about abstracting the common parts if there are more cases to handle in the future.
        if url.to_string().contains("nitter") {
            let selector = Selector::parse(r#"meta[property="og:image"]"#)
                .or_else(|e| Err(Error::msg(format!("Could not parse selector {e:?}"))))?;
            let image_links = html.select(&selector).map(|element_ref| {
                element_ref
                    .value()
                    .attr("content")
                    .map(|s| s.into())
                    .ok_or_else(|| Error::msg("Missing content attribute for og:image property"))
            });
            Ok(image_links.collect())
        } else {
            let single_image_selector = Selector::parse(r#".post_media_image"#)
                .or_else(|e| Err(Error::msg(format!("Could not parse selector {e:?}"))))?;
            let multiple_image_selector = Selector::parse(r#".gallery a"#)
                .or_else(|e| Err(Error::msg(format!("Could not parse selector {e:?}"))))?;

            let image_links = html
                .select(&single_image_selector)
                .chain(html.select(&multiple_image_selector))
                .map(|element_ref| {
                    element_ref
                        .value()
                        .attr("href")
                        .map(|href| format!("https://libredd.it{href}"))
                        // TODO: Use a custom website to browse images that can handle image urls with query parameters.
                        .map(|href| {
                            if href.contains("preview/external-pre") {
                                let url = Url::try_from(href.as_str()).unwrap();
                                let query =
                                    url.query().map(|q| base64::encode(q)).unwrap_or_default();
                                let path = url.path();
                                format!("https://feedragon.privacy.qvarford.net/libreddit/ep/{query}{path}")
                            } else {
                                href
                            }
                        })
                        .map(|href| href.replace("preview/pre", "img"))
                        .ok_or_else(|| {
                            Error::msg("Missing content attribute for og:image property")
                        })
                });
            Ok(image_links.collect())
        }
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
            let page = self
                .hash_map
                .get(url.as_str())
                .ok_or(anyhow::Error::msg("Could not download url"))?;
            bytes(&format!("./src/res/static/pages/{}.html", page.0))
        }
    }

    fn bytes(filename: &str) -> Result<Bytes> {
        let bytes = std::fs::read(filename)?.into();
        Ok(bytes)
    }

    #[actix_rt::test]
    async fn nitter_single_image_is_extracted() {
        let url = "https://nitter.privacy.qvarford.net/SerebiiNet/status/1554195261253046272";
        let transformer = transformer([(url.into(), Page("nitter_single_image"))].into());
        let feed = feed(vec![url]);
        let expected_url = "https://nitter.privacy.qvarford.net/pic/media%2FFY_ABU8XoAAoLX6.jpg";
        let mut expected_feed = feed.clone();

        let transformed_feed = transformer.extract_images_from_feed(feed).await;

        expected_feed.entries[0].id = expected_url.into();
        expected_feed.entries[0].link = expected_url.into();
        assert_eq!(expected_feed, transformed_feed)
    }

    #[actix_rt::test]
    async fn libreddit_single_image_is_extracted() {
        let url = "https://libredd.it/r/AceAttorneyCirclejerk/comments/weucga/if_you_criticize_the_quality_of_the_judge_images/";
        let transformer = transformer([(url.into(), Page("libreddit_single_image"))].into());
        let feed = feed(vec![url]);
        let expected_url = "https://libredd.it/img/yhra1yqisef91.jpg";
        let mut expected_feed = feed.clone();

        let transformed_feed = transformer.extract_images_from_feed(feed).await;

        expected_feed.entries[0].id = expected_url.into();
        expected_feed.entries[0].link = expected_url.into();
        assert_eq!(expected_feed, transformed_feed)
    }

    #[actix_rt::test]
    async fn libreddit_external_images_are_proxied() {
        let url = "https://www.reddit.com/r/AceAttorneyCirclejerk/comments/wev5mg/facts/";
        let transformer = transformer([(url.into(), Page("libreddit_external_image"))].into());
        let feed = feed(vec![url]);
        let expected_url = "https://feedragon.privacy.qvarford.net/libreddit/ep/YXV0bz13ZWJwJnM9Y2RkMTljYWE3YzBhMGZiYjg1Yzk2OTk1OTVmZTdmYTRhYjliZDkyNQ==/preview/external-pre/89qweeQXjTSRVVKQhcu-mgN2RRQHr7HM1wenvITkRCo.jpg";
        let mut expected_feed = feed.clone();

        let transformed_feed = transformer.extract_images_from_feed(feed).await;

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

        let transformed_feed = transformer.extract_images_from_feed(feed).await;

        expected_feed.entries = vec![];
        assert_eq!(expected_feed, transformed_feed)
    }

    #[actix_rt::test]
    async fn empty_if_the_page_could_not_be_downloaded() {
        let url = "https://nitter.privacy.qvarford.net/jeremysmiles/status/1554270809509748737";
        let transformer = transformer([].into());
        let feed = feed(vec![url]);
        let mut expected_feed = feed.clone();

        let transformed_feed = transformer.extract_images_from_feed(feed).await;

        expected_feed.entries = vec![];
        assert_eq!(expected_feed, transformed_feed)
    }

    #[actix_rt::test]
    async fn empty_if_the_page_was_empty() {
        let url = "https://nitter.privacy.qvarford.net/jeremysmiles/status/1554270809509748737";
        let transformer = transformer([(url.into(), Page("empty"))].into());
        let feed = feed(vec![url]);
        let mut expected_feed = feed.clone();

        let transformed_feed = transformer.extract_images_from_feed(feed).await;

        expected_feed.entries = vec![];
        assert_eq!(expected_feed, transformed_feed)
    }

    #[actix_rt::test]
    async fn images_from_second_entry_is_extracted_even_if_first_entry_is_invalid() {
        let url = "https://nitter.privacy.qvarford.net/SerebiiNet/status/1554195261253046272";
        let transformer = transformer([(url.into(), Page("nitter_single_image"))].into());
        let feed = feed(vec!["https://invalid.com", url]);
        let expected_url = "https://nitter.privacy.qvarford.net/pic/media%2FFY_ABU8XoAAoLX6.jpg";
        let mut expected_feed = feed.clone();

        let transformed_feed = transformer.extract_images_from_feed(feed).await;

        expected_feed.entries.remove(0);
        expected_feed.entries[0].id = expected_url.into();
        expected_feed.entries[0].link = expected_url.into();
        assert_eq!(expected_feed, transformed_feed)
    }

    #[actix_rt::test]
    async fn nitter_multiple_images_can_be_extracted() {
        let url = "https://nitter.privacy.qvarford.net/SerebiiNet/status/1554709371459981313";
        let transformer = transformer([(url.into(), Page("nitter_two_images"))].into());
        let feed = feed(vec![url]);
        let expected_url1 = "https://nitter.privacy.qvarford.net/pic/media%2FFZNv5wmXgAE5UOB.jpg";
        let expected_url2 = "https://nitter.privacy.qvarford.net/pic/media%2FFZNv6siWQAQ6A0t.jpg";
        let mut expected_feed = feed.clone();

        let transformed_feed = transformer.extract_images_from_feed(feed).await;

        expected_feed.entries.push(expected_feed.entries[0].clone());
        expected_feed.entries[0].id = expected_url1.into();
        expected_feed.entries[0].link = expected_url1.into();
        expected_feed.entries[1].id = expected_url2.into();
        expected_feed.entries[1].link = expected_url2.into();
        assert_eq!(expected_feed, transformed_feed)
    }

    #[actix_rt::test]
    async fn libreddit_multiple_images_can_be_extracted() {
        let url = "https://libredd.it/r/AnarchyChess/comments/wf2945/new_response_just_dropped/";
        let transformer = transformer([(url.into(), Page("libreddit_two_images"))].into());
        let feed = feed(vec![url]);
        let expected_url1 = "https://libredd.it/img/0kv9uvdzygf91.png";
        let expected_url2 = "https://libredd.it/img/cvu0eyozygf91.png";
        let mut expected_feed = feed.clone();

        let transformed_feed = transformer.extract_images_from_feed(feed).await;

        expected_feed.entries.push(expected_feed.entries[0].clone());
        expected_feed.entries[0].id = expected_url1.into();
        expected_feed.entries[0].link = expected_url1.into();
        expected_feed.entries[1].id = expected_url2.into();
        expected_feed.entries[1].link = expected_url2.into();
        assert_eq!(expected_feed, transformed_feed)
    }

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
