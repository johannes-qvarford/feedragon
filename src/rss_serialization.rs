use crate::model::Entry;
use crate::model::Feed;
use crate::serialization::{invalid_xml_structure, FeedDeserializer};
use anyhow::{Context, Result};
use chrono::DateTime;
use url::Url;
use yaserde::de::from_reader;
use yaserde_derive::YaDeserialize;

pub struct RssDeserializer {}

#[derive(YaDeserialize, Default, Debug, PartialEq)]
#[yaserde(namespace = "atom: http://www.w3.org/2005/Atom", root = "rss")]
struct Rss {
    channel: Channel,
}

#[derive(YaDeserialize, Default, Debug, PartialEq)]
#[yaserde(namespace = "atom: http://www.w3.org/2005/Atom")]
struct Channel {
    #[yaserde(prefix = "atom", rename = "link")]
    link: Vec<Link>,
    title: String,
    #[yaserde(rename = "item")]
    items: Vec<Item>,
}

#[derive(YaDeserialize, Default, Debug, PartialEq)]
struct Item {
    #[yaserde(rename = "guid")]
    id: String,
    link: String,
    description: String,
    title: String,
    #[yaserde(rename = "pubDate")]
    updated: String,
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
    #[yaserde(attribute, rename = "type")]
    link_type: String,
}

impl FeedDeserializer for RssDeserializer {
    fn parse_feed_from_bytes(&self, bytes: &[u8]) -> Result<Feed> {
        let mut rss: Rss = from_reader(bytes).map_err(invalid_xml_structure)?;

        // Even though we require that the link element should have the atom namespace, the regular rss link element is still included.
        // We therefor have to find the actual atom link.
        let href = &rss
            .channel
            .link
            .iter()
            .find(|li| li.link_type == "application/rss+xml")
            .ok_or_else(|| {
                invalid_xml_structure("Could not find self-referencial link in rss feed".into())
            })?
            .href;
        let link = Url::parse(href)
            .map_err(|err| invalid_xml_structure(format!("Invalid url {}", err)))?;

        let items = std::mem::replace(&mut rss.channel.items, vec![]);
        let entry_results: Vec<Result<Entry>> = items
            .into_iter()
            .map(|it| {
                Ok(Entry {
                    id: it.id,
                    link: it.link,
                    summary: it.description,
                    title: it.title,
                    updated: DateTime::parse_from_rfc2822(&it.updated)
                        .map_err(|_dt_err| {
                            invalid_xml_structure(format!("Invalid rss date time: {}", _dt_err))
                        })?
                        .into(),
                })
            })
            .collect();
        let mut entries = vec![];
        for e in entry_results {
            entries.push(e.context("Failed to deserialize an atom feed entry")?);
        }

        Ok(Feed {
            author_name: "Unknown".into(),
            id: rss.channel.title.clone(),
            entries: entries,
            link: link,
            title: rss.channel.title,
        })
    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;
    use chrono::DateTime;

    #[test]
    fn feed_with_no_entries_can_be_parsed() {
        let feed_str = std::fs::read_to_string("src/res/example_empty_rss_feed.xml")
            .expect("Expected example file to exist.");
        let parser = RssDeserializer {};

        let feed = parser.parse_feed_from_bytes(feed_str.as_bytes()).unwrap();

        let expected = Feed {
            author_name: "Unknown".into(),
            entries: vec![],
            id: "Hard Drive / @HardDriveMag".into(),
            link: "https://nitter.net/HardDriveMag/rss".try_into().unwrap(),
            title: "Hard Drive / @HardDriveMag".into(),
        };
        assert_eq!(expected, feed);
    }

    #[test]
    fn feed_with_one_entry_can_be_parsed() {
        let feed_str = std::fs::read_to_string("src/res/example_one_element_rss_feed.xml")
            .expect("Expected example file to exist.");
        let parser = RssDeserializer {};

        let feed = parser.parse_feed_from_bytes(feed_str.as_bytes()).unwrap();

        let expected = Feed {
            author_name: "Unknown".into(),
            entries: vec![
                Entry {
                    title: "messing around in photoshop on twitch. not sure for how long. maybe 15 minutes. maybe 24 hours. probably not 24 hours though http://twitch.tv/harddrivenews".into(),
                    id: "https://nitter.net/HardDriveMag/status/1512602002425004039#m".into(),
                    link: "https://nitter.net/HardDriveMag/status/1512602002425004039#m".into(),
                    summary: r##"<p>messing around in photoshop on twitch. not sure for how long. maybe 15 minutes. maybe 24 hours. probably not 24 hours though <a href="http://twitch.tv/harddrivenews">twitch.tv/harddrivenews</a></p><img src="https://nitter.net/pic/media%2FFP3Wqt-XMAQ7IIK.png" style="max-width:250px;" />"##.into(),
                    updated: DateTime::parse_from_rfc3339("2022-04-09T01:23:14+00:00").unwrap().into()
                }
            ],
            id: "Hard Drive / @HardDriveMag".into(),
            link: "https://nitter.net/HardDriveMag/rss".try_into().unwrap(),
            title: "Hard Drive / @HardDriveMag".into()
        };

        assert_eq!(expected, feed);
    }
}
