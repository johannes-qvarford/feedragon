use std::{collections::HashMap, hash::Hash, sync::Arc};

use anyhow::{Context, Result};
use chrono::Utc;
use futures::Future;
use log::warn;
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct CacheEntry<V: Send> {
    expiration_date_time: chrono::DateTime<Utc>,
    value: V,
}

pub struct TimedCache<K, V: Send> {
    expiration_duration: chrono::Duration,
    entries: Arc<HashMap<K, Mutex<Option<CacheEntry<V>>>>>,
}

impl<V: Clone + Send> TimedCache<String, V> {}

impl<K: Eq + Hash, V: Clone + Send + std::fmt::Debug> TimedCache<K, V> {
    pub fn from_expiration_duration_and_keys<I: Iterator<Item = K>>(
        duration: chrono::Duration,
        category_names: I,
    ) -> TimedCache<K, V> {
        let hash_map = category_names
            .map(|name| (name, Mutex::new(None)))
            .collect();
        TimedCache {
            expiration_duration: duration,
            entries: Arc::new(hash_map),
        }
    }

    pub async fn get_or_compute<F, Fut>(&self, key: K, f: F) -> Result<V>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<V>>,
    {
        let new_entry = self.entries.get(&key).unwrap();
        let entry_copy = (*new_entry.lock().await).clone();

        match entry_copy {
            Some(CacheEntry {
                expiration_date_time,
                value: exisiting_value,
            }) => {
                if expiration_date_time < chrono::offset::Utc::now() {
                    Ok(exisiting_value)
                } else {
                    let result = f().await;
                    if let Ok(new_value) = result {
                        let entry = Some(self.new_cache_entry(new_value.clone()));
                        *(new_entry.lock().await) = entry;
                        Ok(new_value)
                    } else {
                        warn!("Failed to compute a new value after expiration. The previous value was used instead.");
                        Ok(exisiting_value)
                    }
                }
            }
            _ => {
                let result = f().await;

                match &result {
                    Ok(value) => {
                        let entry = Some(self.new_cache_entry(value.clone()));
                        let mut new_entry_lock = new_entry.lock().await;
                        *new_entry_lock = entry;
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
