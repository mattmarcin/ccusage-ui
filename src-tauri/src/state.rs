use crate::{ccusage::cache::CachedEntry, settings::Settings};
use std::{collections::HashMap, path::PathBuf, sync::RwLock};

pub struct AppState {
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub settings: RwLock<Settings>,
    pub cache: RwLock<HashMap<String, CachedEntry>>,
}

impl AppState {
    pub fn new(config_dir: PathBuf, cache_dir: PathBuf, settings: Settings) -> Self {
        Self {
            config_dir,
            cache_dir,
            settings: RwLock::new(settings),
            cache: RwLock::new(HashMap::new()),
        }
    }
}
