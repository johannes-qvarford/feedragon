use url::Url;
use crate::Parser;
use crate::parsing;
use crate::xml_tree;
use yaserde_derive::YaDeserialize;
use yaserde::de::from_str;

struct RssParser {}

#[derive(YaDeserialize, Default, Debug, PartialEq)]
#[yaserde(
    namespace = "atom: http://www.w3.org/2005/Atom",
    root = "rss"
)]
struct Rss {
    channel: Channel
}

#[derive(YaDeserialize, Default, Debug, PartialEq)]
#[yaserde(
    namespace = "atom: http://www.w3.org/2005/Atom",
)]
struct Channel {
    #[yaserde(prefix="atom", rename="link")]
    link: Vec<Link>,
    title: String,
    #[yaserde(rename = "item")]
    items: Vec<Item>
}

#[derive(YaDeserialize, Default, Debug, PartialEq)]
struct Item {

}

#[derive(YaDeserialize, Default, Debug, PartialEq)]
#[yaserde(
    prefix = "atom"
    namespace = "atom: http://www.w3.org/2005/Atom",
)]
struct Link {
    #[yaserde(attribute)]
    href: String,
    #[yaserde(attribute)]
    rel: String,
    #[yaserde(attribute, rename="type")]
    r#type: String
}

impl Parser for RssParser {

    fn parse_feed(&self, element: xmltree::Element) -> std::result::Result<parsing::Feed, parsing::ParsingError> {
        let s = xml_tree::write_element_to_string(&element, "random")
            .map_err(|e| parsing::ParsingError::InvalidXmlStructure(e.to_string()))?;
        let f: Rss = from_str(&s).map_err(parsing::ParsingError::InvalidXmlStructure)?;
        println!("{:?}", f);

        // Even though we require that the link element should have the atom namespace, the regular rss link element is still included.
        // We therefor have to find the actual atom link.
        let href = &f.channel.link.iter().find(|li| li.r#type == "application/rss+xml")
            .ok_or_else(|| parsing::ParsingError::InvalidXmlStructure("Could not find self-referencial link in rss feed".into()))?
            .href;
        let link = Url::parse(href).map_err(|err| parsing::ParsingError::InvalidXmlStructure(format!("Invalid url {}", err)))?;

        Ok(parsing::Feed {
            author_name: "Unknown".into(),
            id: f.channel.title.clone(),
            entries: vec![],
            link: link,
            title: f.channel.title
        })
    }
}

#[cfg(test)]
mod parser_tests {
    use crate::parsing::Entry;
    use crate::parsing::Feed;
    use chrono::DateTime;
    use crate::atom_parser::AtomParser;
    use xmltree::Element;
    use super::*;
    use xmltree::XMLNode;

    #[test]
    fn feed_with_no_entries_can_be_parsed() {
        let feed_str = std::fs::read_to_string("src/example_empty_rss_feed.xml")
            .expect("Expected example file to exist.");
        let feed_element = Element::parse(feed_str.as_bytes()).unwrap();
        let parser = RssParser{};
        
        let feed = parser.parse_feed(feed_element);

        let expected = Feed {
            author_name: "Unknown".into(),
            entries: vec![],
            id: "Hard Drive / @HardDriveMag".into(),
            link: "https://nitter.net/HardDriveMag/rss".try_into().unwrap(),
            title: "Hard Drive / @HardDriveMag".into()
        };
        assert_eq!(Ok(expected), feed);
    }

    #[test]
    fn feed_with_one_entry_can_be_parsed() {
        let feed_str = std::fs::read_to_string("src/example_empty_atom_feed.xml")
            .expect("Expected example file to exist.");
        let entry_str = std::fs::read_to_string("src/example_atom_entry.xml")
        .expect("Expected example file to exist.");
        let mut feed_element = Element::parse(feed_str.as_bytes()).unwrap();
        let entry_element = Element::parse(entry_str.as_bytes()).unwrap();
        feed_element.children.push(XMLNode::Element(entry_element));
        let parser = AtomParser{};

        let feed = parser.parse_feed(feed_element);

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