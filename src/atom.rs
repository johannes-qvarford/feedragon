use yaserde_derive::YaDeserialize;
use yaserde_derive::YaSerialize;

#[derive(YaDeserialize, YaSerialize, Default, Debug, PartialEq)]
#[yaserde(
    prefix = "ns",
    namespace = "ns: http://www.w3.org/2005/Atom"
    root = "feed"
    rename = "feed"
)]
pub struct AtomFeed {
    #[yaserde(rename="link", prefix="ns")]
    pub links: Vec<Link>,
    #[yaserde(prefix="ns")]
    pub title: String,
    #[yaserde(rename="entry", prefix="ns")]
    pub entries: Vec<AtomEntry>,
}

#[derive(YaDeserialize, YaSerialize, Default, Debug, PartialEq)]
#[yaserde(
    prefix = "ns",
    namespace = "ns: http://www.w3.org/2005/Atom"
)]
pub struct AtomEntry {
    #[yaserde(prefix="ns")]
    pub title: String,
    #[yaserde(prefix="ns")]
    pub id: String,
    #[yaserde(prefix="ns")]
    pub link: Link,
    #[yaserde(prefix="ns")]
    pub updated: String,
}

#[derive(YaDeserialize, YaSerialize, Default, Debug, PartialEq)]
#[yaserde(
    prefix = "ns",
    namespace = "ns: http://www.w3.org/2005/Atom"
)]
pub struct Link {
    #[yaserde(attribute, prefix="ns")]
    pub href: String,
    #[yaserde(attribute, prefix="ns")]
    pub rel: String,
    #[yaserde(attribute, rename="type", prefix="ns")]
    pub r#type: String
}