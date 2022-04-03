use xmltree::Element;
use url::Url;
use chrono::prelude::*;
use derive_more::{Display, Error};

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
    fn parse_feed(&self, tree: Element) -> Result<Feed, ParsingError>;
    fn serialize_feed(&self, feed: Feed) -> Element;
}