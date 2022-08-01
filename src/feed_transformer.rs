use std::rc::Rc;
use std::str;

use crate::{feed::{Feed, model::Entry}, http_client::HttpClient};

use anyhow::{Result, Context, Error};
use futures::{stream, StreamExt, Stream, FutureExt, TryStreamExt};
use reqwest::Url;
use scraper::{Html, Selector};

struct FeedTransformer {
    http_client: Rc<dyn HttpClient>
}

impl FeedTransformer {
    
    pub async fn extract_images_from_feed(&self, feed: Feed) -> Result<Feed> {
        let feed = feed;
        let entry_results = stream::iter(feed.entries)
            .flat_map(|e| self.convert_to_image_entry(e));

        let results: Vec<Result<Vec<Entry>>> = entry_results.collect().await;
        let results: Result<Vec<Vec<Entry>>> = results.into_iter().collect();
        let entries: Vec<Entry> = results.with_context(|| format!("Could not extract images from feed"))?
            .into_iter()
            .flatten()
            .collect();
        
        Ok(Feed {
            author_name: feed.author_name,
            id: feed.id,
            link: feed.link,
            title: feed.title,
            entries
        })
    }

    fn convert_to_image_entry(&self, e: Entry) -> impl Stream<Item = Result<Vec<Entry>>> + '_ {
        let links = self.extract_images_from_page(e.link)
            .into_stream();

        let entries = links
            .map_ok(move |links| 
                links.into_iter()
                    .map(|link| Entry {
                        id: link.to_string(),
                    link: link.to_string(),
                    summary: e.summary.clone(),
                    title: e.title.clone(),
                    updated: e.updated
                    }).collect::<Vec<_>>()
                )
            ;
        entries
    }

    async fn extract_images_from_page(&self, url: String) -> Result<Vec<Url>> {
        let url: Url = Url::try_from(url.as_str()).with_context(|| format!("Invalid link {url} during image extraction."))?;
        let bytes = self.http_client.get_bytes(&url).await?;
        let content: &str = str::from_utf8(&bytes).with_context(|| format!("Page at {url} is not valid utf8"))?;
        let html = Html::parse_document(content);
        
        // TODO: Nitter specific, do different things for libreddit
        let selector = Selector::parse(r#"meta[property="og:image"]"#)
            .or_else(|e| Err(Error::msg(format!("Could not parse selector {e:?}"))))?;
        let image_links = html.select(&selector)
            .map(|element_ref|
                element_ref.value().attr("content")
                    .ok_or_else(|| Error::msg("Missing content attribute for og:image property")));
        let r: Result<Vec<Url>> = image_links
            .map(|s| -> Result<Url> { Url::try_from(s?).with_context(|| format!("Invalid link {url} during image extraction.")) })
            .collect();
        r
    }
}
