use crate::{
    ccusage::{
        cache,
        models::{AppStatus, Diagnostics, UsageRequest, UsageResponse},
        parser, runner,
    },
    errors::{ApiError, AppError},
    settings::{self, SettingsDto, SettingsPatch},
    state::AppState,
};
use chrono::Local;
use tauri::State;

#[tauri::command]
pub async fn get_status(state: State<'_, AppState>) -> Result<AppStatus, ApiError> {
    let settings = read_settings(&state)?;
    let resolved = runner::resolve_ccusage_path(&settings);
    let version = match &resolved {
        Some(path) => runner::ccusage_version(path).await.ok(),
        None => None,
    };

    Ok(AppStatus {
        ccusage_found: resolved.is_some(),
        ccusage_path: resolved.map(|path| path.to_string_lossy().to_string()),
        ccusage_version: version,
        settings_path: settings::settings_path(&state.config_dir)
            .to_string_lossy()
            .to_string(),
        cache_path: cache::cache_path(&state.cache_dir)
            .to_string_lossy()
            .to_string(),
    })
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<SettingsDto, ApiError> {
    Ok(read_settings(&state)?.into())
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> Result<SettingsDto, ApiError> {
    let saved = {
        let mut guard = state.settings.write().map_err(lock_error)?;
        settings::apply_patch(&mut guard, patch).map_err(ApiError::from)?;
        settings::save_settings(&state.config_dir, &guard).map_err(ApiError::from)?;
        guard.clone()
    };
    Ok(saved.into())
}

#[tauri::command]
pub async fn query_usage(
    state: State<'_, AppState>,
    request: UsageRequest,
) -> Result<UsageResponse, ApiError> {
    let settings = read_settings(&state)?;
    let normalized = request
        .normalize(&settings, Local::now().date_naive())
        .map_err(ApiError::from)?;
    let key = cache::cache_key(&normalized, &settings);

    if !normalized.force_refresh {
        if let Some(entry) = state
            .cache
            .read()
            .map_err(lock_error)?
            .get(&key)
            .filter(|entry| entry.is_fresh(settings.cache_ttl_seconds))
            .cloned()
        {
            return Ok(entry.response.with_cache_flags(true, false, None));
        }

        if let Ok(Some(entry)) = cache::load_cache(&state.cache_dir) {
            if entry.key == key && entry.is_fresh(settings.cache_ttl_seconds) {
                state
                    .cache
                    .write()
                    .map_err(lock_error)?
                    .insert(key.clone(), entry.clone());
                return Ok(entry.response.with_cache_flags(true, false, None));
            }
        }
    }

    match runner::run_daily_report(&settings, &normalized).await {
        Ok(run) => {
            let mut response = parser::parse_usage_json(&run.stdout).map_err(ApiError::from)?;
            response.ccusage_version = run.ccusage_version;
            response.command = run.command;
            response.last_refreshed = chrono::Utc::now().to_rfc3339();
            response.generated_at = response.last_refreshed.clone();
            response.from_cache = false;
            response.stale = false;

            let entry = cache::CachedEntry::new(key.clone(), response.clone());
            state
                .cache
                .write()
                .map_err(lock_error)?
                .insert(key, entry.clone());
            cache::save_cache(&state.cache_dir, &entry).map_err(ApiError::from)?;
            Ok(response)
        }
        Err(error) => {
            if let Some(entry) = state
                .cache
                .read()
                .map_err(lock_error)?
                .get(&key)
                .cloned()
                .or_else(|| cache::load_cache(&state.cache_dir).ok().flatten())
            {
                let warning = ApiError::from(error);
                return Ok(entry.response.with_cache_flags(true, true, Some(warning)));
            }
            Err(ApiError::from(error))
        }
    }
}

#[tauri::command]
pub async fn clear_cache(state: State<'_, AppState>) -> Result<(), ApiError> {
    state.cache.write().map_err(lock_error)?.clear();
    cache::clear_cache(&state.cache_dir).map_err(ApiError::from)
}

#[tauri::command]
pub async fn run_diagnostics(state: State<'_, AppState>) -> Result<Diagnostics, ApiError> {
    let status = get_status(state.clone()).await?;
    let settings = read_settings(&state)?;
    let normalized = UsageRequest::default()
        .normalize(&settings, Local::now().date_naive())
        .map_err(ApiError::from)?;
    let command = runner::build_daily_args(&settings, &normalized);

    let (stdout_excerpt, stderr_excerpt, error) =
        match runner::run_daily_report(&settings, &normalized).await {
            Ok(run) => (
                Some(runner::truncate(&run.stdout, 800)),
                run.stderr.map(|stderr| runner::truncate(&stderr, 800)),
                None,
            ),
            Err(err) => (None, None, Some(ApiError::from(err))),
        };

    Ok(Diagnostics {
        status,
        command,
        stdout_excerpt,
        stderr_excerpt,
        error,
    })
}

fn read_settings(state: &State<'_, AppState>) -> Result<crate::settings::Settings, ApiError> {
    state
        .settings
        .read()
        .map_err(lock_error)
        .map(|guard| guard.clone())
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> ApiError {
    ApiError::from(AppError::Io {
        details: "internal state lock was poisoned".to_string(),
    })
}
