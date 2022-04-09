use chrono::DateTime;
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
    #[yaserde(rename="guid")]
    id: String,
    link: String,
    description: String,
    title: String,
    #[yaserde(rename="pubDate")]
    updated: String
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
        let mut f: Rss = from_str(&s).map_err(parsing::ParsingError::InvalidXmlStructure)?;

        // Even though we require that the link element should have the atom namespace, the regular rss link element is still included.
        // We therefor have to find the actual atom link.
        let href = &f.channel.link.iter().find(|li| li.r#type == "application/rss+xml")
            .ok_or_else(|| parsing::ParsingError::InvalidXmlStructure("Could not find self-referencial link in rss feed".into()))?
            .href;
        let link = Url::parse(href).map_err(|err| parsing::ParsingError::InvalidXmlStructure(format!("Invalid url {}", err)))?;

        let items = std::mem::replace(&mut f.channel.items, vec![]);
        let entry_results: Vec<Result<_, _>> = items.into_iter().map(|it| {
            let updated: Result<_, parsing::ParsingError> = DateTime::parse_from_rfc2822(&it.updated)
                .map_err(|_dt_err|
                    parsing::ParsingError::InvalidXmlStructure(format!("Invalid rss date time: {}", _dt_err)));

            let e = parsing::Entry {
                id: it.id,
                link: it.link,
                summary: it.description,
                title: it.title,
                // Sat, 09 Apr 2022 01:23:14 GMT
                updated: updated?.into(),
            };
            Ok(e)
        }).collect();
        let mut entries = vec![];
        for e in entry_results {
            entries.push(e?);
        }

        Ok(parsing::Feed {
            author_name: "Unknown".into(),
            id: f.channel.title.clone(),
            entries: entries,
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
        let feed_str = std::fs::read_to_string("src/example_empty_rss_feed.xml")
            .expect("Expected example file to exist.");
        let entry_str = std::fs::read_to_string("src/example_rss_entry.xml")
        .expect("Expected example file to exist.");
        let mut feed_element = Element::parse(feed_str.as_bytes()).unwrap();
        let entry_element = Element::parse(entry_str.as_bytes()).unwrap();
        feed_element.get_mut_child("channel").unwrap().children.push(XMLNode::Element(entry_element));
        let parser = RssParser{};

        let feed = parser.parse_feed(feed_element);

        let expected = Feed {
            author_name: "Unknown".into(),
            entries: vec![
                Entry {
                    title: "messing around in photoshop on twitch. not sure for how long. maybe 15 minutes. maybe 24 hours. probably not 24 hours though http://twitch.tv/harddrivenews".into(),
                    id: "https://nitter.net/HardDriveMag/status/1512602002425004039#m".into(),
                    link: "https://nitter.net/HardDriveMag/status/1512602002425004039#m".into(),
                    summary: r##"<p>messing around in photoshop on twitch. not sure for how long. maybe 15 minutes. maybe 24 hours. probably not 24 hours though <a href="http://twitch.tv/harddrivenews">twitch.tv/harddrivenews</a></p><img src="https://nitter.net/pic/media%2FFP3Wqt-XMAQ7IIK.png" style="max-width:250px;" />"##.into(),
                    // Sat, 09 Apr 2022 01:23:14 GMT
                    updated: DateTime::parse_from_rfc3339("2022-04-09T01:23:14+00:00").unwrap().into()
                }
            ],
            id: "Hard Drive / @HardDriveMag".into(),
            link: "https://nitter.net/HardDriveMag/rss".try_into().unwrap(),
            title: "Hard Drive / @HardDriveMag".into()
        };

        assert_eq!(Ok(expected), feed);
    }
}