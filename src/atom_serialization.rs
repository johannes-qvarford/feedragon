use crate::atom_serialization::DeserializationError::InvalidXmlStructure;
use url::Url;
use yaserde::de::from_reader;
use crate::serialization::*;
use chrono::prelude::*;
use crate::atom::*;

pub struct AtomDeserializer;

impl FeedDeserializer for AtomDeserializer {
    fn parse_feed_from_bytes(&self, bytes: &[u8]) -> Result<Feed, DeserializationError> {
        let mut feed: AtomFeed = from_reader(bytes).map_err(InvalidXmlStructure)?;

        let href = &feed.links.iter().find(|li| li.link_type == "application/atom+xml")
            .ok_or_else(|| InvalidXmlStructure("Could not find self-referencial link in atom feed".into()))?
            .href;
        let link = Url::parse(href).map_err(|err| InvalidXmlStructure(format!("Invalid url {}", err)))?;

        let entries = std::mem::replace(&mut feed.entries, vec![]);
        let entry_results: Vec<Result<Entry, DeserializationError>> = entries.into_iter().map(|ae| {
            let updated = DateTime::parse_from_rfc3339(&ae.updated).map_err(|e| InvalidXmlStructure(e.to_string()))?;
            let e = Entry {
                id: ae.id,
                summary: ae.title.clone(),
                link: ae.link.href,
                title: ae.title,
                updated: updated.into()
            };
            Ok(e)
        }).collect();
        let mut entries = vec![];
        for e in entry_results {
            entries.push(e?);
        }

        Ok(Feed{
            author_name: "Unknown".into(),
            id: feed.title.clone(),
            link: link,
            title: feed.title,
            entries: entries
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
        let deserializer = AtomDeserializer{};
        
        let feed = deserializer.parse_feed_from_bytes(feed_str.as_bytes());
        let expected = Feed {
            author_name: "Unknown".into(),
            entries: vec![],
            id: "Example feed".into(),
            link: "https://invidious.privacy.qvarford.net/feed/private?token=something".try_into().unwrap(),
            title: "Example feed".into()
        };
        assert_eq!(Ok(expected), feed);
    }

    #[test]
    fn feed_with_one_entry_can_be_parsed() {
        let feed_str = std::fs::read_to_string("src/res/example_one_element_atom_feed.xml")
            .expect("Expected example file to exist.");
        let deserializer = AtomDeserializer{};

        let feed = deserializer.parse_feed_from_bytes(feed_str.as_bytes());

        let expected = Feed {
            author_name: "Unknown".into(),
            entries: vec![Entry {
                title: String::from("SmallAnt makes a ✨𝘧𝘳𝘪𝘦𝘯𝘥✨"),
                id: String::from("yt:video:be8ZARHsjmc"),
                link: "http://invidious.privacy.qvarford.net/watch?v=be8ZARHsjmc".parse().unwrap(),
                summary: String::from("SmallAnt makes a ✨𝘧𝘳𝘪𝘦𝘯𝘥✨"),
                updated: DateTime::parse_from_rfc3339("2022-03-22T07:26:01+00:00").unwrap().into(),
            }],
            id: "Example feed".into(),
            link: "https://invidious.privacy.qvarford.net/feed/private?token=something".try_into().unwrap(),
            title: "Example feed".into()
        };
        assert_eq!(Ok(expected), feed);
    }
}
