pub mod atom;
pub mod atom_serialization;
pub mod model;
pub mod rss_serialization;
pub mod serialization;

use self::rss_serialization::RssDeserializer;

pub use self::model::{merge_feeds, Feed};
pub use self::serialization::FeedDeserializer;

pub fn default_feed_deserializer() -> impl FeedDeserializer {
    RssDeserializer {}
}
