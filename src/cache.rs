use std::{cell::RefCell, collections::HashMap, hash::Hash};

use anyhow::{Context, Result};
use chrono::Utc;
use futures::Future;
use log::warn;

#[derive(Clone, Debug)]
pub struct CacheEntry<V: Send> {
    expiration_date_time: chrono::DateTime<Utc>,
    value: V,
}

pub struct TimedCache<K, V: Send> {
    expiration_duration: chrono::Duration,
    // TODO: Investigate if we need the inner RefCell
    entries: RefCell<HashMap<K, CacheEntry<V>>>,
}

impl<V: Clone + Send> TimedCache<String, V> {}

impl<K: Clone + Eq + Hash + std::fmt::Debug + ToString, V: Clone + Send + std::fmt::Debug>
    TimedCache<K, V>
{
    pub fn from_expiration_duration_and_keys<I: Iterator<Item = K>>(
        duration: chrono::Duration,
        _keys: I,
    ) -> TimedCache<K, V> {
        // TODO: We don't need to pre-fill the map anymore. Also, the size of the map can grow unbounded currently.
        // Will have to remove expired entries on a schedule, and not just their values on request.
        TimedCache {
            expiration_duration: duration,
            entries: RefCell::new(HashMap::new()),
        }
    }

    pub async fn get_or_compute<F, Fut>(&self, key: K, f: F) -> Result<V>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<V>>,
    {
        let entries = self.entries.borrow();
        let value = entries.get(&key).map(|v| v.clone());
        drop(entries);

        match value {
            Some(entry) => {
                if entry.expiration_date_time.timestamp() > chrono::offset::Utc::now().timestamp() {
                    Ok(entry.value.clone())
                } else {
                    let result = f().await;
                    if let Ok(new_value) = result {
                        let entry = self.new_cache_entry(new_value.clone());
                        let mut entries = self.entries.borrow_mut();
                        entries.insert(key.clone(), entry);
                        Ok(new_value)
                    } else {
                        warn!("Failed to compute a new value after expiration. The previous value was used instead.");
                        Ok(entry.value.clone())
                    }
                }
            }
            _ => {
                let result = f().await;

                match &result {
                    Ok(value) => {
                        let entry = self.new_cache_entry(value.clone());
                        let mut entries = self.entries.borrow_mut();
                        entries.insert(key.clone(), entry);
                        result
                    }
                    Err(_) => result.context("Failed to compute a successful response when there was nothing cached to use."),
                }
            }
        }
    }

    fn new_cache_entry(&self, value: V) -> CacheEntry<V> {
        CacheEntry {
            value: value,
            expiration_date_time: chrono::offset::Utc::now()
                .checked_add_signed(self.expiration_duration)
                .unwrap(),
        }
    }
}

#[cfg(test)]
mod test {

    use super::TimedCache;
    use futures::future;
    use url::Url;

    fn url() -> Url {
        "https://google.com".try_into().unwrap()
    }

    fn cache2(duration: chrono::Duration) -> TimedCache<Url, &'static str> {
        TimedCache::from_expiration_duration_and_keys(duration, vec![url()].into_iter())
    }

    fn cache() -> TimedCache<Url, &'static str> {
        cache2(chrono::Duration::hours(1))
    }

    #[actix_rt::test]
    async fn entry_is_computed_if_not_exists() {
        let c = cache();

        let r = c
            .get_or_compute(url(), || future::lazy(|_| Ok("x")))
            .await
            .unwrap();

        assert_eq!("x", r);
    }

    #[actix_rt::test]
    async fn entry_is_not_recomputed_if_it_has_not_expired() {
        let c = cache();

        let _ = c
            .get_or_compute(url(), || future::lazy(|_| Ok("x")))
            .await
            .unwrap();

        let r2 = c
            .get_or_compute(url(), || future::lazy(|_| Ok("y")))
            .await
            .unwrap();

        assert_eq!("x", r2);
    }

    #[actix_rt::test]
    async fn entry_is_recomputed_if_it_has_expired() {
        let c = cache2(chrono::Duration::zero());

        let _ = c
            .get_or_compute(url(), || future::lazy(|_| Ok("x")))
            .await
            .unwrap();

        let r2 = c
            .get_or_compute(url(), || future::lazy(|_| Ok("y")))
            .await
            .unwrap();

        assert_eq!("y", r2);
    }
}
