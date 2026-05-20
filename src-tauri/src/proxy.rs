use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use futures_util::StreamExt;
use tauri::{Emitter, Manager};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use crate::config::read_config;
use crate::calc::{calculate_map_internal, rebuild_songs_index};

#[derive(serde::Deserialize)]
struct TosuPath {
    #[serde(default)]
    folder: String,
    #[serde(default)]
    file: String,
}

#[derive(serde::Deserialize)]
struct TosuBeatmap {
    path: TosuPath,
    #[serde(default)]
    md5: String,
    #[serde(default)]
    id: i64,
}

#[derive(serde::Deserialize)]
struct TosuMods {
    #[serde(rename = "str")]
    mods_str: String,
}

#[derive(serde::Deserialize)]
struct TosuMenu {
    bm: TosuBeatmap,
    mods: TosuMods,
}

#[derive(serde::Deserialize)]
struct TosuFallbackBeatmap {
    path: TosuPath,
    #[serde(default)]
    mods: String,
    #[serde(default)]
    md5: String,
    #[serde(default)]
    id: i64,
}

#[derive(serde::Deserialize)]
struct TosuPayload {
    menu: TosuMenu,
    beatmap: Option<TosuFallbackBeatmap>,
}

pub static TOSU_CONNECTED: AtomicBool = AtomicBool::new(false);

pub fn get_overlay_path() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            return dir.to_path_buf();
        }
    }
    PathBuf::from(".")
}

pub async fn start_tosu_proxy(app: tauri::AppHandle) {
    let port = 24050;
    let url = format!("ws://127.0.0.1:{}/ws", port);

    let mut last_map_key = String::new();
    let mut last_mods = String::new();

    {
        let config = read_config();
        if !config.osu_songs_path.is_empty() {
            rebuild_songs_index(&config.osu_songs_path);
        }
    }

    loop {
        let mut request = match url.clone().into_client_request() {
            Ok(req) => req,
            Err(_) => {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        if let Ok(origin) = "http://127.0.0.1:24050".parse() {
            request.headers_mut().insert("Origin", origin);
        }

        match tokio_tungstenite::connect_async(request).await {
            Ok((ws_stream, _)) => {
                TOSU_CONNECTED.store(true, Ordering::SeqCst);
                let _ = app.emit("tosu-status", "connected");
                let mut ws_stream = ws_stream;
                let mut last_emit = std::time::Instant::now();

                while let Some(message) = ws_stream.next().await {
                    match message {
                        Ok(msg) => {
                            if msg.is_text() || msg.is_binary() {
                                let payload = msg.into_text().unwrap_or_default();
                                let mut map_changed = false;

                                if let Ok(data) = serde_json::from_str::<TosuPayload>(&payload) {
                                    let mut folder = data.menu.bm.path.folder;
                                    let mut file = data.menu.bm.path.file;
                                    let mut mods_str = data.menu.mods.mods_str;
                                    let mut md5 = data.menu.bm.md5;
                                    let mut id = data.menu.bm.id.to_string();

                                    if folder.is_empty() && file.is_empty() {
                                        if let Some(fallback) = data.beatmap {
                                            folder = fallback.path.folder;
                                            file = fallback.path.file;
                                            mods_str = fallback.mods;
                                            md5 = fallback.md5;
                                            id = fallback.id.to_string();
                                        }
                                    }

                                    let use_key = if !md5.is_empty() { md5.clone() } else { id.clone() };

                                    if !use_key.is_empty() && use_key != "0" && (use_key != last_map_key || mods_str != last_mods) {
                                        last_map_key = use_key.clone();
                                        last_mods = mods_str.clone();
                                        map_changed = true;

                                        let mut rate = 1.0f32;
                                        let mods_upper = mods_str.to_uppercase();
                                        if mods_upper.contains("DT") || mods_upper.contains("NC") {
                                            rate = 1.5;
                                        } else if mods_upper.contains("HT") {
                                            rate = 0.75;
                                        }

                                        let config = read_config();
                                        let overlay_dir = get_overlay_path();
                                        let _ = tokio::fs::create_dir_all(&overlay_dir).await;
                                        let msd_path = overlay_dir.join("msd.json");

                                        let app_clone = app.clone();
                                        let folder_c = folder.clone();
                                        let file_c = file.clone();
                                        let md5_c = md5.clone();
                                        let use_key_c = use_key.clone();
                                        let mods_str_c = mods_str.clone();

                                        tokio::task::spawn_blocking(move || {
                                            match calculate_map_internal(&config.osu_songs_path, &folder_c, &file_c, rate, &md5_c) {
                                                Ok(ratings) => {
                                                    let output = serde_json::json!({
                                                        "map_key": format!("{}||{}", use_key_c, mods_str_c),
                                                        "ratings": ratings
                                                    });
                                                    if let Ok(json_str) = serde_json::to_string(&output) {
                                                        let _ = std::fs::write(&msd_path, json_str);
                                                    }
                                                    let _ = app_clone.emit("msd-calculated", &ratings);
                                                }
                                                Err(e) => {
                                                    let output = serde_json::json!({
                                                        "map_key": format!("{}||{}", use_key_c, mods_str_c),
                                                        "error": e
                                                    });
                                                    if let Ok(json_str) = serde_json::to_string(&output) {
                                                        let _ = std::fs::write(&msd_path, json_str);
                                                    }
                                                }
                                            }
                                        });
                                    }
                                }

                                let mut should_emit = false;
                                if map_changed {
                                    should_emit = true;
                                } else if last_emit.elapsed() >= std::time::Duration::from_millis(200) {
                                    should_emit = true;
                                }

                                if should_emit {
                                    if let Some(window) = app.get_webview_window("main") {
                                        if let Ok(false) = window.is_minimized() {
                                            let _ = app.emit("tosu-data", payload);
                                            last_emit = std::time::Instant::now();
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            TOSU_CONNECTED.store(false, Ordering::SeqCst);
                            let _ = app.emit("tosu-status", "disconnected");
                            break;
                        }
                    }
                }
                TOSU_CONNECTED.store(false, Ordering::SeqCst);
                let _ = app.emit("tosu-status", "disconnected");
            }
            Err(_) => {
                TOSU_CONNECTED.store(false, Ordering::SeqCst);
                let _ = app.emit("tosu-status", "disconnected");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    }
}
