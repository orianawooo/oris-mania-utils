mod config;
mod calc;
mod proxy;
mod keys;

use std::fs;
use std::path::PathBuf;

pub use config::{Config, read_config, load_config_from_disk, write_config, config_path};
pub use calc::{SkillRatings, calculate_map_internal, rebuild_songs_index};
pub use proxy::{start_tosu_proxy, TOSU_CONNECTED};
pub use keys::{start_key_server, update_active_keys};

lazy_static::lazy_static! {
    pub static ref KEY_SENDER: tokio::sync::broadcast::Sender<String> = {
        let (tx, _) = tokio::sync::broadcast::channel(32);
        tx
    };
}

#[tauri::command]
fn get_config() -> Config {
    read_config()
}

#[tauri::command]
fn save_config(config: Config) {
    update_active_keys(&config.keys);
    write_config(&config);
    
    if !config.osu_songs_path.is_empty() {
        let path = config.osu_songs_path.clone();
        std::thread::spawn(move || rebuild_songs_index(&path));
    }
    
    let mut val = serde_json::to_value(&config).unwrap_or_default();
    if let Some(obj) = val.as_object_mut() {
        obj.insert("event".to_string(), serde_json::json!("bindings"));
    }
    let bindings_msg = val.to_string();
    let _ = KEY_SENDER.send(bindings_msg);
}

#[tauri::command]
fn check_tosu(port: u16) -> bool {
    std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok()
}

#[tauri::command]
fn get_default_osu_path() -> String {
    if let Some(local_dir) = dirs_next::data_local_dir() {
        let path = local_dir.join("osu!").join("Songs");
        if path.exists() {
            return path.to_string_lossy().to_string();
        }
    }
    String::new()
}

#[tauri::command]
fn select_folder(app: tauri::AppHandle) -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    
    let _ = app.run_on_main_thread(move || {
        let folder = rfd::FileDialog::new()
            .set_title("Select osu! Songs Folder")
            .pick_folder();
        let _ = tx.send(folder);
    });
    
    let folder = rx.recv().unwrap_or(None);
    folder.map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
fn validate_path(path: String) -> bool {
    let p = std::path::Path::new(&path);
    if !p.exists() || !p.is_dir() {
        return false;
    }
    
    if let Ok(entries) = std::fs::read_dir(p) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Ok(sub_entries) = std::fs::read_dir(path) {
                    for sub_entry in sub_entries.flatten() {
                        if sub_entry.path().extension().map_or(false, |ext| ext == "osu") {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

#[tauri::command]
fn open_url(app: tauri::AppHandle, url: String) {
    use tauri_plugin_opener::OpenerExt;
    let _ = app.opener().open_url(&url, None::<String>);
}

fn log_path() -> PathBuf {
    let base = dirs_next::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("oris-mania-utils").join("debug.log")
}

#[tauri::command]
fn get_log() -> String {
    fs::read_to_string(log_path()).unwrap_or_else(|_| "No log file yet.".to_string())
}

#[tauri::command]
fn clear_log() {
    let _ = fs::write(log_path(), "");
}

#[tauri::command]
fn calculate_map(_app: tauri::AppHandle, osu_folder: String, osu_file: String, md5: Option<String>) -> Result<SkillRatings, String> {
    let config = read_config();
    let md5_str = md5.unwrap_or_default();
    calculate_map_internal(&config.osu_songs_path, &osu_folder, &osu_file, 1.0, &md5_str)
}

#[tauri::command]
fn get_tosu_status() -> bool {
    TOSU_CONNECTED.load(std::sync::atomic::Ordering::SeqCst)
}

#[tauri::command]
fn minimize_window(window: tauri::Window) {
    let _ = window.minimize();
}

#[tauri::command]
fn maximize_window(window: tauri::Window) {
    if let Ok(is_max) = window.is_maximized() {
        if is_max {
            let _ = window.unmaximize();
        } else {
            let _ = window.maximize();
        }
    }
}

#[tauri::command]
fn close_window(window: tauri::Window) {
    let _ = window.close();
}

#[tauri::command]
fn debug_log(msg: String) {
    println!("[JS DEBUG] {}", msg);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), tauri::Error> {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            load_config_from_disk();

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                start_tosu_proxy(app_handle).await;
            });

            let app_handle2 = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                start_key_server(app_handle2).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            check_tosu,
            calculate_map,
            get_log,
            clear_log,
            get_default_osu_path,
            select_folder,
            validate_path,
            open_url,
            get_tosu_status,
            minimize_window,
            maximize_window,
            close_window,
            debug_log
        ])
        .run(tauri::generate_context!())
} 

