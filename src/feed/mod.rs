pub mod atom;
pub mod atom_serialization;
pub mod fallback_serialization;
pub mod model;
pub mod rss_serialization;
pub mod serialization;

use self::atom_serialization::AtomDeserializer;
use self::fallback_serialization::FallbackDeserializer;
use self::rss_serialization::RssDeserializer;

pub use self::model::{merge_feeds, Feed};
pub use self::serialization::FeedDeserializer;

pub fn default_feed_deserializer() -> impl FeedDeserializer {
    FallbackDeserializer::new(vec![
        Box::new(RssDeserializer {}),
        Box::new(AtomDeserializer {}),
    ])
}
