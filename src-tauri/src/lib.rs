mod commands;
mod errors;
mod settings;
mod state;

pub mod ccusage;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let cache_dir = app.path().app_cache_dir()?;
            std::fs::create_dir_all(&config_dir)?;
            std::fs::create_dir_all(&cache_dir)?;
            let settings = settings::load_settings(&config_dir).unwrap_or_default();
            app.manage(AppState::new(config_dir, cache_dir, settings));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::get_settings,
            commands::update_settings,
            commands::query_usage,
            commands::clear_cache,
            commands::run_diagnostics
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
