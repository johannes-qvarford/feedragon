use std::collections::HashMap;

use anyhow::{Context, Result};
use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub categories: HashMap<String, Vec<String>>,
}

impl Config {
    pub fn from_toml_str(s: &str) -> Result<Config> {
        let result = toml::from_str::<Config>(&s);
        result.with_context(|| format!("Failed to parse feedragon config {}", s))
    }
}
