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
    entries: HashMap<K, RefCell<Option<CacheEntry<V>>>>,
}

impl<V: Clone + Send> TimedCache<String, V> {}

impl<K: Eq + Hash, V: Clone + Send + std::fmt::Debug> TimedCache<K, V> {
    pub fn from_expiration_duration_and_keys<I: Iterator<Item = K>>(
        duration: chrono::Duration,
        category_names: I,
    ) -> TimedCache<K, V> {
        let hash_map = category_names
            .map(|name| (name, RefCell::new(None)))
            .collect();
        TimedCache {
            expiration_duration: duration,
            entries: hash_map,
        }
    }

    pub async fn get_or_compute<F, Fut>(&self, key: K, f: F) -> Result<V>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<V>>,
    {
        let new_entry = self.entries.get(&key).unwrap();

        let mut borrow = new_entry.borrow_mut();
        match borrow.as_ref() {
            Some(entry) => {
                if entry.expiration_date_time < chrono::offset::Utc::now() {
                    Ok(entry.value.clone())
                } else {
                    let result = f().await;
                    if let Ok(new_value) = result {
                        let entry = Some(self.new_cache_entry(new_value.clone()));
                        *borrow = entry;
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
                        let entry = Some(self.new_cache_entry(value.clone()));
                        *borrow = entry;
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
