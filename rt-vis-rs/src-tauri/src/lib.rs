mod simulator;

use rt_config::SchedulerConfig;

use simulator::simulate_plan;

#[tauri::command]
fn load_config(path: &str) -> Result<SchedulerConfig, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    let config: SchedulerConfig =
        toml::from_str(&content).map_err(|e| format!("Failed to parse TOML: {}", e))?;
    Ok(config)
}

#[tauri::command]
fn save_config(path: &str, config: SchedulerConfig) -> Result<(), String> {
    let content =
        toml::to_string(&config).map_err(|e| format!("Failed to serialize TOML: {}", e))?;
    std::fs::write(path, content).map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            simulate_plan
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
