use std::error::Error;
use url::Url;
use chrono::prelude::*;
use derive_more::{Display};
use crate::atom::*;

#[derive(Debug, PartialEq, Clone)]
pub struct Entry {
    pub title: String,
    pub link: String,
    pub id: String,
    pub updated: DateTime<Utc>,
    pub summary: String
}

#[derive(Debug, PartialEq)]
pub struct Feed {
    pub title: String,
    pub link: Url,
    pub author_name: String,
    pub id: String,
    pub entries: Vec<Entry>
}

#[derive(Debug, Eq, PartialEq, Clone, Display)]
pub enum ParsingError {
    InvalidXmlStructure(String)
}

pub trait Parser {
    fn parse_feed_from_bytes(&self, bytes: &[u8]) -> Result<Feed, ParsingError>;
}

impl Feed {
    pub fn serialize_to_string(mut self) -> Result<String, Box<dyn Error>> {
        let feed = AtomFeed {
            title: self.title,
            links: vec![
                Link { r#type: "application/atom+xml".into(), rel: "self".into(), href: self.link.as_str().into() }
            ],
            entries: self.entries.into_iter().map(|e| AtomEntry {
                id: e.id,
                link: Link { rel: "alternate".into(), href: e.link, r#type: "".into() },
                title: e.title,
                updated: e.updated.to_string()
            }).collect()
        };

        let yaserde_cfg = yaserde::ser::Config{
            perform_indent: true,
            .. Default::default()
        };

        Ok(yaserde::ser::to_string_with_config(&feed, &yaserde_cfg)?)
    }
}