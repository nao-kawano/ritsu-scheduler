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
mod simulator;

use rt_config::SchedulerConfig;
use tauri::Manager;

use simulator::simulate_plan;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    {
        let env = env_logger::Env::default().default_filter_or("debug");
        env_logger::Builder::from_env(env)
            .format(|buf, record| {
                use std::io::Write;
                let target = record.target();
                let module = target.split("::").last().unwrap_or(target);
                writeln!(
                    buf,
                    "{} [{:5}] {} - {}",
                    chrono::Local::now().format("%Y/%m/%d %H:%M:%S%.6f"),
                    record.level(),
                    module,
                    record.args()
                )
            })
            .init();
    }

    tauri::Builder::default()
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
            load_config,
            save_config,
            simulate_plan
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
