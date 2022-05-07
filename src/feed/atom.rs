use derive_alias::derive_alias;
use yaserde_derive::YaDeserialize;
use yaserde_derive::YaSerialize;

derive_alias! {
    derive_item => #[derive(YaDeserialize, YaSerialize, Default, Debug, PartialEq)]
}

derive_item! {
    #[yaserde(
        prefix = "ns",
        namespace = "ns: http://www.w3.org/2005/Atom"
        root = "feed"
        rename = "feed"
    )]
    pub struct AtomFeed {
        #[yaserde(rename="link", prefix="ns")]
        pub links: Vec<AtomLink>,
        #[yaserde(prefix="ns")]
        pub title: String,
        #[yaserde(prefix="ns")]
        pub updated: String,
        #[yaserde(rename="entry", prefix="ns")]
        pub entries: Vec<AtomEntry>,
    }
}

derive_item! {
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
        pub link: AtomLink,
        #[yaserde(prefix="ns")]
        pub updated: String,
    }
}

derive_item! {
    #[yaserde(
        prefix = "ns",
        namespace = "ns: http://www.w3.org/2005/Atom"
    )]
    pub struct AtomLink {
        #[yaserde(attribute, prefix="ns")]
        pub href: String,
        #[yaserde(attribute, prefix="ns")]
        pub rel: String,
        #[yaserde(attribute, rename="type", prefix="ns")]
        pub link_type: String
    }
}
