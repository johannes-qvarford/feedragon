use anyhow::Result;
use anyhow::*;

use super::atom::{AtomEntry, AtomEntryLink, AtomFeed, AtomLink};
use super::Feed;

pub fn invalid_xml_structure(s: String) -> Error {
    Error::msg(format!("Invalid xml structure: {}", s))
}

pub trait FeedDeserializer: Send + Sync {
    fn parse_feed_from_bytes(&self, bytes: &[u8]) -> Result<Feed>;
}

impl Feed {
    pub fn serialize_to_string(self) -> Result<String> {
        let feed = AtomFeed {
            title: self.title,
            links: vec![AtomLink {
                link_type: "application/atom+xml".into(),
                rel: "self".into(),
                href: self.link.as_str().into(),
            }],
            entries: self
                .entries
                .into_iter()
                .map(|e| AtomEntry {
                    id: e.id,
                    link: AtomEntryLink {
                        rel: "alternate".into(),
                        href: e.link,
                    },
                    title: e.title,
                    updated: e.updated.format("%+").to_string(),
                })
                .collect(),
            updated: chrono::Utc::now().format("%+").to_string(),
        };

        let yaserde_cfg = yaserde::ser::Config {
            perform_indent: true,
            ..Default::default()
        };

        Ok(yaserde::ser::to_string_with_config(&feed, &yaserde_cfg)
            .map_err(Error::msg)
            .with_context(|| {
                format!("Failed to serialize feed to string: {}", feed.title.clone())
            })?)
    }
}
