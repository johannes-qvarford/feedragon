use crate::write_element_to_string;
use std::collections::HashMap;
use xmltree::XMLNode;
use std::collections::BTreeMap;
use xmltree::Namespace;
use xmltree::Element;
use url::Url;
use chrono::prelude::*;
use derive_more::{Display};
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
    fn parse_feed(&self, tree: Element) -> Result<Feed, ParsingError> {
        let s = write_element_to_string(&tree, "random")
            .map_err(|e| ParsingError::InvalidXmlStructure(e.to_string()))?;
        self.parse_feed_from_bytes(s.as_bytes())
    }
    fn parse_feed_from_bytes(&self, bytes: &[u8]) -> Result<Feed, ParsingError> {
        self.parse_feed(xmltree::Element::parse(bytes)
            .map_err(|e| ParsingError::InvalidXmlStructure(e.to_string()))?)
    }
}

impl Feed {
    pub fn serialize(mut self) -> Element {
        let mut feed_children = vec![
            Feed::text_element("id".into(), std::mem::replace(&mut self.id, String::new())),
            Feed::link(&self.link.as_str()),
        ];
        let entries = std::mem::replace(&mut self.entries, vec![]);
        feed_children.append(&mut Feed::serialize_entries(entries));
        let root = Element {
            name: "feed".into(),
            // xmlns="http://www.w3.org/2005/Atom" xml:lang="en-US"
            namespaces: Some(Namespace(BTreeMap::from([
                ("".into(), "http://www.w3.org/2005/Atom".into())
            ]))),
            namespace: Some("http://www.w3.org/2005/Atom".into()),
            prefix: None,
            attributes: [("xml:lang".into(), "en-US".into())].into(),
            children: feed_children
        };
        root
    }

    fn serialize_entries(entries: Vec<Entry>) -> Vec<XMLNode> {
        entries.into_iter().map(|mut entry| XMLNode::Element(Element {
            name: "entry".into(),
            namespaces: Some(Namespace(BTreeMap::from([
                ("".into(), "http://www.w3.org/2005/Atom".into())
            ]))),
            namespace: Some("http://www.w3.org/2005/Atom".into()),
            prefix: None,
            attributes: HashMap::new(),
            children: vec![
                Feed::text_element("title".into(), std::mem::replace(&mut entry.title, String::new())),
                Feed::text_element("id".into(), std::mem::replace(&mut entry.id, String::new())),
                Feed::link(&entry.link),
                Feed::text_element("updated".into(), entry.updated.to_rfc3339())
            ]
        })).collect()
    }

    fn text_element(name: String, text: String) -> XMLNode {
        XMLNode::Element(Element {
            prefix: None,
            name: name,
            namespaces: Some(Namespace(BTreeMap::from([
                ("".into(), "http://www.w3.org/2005/Atom".into())
            ]))),
            namespace: Some("http://www.w3.org/2005/Atom".into()),
            attributes: HashMap::new(),
            children: vec![
                XMLNode::Text(text)
            ]
        })
    }

    fn link(url: &str) -> XMLNode {
        XMLNode::Element(Element {
            prefix: None,
            name: "link".into(),
            namespaces: Some(Namespace(BTreeMap::from([
                ("".into(), "http://www.w3.org/2005/Atom".into())
            ]))),
            namespace: Some("http://www.w3.org/2005/Atom".into()),
            attributes: [("href".into(), url.into())].into(),
            children: vec![]
        })
    }
}