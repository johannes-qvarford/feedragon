use anyhow::{Context, Result};
use chrono::prelude::*;
use url::Url;
use yaserde::de::from_reader;

use super::{
    atom::AtomFeed, model::Entry, serialization::invalid_xml_structure, Feed, FeedDeserializer,
};

pub struct AtomDeserializer;

impl FeedDeserializer for AtomDeserializer {
    fn parse_feed_from_bytes(&self, bytes: &[u8]) -> Result<Feed> {
        let mut feed: AtomFeed = from_reader(bytes).map_err(invalid_xml_structure)?;

        let href = &feed
            .links
            .iter()
            .find(|li| li.link_type == "application/atom+xml")
            .ok_or_else(|| {
                invalid_xml_structure(
                    "Could not find link.type='application/atom+xml' in atom feed".into(),
                )
            })?
            .href;
        let link = Url::parse(href).map_err(|err| {
            invalid_xml_structure(format!("Failed to parse 'link.href' in atom feed: {}", err))
        })?;

        let entries = std::mem::replace(&mut feed.entries, vec![]);
        let entry_results: Vec<Result<Entry>> = entries
            .into_iter()
            .map(|ae| {
                let updated = DateTime::parse_from_rfc3339(&ae.updated).map_err(|e| {
                    invalid_xml_structure(format!(
                        "Failed to parse 'updated' element: {}",
                        e.to_string()
                    ))
                })?;
                let e = Entry {
                    id: ae.link.href.clone(),
                    summary: ae.title.clone(),
                    link: ae.link.href,
                    title: ae.title,
                    updated: updated.into(),
                };
                Ok(e)
            })
            .collect();
        let mut entries = vec![];
        for e in entry_results {
            entries.push(e.context("Failed to deserialize an atom feed entry")?);
        }

        Ok(Feed {
            author_name: "Unknown".into(),
            id: feed.title.clone(),
            link: link,
            title: feed.title,
            entries: entries,
        })
    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn feed_with_no_entries_can_be_parsed() {
        let feed_str = std::fs::read_to_string("src/res/example_empty_atom_feed.xml")
            .expect("Expected example file to exist.");
        let deserializer = AtomDeserializer {};

        let feed = deserializer
            .parse_feed_from_bytes(feed_str.as_bytes())
            .unwrap();
        let expected = Feed {
            author_name: "Unknown".into(),
            entries: vec![],
            id: "Example feed".into(),
            link: "https://invidious.privacy.qvarford.net/feed/private?token=something"
                .try_into()
                .unwrap(),
            title: "Example feed".into(),
        };
        assert_eq!(expected, feed);
    }

    #[test]
    fn feed_with_one_entry_can_be_parsed() {
        let feed_str = std::fs::read_to_string("src/res/example_one_element_atom_feed.xml")
            .expect("Expected example file to exist.");
        let deserializer = AtomDeserializer {};

        let feed = deserializer
            .parse_feed_from_bytes(feed_str.as_bytes())
            .unwrap();

        let expected = Feed {
            author_name: "Unknown".into(),
            entries: vec![Entry {
                title: String::from("SmallAnt makes a ✨𝘧𝘳𝘪𝘦𝘯𝘥✨"),
                id: "http://invidious.privacy.qvarford.net/watch?v=be8ZARHsjmc".into(),
                link: "http://invidious.privacy.qvarford.net/watch?v=be8ZARHsjmc"
                    .parse()
                    .unwrap(),
                summary: String::from("SmallAnt makes a ✨𝘧𝘳𝘪𝘦𝘯𝘥✨"),
                updated: DateTime::parse_from_rfc3339("2022-03-22T07:26:01+00:00")
                    .unwrap()
                    .into(),
            }],
            id: "Example feed".into(),
            link: "https://invidious.privacy.qvarford.net/feed/private?token=something"
                .try_into()
                .unwrap(),
            title: "Example feed".into(),
        };
        assert_eq!(expected, feed);
    }
}
