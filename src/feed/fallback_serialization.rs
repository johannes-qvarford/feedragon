use anyhow::Context;

use super::FeedDeserializer;

pub struct FallbackDeserializer {
    fallbacks: Vec<Box<dyn FeedDeserializer>>,
}

impl FallbackDeserializer {
    pub fn new(fallbacks: Vec<Box<dyn FeedDeserializer>>) -> FallbackDeserializer {
        FallbackDeserializer { fallbacks }
    }
}

impl FeedDeserializer for FallbackDeserializer {
    fn parse_feed_from_bytes(&self, bytes: &[u8]) -> anyhow::Result<super::Feed> {
        self.fallbacks
            .iter()
            .fold(
                Err(anyhow::Error::msg("No fallbacks to parse feeds for!")),
                |result, deserializer| {
                    result.or_else(|_| deserializer.parse_feed_from_bytes(bytes))
                },
            )
            .context("All fallbacks failed, returning last error message.")
    }
}
