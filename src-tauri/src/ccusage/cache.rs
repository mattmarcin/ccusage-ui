use crate::{
    ccusage::models::{NormalizedUsageRequest, UsageResponse},
    errors::AppError,
    settings::Settings,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

const CACHE_FILE: &str = "usage_cache.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedEntry {
    pub key: String,
    pub cached_at_epoch: i64,
    pub response: UsageResponse,
}

impl CachedEntry {
    pub fn new(key: String, response: UsageResponse) -> Self {
        Self {
            key,
            cached_at_epoch: Utc::now().timestamp(),
            response,
        }
    }

    pub fn is_fresh(&self, ttl_seconds: u64) -> bool {
        if ttl_seconds == 0 {
            return false;
        }
        let age = Utc::now().timestamp() - self.cached_at_epoch;
        age >= 0 && age <= ttl_seconds as i64
    }
}

pub fn cache_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join(CACHE_FILE)
}

pub fn cache_key(request: &NormalizedUsageRequest, settings: &Settings) -> String {
    format!(
        "daily|since={}|until={}|tz={}|offline={}|path={}",
        request.since.as_deref().unwrap_or("all"),
        request.until.as_deref().unwrap_or("all"),
        request.timezone,
        settings.offline,
        settings
            .ccusage_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|| "auto".to_string())
    )
}

pub fn load_cache(cache_dir: &Path) -> Result<Option<CachedEntry>, AppError> {
    let path = cache_path(cache_dir);
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|err| AppError::Cache {
        details: format!("{}: {}", path.display(), err),
    })?;
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|err| AppError::Cache {
            details: format!("{}: {}", path.display(), err),
        })
}

pub fn save_cache(cache_dir: &Path, entry: &CachedEntry) -> Result<(), AppError> {
    fs::create_dir_all(cache_dir)?;
    let path = cache_path(cache_dir);
    let text = serde_json::to_string_pretty(entry).map_err(|err| AppError::Cache {
        details: err.to_string(),
    })?;
    fs::write(path, text)?;
    Ok(())
}

pub fn clear_cache(cache_dir: &Path) -> Result<(), AppError> {
    let path = cache_path(cache_dir);
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}
