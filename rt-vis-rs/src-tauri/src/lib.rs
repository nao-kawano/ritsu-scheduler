// Copyright 2026 Naoyuki Kawano
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// =============================================================================

use rt_config::SchedulerConfig;

mod log_parser;
mod log_store;
mod simulator;
mod stats;
mod types;

use log_store::LogStore;
use simulator::simulate_plan;
use types::{AnomalyEventsPage, ClientBudgetStats, InstantEventType, LogRangeData, LogSummary};

use std::fs::File;
use std::io::BufReader;
use std::sync::Mutex;
use tauri::Manager;

/* -------------------------------------------------------------------------- */

/// Session state managed by Tauri runtime across IPC invocations.
pub struct AppState {
    pub log_store: Mutex<Option<LogStore>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            log_store: Mutex::new(None),
        }
    }
}

/// Executes a read-only operation against the currently loaded LogStore.
fn with_loaded_log_store<T>(
    state: &tauri::State<'_, AppState>,
    f: impl FnOnce(&LogStore) -> T,
) -> Result<T, String> {
    let lock = state
        .log_store
        .lock()
        .map_err(|e| format!("Failed to acquire log store lock: {}", e))?;
    let store = lock
        .as_ref()
        .ok_or_else(|| "No log file has been loaded yet".to_string())?;
    Ok(f(store))
}

/* -------------------------------------------------------------------------- */

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn load_config(path: &str) -> Result<SchedulerConfig, String> {
    log::info!("Loading config from: {}", path);
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    let config: SchedulerConfig =
        toml::from_str(&content).map_err(|e| format!("Failed to parse TOML: {}", e))?;
    Ok(config)
}

#[tauri::command]
fn save_config(path: &str, config: SchedulerConfig) -> Result<(), String> {
    log::info!("Saving config to: {}", path);
    let content =
        toml::to_string(&config).map_err(|e| format!("Failed to serialize TOML: {}", e))?;
    std::fs::write(path, content).map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
}

#[tauri::command]
fn load_log(path: &str, state: tauri::State<'_, AppState>) -> Result<LogSummary, String> {
    log::info!("Loading log from: {}", path);
    let file = File::open(path).map_err(|e| format!("Failed to open log file: {}", e))?;
    let reader = BufReader::new(file);
    let store = log_parser::parse_log(reader).map_err(|e| format!("Failed to parse log: {}", e))?;
    let summary = store.summary.clone();

    let mut lock = state
        .log_store
        .lock()
        .map_err(|e| format!("Failed to acquire log store lock: {}", e))?;
    *lock = Some(store);

    Ok(summary)
}

#[tauri::command]
fn get_log_range(
    start_us: u64,
    end_us: u64,
    state: tauri::State<'_, AppState>,
) -> Result<LogRangeData, String> {
    with_loaded_log_store(&state, |store| store.get_range(start_us, end_us))
}

#[tauri::command]
fn get_anomaly_events(
    cid: Option<u16>,
    event_types: Vec<InstantEventType>,
    offset: usize,
    limit: usize,
    state: tauri::State<'_, AppState>,
) -> Result<AnomalyEventsPage, String> {
    with_loaded_log_store(&state, |store| {
        store.get_anomaly_events(cid, &event_types, offset, limit)
    })
}

#[tauri::command]
fn recompute_client_budget(
    cid: u16,
    expected_duration_ms: u32,
    state: tauri::State<'_, AppState>,
) -> Result<Option<ClientBudgetStats>, String> {
    with_loaded_log_store(&state, |store| {
        store.recompute_client_budget(cid, expected_duration_ms)
    })
}

#[cfg(debug_assertions)]
/// Formats log records with standard microsecond timestamp layout:
/// YYYY/MM/DD HH:MM:SS.ffffff [LEVEL] Module - Message
fn format_log(
    w: &mut dyn std::io::Write,
    now: &mut flexi_logger::DeferredNow,
    record: &log::Record,
) -> Result<(), std::io::Error> {
    let target = record.target();
    let module = target.split("::").last().unwrap_or(target);
    write!(
        w,
        "{} [{:5}] {} - {}",
        now.format("%Y/%m/%d %H:%M:%S%.6f"),
        record.level(),
        module,
        record.args()
    )
}

/* -------------------------------------------------------------------------- */

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    let _logger_handle = flexi_logger::Logger::try_with_str("debug")
        .expect("Failed to initialize logger")
        .format(format_log)
        .log_to_stderr()
        .start()
        .expect("Failed to start logger");

    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Workaround: In multi-monitor setups with different DPI scales on Windows,
            // Tauri's initial window size can be scaled incorrectly (double scaling).
            // Forcefully applying the logical size from the config during setup resolves
            // this issue by ensuring the correct scale factor of the current monitor is used.
            // Additionally, the window starts hidden ("visible": false in config) to prevent
            // visual flickering during the resizing process, and is explicitly shown here.
            if let Some(main_window_config) = app.config().app.windows.first() {
                let width = main_window_config.width;
                let height = main_window_config.height;
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_size(tauri::LogicalSize::new(width, height));
                    let _ = window.show();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_version,
            load_config,
            save_config,
            simulate_plan,
            load_log,
            get_log_range,
            get_anomaly_events,
            recompute_client_budget
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
