use crate::parsing::*;
use chrono::prelude::*;
use crate::xml_tree::*;
use xmltree::{Element};
use std::borrow::Borrow;


pub struct AtomParser;

impl Parser for AtomParser {
    fn parse_feed(&self, tree: Element) -> Result<Feed, ParsingError> {
        let tree: ElementContext = (&tree).into();
        let entry_results: Vec<Result<Entry, ParsingError>> = tree.elements("entry")
            .map(|ec| self.parse_entry(&ec)).value_take();

        let mut entries = vec![];
        for e in entry_results {
            entries.push(e?);
        }

        Ok(Feed {
            author_name: "Unknown".into(),
            entries: entries,
            id: tree.element("title")?.text()?.value_ref().to_string(),
            link: tree.elements("link")
                .find_with_attribute_value("rel", "self")
                .into_parsing_result()?
                .attribute("href")?
                .try_into()?,
            title: tree.element("title")?.text()?.value_ref().to_string()
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

impl AtomParser {
    fn parse_entry(&self, atom_entry: &ElementContext) -> Result<Entry, ParsingError> {
        let extract_text = |id: &str| -> Result<String, ParsingError> {
             Ok(atom_entry.element(id)?.text()?.value_ref().to_string())
        };

        let extract_attribute = |id: &str, attribute_name: &str| -> Result<String, ParsingError> {
            Ok(atom_entry.element(id)?.attribute(attribute_name)?.value_ref().to_string())
        };

        let extract_date_time = |id: &str| -> Result<DateTime<Utc>, ParsingError> {
            let element = atom_entry.element(id)?;
            let text = element.text()?;
            let date_time_with_offset = DateTime::parse_from_rfc3339(text.value_ref().borrow())
                .map_err(|_dt_err| text.invalid_xml_structure("Invalid rfc 3339 date time"))?;
            return Ok(DateTime::from(date_time_with_offset))
        };

        Ok(Entry {
            title: extract_text("title")?,
            id: extract_text("id")?,
            link: extract_attribute("link", "href")?,
            summary: extract_text("title")?,
            updated: extract_date_time("updated")?
        })
    }
}