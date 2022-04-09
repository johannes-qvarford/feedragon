use url::Url;
use yaserde::de::from_reader;
use crate::atom_parser::ParsingError::InvalidXmlStructure;
use crate::parsing::*;
use chrono::prelude::*;
use crate::atom::*;

pub struct AtomParser;

impl Parser for AtomParser {
    fn parse_feed_from_bytes(&self, bytes: &[u8]) -> Result<Feed, ParsingError> {
        let mut f: AtomFeed = from_reader(bytes).map_err(InvalidXmlStructure)?;

        let href = &f.links.iter().find(|li| li.r#type == "application/atom+xml")
        .ok_or_else(|| InvalidXmlStructure("Could not find self-referencial link in atom feed".into()))?
        .href;
        let link = Url::parse(href).map_err(|err| InvalidXmlStructure(format!("Invalid url {}", err)))?;

        let entries = std::mem::replace(&mut f.entries, vec![]);
        let entry_results: Vec<Result<Entry, ParsingError>> = entries.into_iter().map(|ae| {
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
            id: f.title.clone(),
            link: link,
            title: f.title,
            entries: entries
        })
    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn feed_with_no_entries_can_be_parsed() {
        let feed_str = std::fs::read_to_string("src/example_empty_atom_feed.xml")
            .expect("Expected example file to exist.");
        let parser = AtomParser{};
        
        let feed = parser.parse_feed_from_bytes(feed_str.as_bytes());
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
        let feed_str = std::fs::read_to_string("src/example_one_element_atom_feed.xml")
            .expect("Expected example file to exist.");
        let parser = AtomParser{};

        let feed = parser.parse_feed_from_bytes(feed_str.as_bytes());

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
