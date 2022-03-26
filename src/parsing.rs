use xmltree::Element;
use url::Url;
use chrono::prelude::*;

#[derive(Debug, PartialEq)]
pub struct Entry {
    pub title: String,
    pub link: String,
    pub id: String,
    pub updated: DateTime<Utc>,
    pub summary: String
}

pub struct Feed {
    title: String,
    link: Url,
    updated: DateTime<Utc>,
    author_name: String,
    id: String,
    entries: Vec<Entry>
}

#[derive(Debug)]
pub enum ParsingError {
    InvalidXmlStructure(String)
}

pub trait Parser {
    fn parse_feed(tree: Element) -> Result<Feed, ParsingError>;
}