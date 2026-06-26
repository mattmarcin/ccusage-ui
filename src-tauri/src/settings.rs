use crate::errors::AppError;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub ccusage_path: Option<PathBuf>,
    pub claude_config_dirs: Option<String>,
    pub timezone: Option<String>,
    pub cache_ttl_seconds: u64,
    pub offline: bool,
    pub auto_refresh_seconds: Option<u64>,
    pub include_raw_json: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ccusage_path: None,
            claude_config_dirs: None,
            timezone: None,
            cache_ttl_seconds: 300,
            offline: true,
            auto_refresh_seconds: None,
            include_raw_json: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub ccusage_path: Option<String>,
    pub claude_config_dirs: Option<String>,
    pub timezone: Option<String>,
    pub cache_ttl_seconds: u64,
    pub offline: bool,
    pub auto_refresh_seconds: Option<u64>,
    pub include_raw_json: bool,
}

impl From<Settings> for SettingsDto {
    fn from(value: Settings) -> Self {
        Self {
            ccusage_path: value
                .ccusage_path
                .map(|path| path.to_string_lossy().to_string()),
            claude_config_dirs: value.claude_config_dirs,
            timezone: value.timezone,
            cache_ttl_seconds: value.cache_ttl_seconds,
            offline: value.offline,
            auto_refresh_seconds: value.auto_refresh_seconds,
            include_raw_json: value.include_raw_json,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    #[serde(default)]
    pub ccusage_path: Option<Option<String>>,
    #[serde(default)]
    pub claude_config_dirs: Option<Option<String>>,
    #[serde(default)]
    pub timezone: Option<Option<String>>,
    #[serde(default)]
    pub cache_ttl_seconds: Option<u64>,
    #[serde(default)]
    pub offline: Option<bool>,
    #[serde(default)]
    pub auto_refresh_seconds: Option<Option<u64>>,
    #[serde(default)]
    pub include_raw_json: Option<bool>,
}

pub fn settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join(SETTINGS_FILE)
}

pub fn load_settings(config_dir: &Path) -> Result<Settings, AppError> {
    let path = settings_path(config_dir);
    if !path.exists() {
        return Ok(Settings::default());
    }

    let text = fs::read_to_string(&path).map_err(|err| AppError::Settings {
        details: format!("{}: {}", path.display(), err),
    })?;
    serde_json::from_str(&text).map_err(|err| AppError::Settings {
        details: format!("{}: {}", path.display(), err),
    })
}

pub fn save_settings(config_dir: &Path, settings: &Settings) -> Result<(), AppError> {
    fs::create_dir_all(config_dir)?;
    let path = settings_path(config_dir);
    let temp_path = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(settings).map_err(|err| AppError::Settings {
        details: err.to_string(),
    })?;
    fs::write(&temp_path, text)?;
    fs::rename(&temp_path, &path)?;
    Ok(())
}

pub fn apply_patch(settings: &mut Settings, patch: SettingsPatch) -> Result<(), AppError> {
    if let Some(path) = patch.ccusage_path {
        settings.ccusage_path = path.and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| PathBuf::from(trimmed))
        });
    }
    if let Some(dirs) = patch.claude_config_dirs {
        settings.claude_config_dirs = dirs.and_then(|value| {
            let cleaned = value
                .split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(",");
            (!cleaned.is_empty()).then_some(cleaned)
        });
    }
    if let Some(timezone) = patch.timezone {
        settings.timezone = timezone.and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        });
    }
    if let Some(ttl) = patch.cache_ttl_seconds {
        settings.cache_ttl_seconds = ttl.min(86_400);
    }
    if let Some(offline) = patch.offline {
        settings.offline = offline;
    }
    if let Some(auto_refresh) = patch.auto_refresh_seconds {
        settings.auto_refresh_seconds = auto_refresh.filter(|value| *value >= 30);
    }
    if let Some(include_raw_json) = patch.include_raw_json {
        settings.include_raw_json = include_raw_json;
    }

    Ok(())
}
