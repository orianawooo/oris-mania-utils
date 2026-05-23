mod config;
mod calc;
mod bootstrap;
mod proxy;
mod keys;
mod update;

use std::fs;
use std::path::PathBuf;
use tauri::Emitter;

pub use config::{Config, read_config, load_config_from_disk, write_config, config_path, sync_runtime_overlay_files};
pub use calc::{SkillRatings, calculate_map_internal, rebuild_songs_index};
pub use proxy::{start_tosu_proxy, TOSU_CONNECTED};
pub use keys::{start_key_server, update_active_keys};
pub use bootstrap::{bootstrap_config, detect_osu_songs_path, detect_tosu_root, ensure_overlays_installed, install_selected_overlays_with_progress, is_valid_osu_songs_path, is_valid_tosu_root, locate_overlay_source_root, target_msdconverter_path, BootstrapReport, InstallReport, OverlayInstallProgress, OverlayStatus};
pub use update::{apply_latest_update as apply_latest_update_impl, check_for_updates as check_for_updates_impl, AppUpdateProgress, ReleaseAssetInfo, UpdateStatus};

lazy_static::lazy_static! {
    pub static ref KEY_SENDER: tokio::sync::broadcast::Sender<String> = {
        let (tx, _) = tokio::sync::broadcast::channel(32);
        tx
    };
}

#[derive(serde::Serialize)]
struct KeystrokeRuntimeFingerprint<'a> {
    keys: &'a [String],
    key_labels: &'a [String],
    key_colors: &'a [String],
    key_size: u32,
    key_height: u32,
    key_gap: u32,
    show_particles: bool,
    show_trails: bool,
    trail_opacity: f32,
    trail_fade: f32,
    trail_speed: f32,
    trail_height: u32,
    trail_widths: &'a [u32],
    rgb_enabled_keys: &'a [bool],
    rgb_speed: f32,
    keys_bg_color: &'a str,
    keys_bg_opacity: f32,
    keys_bg_enabled: bool,
    keys_bg_offset_x: i32,
    keys_bg_offset_y: i32,
    keys_bg_width: u32,
    keys_bg_height: u32,
    keys_bg_radius: u32,
    keys_bg_padding: u32,
    keys_bg_scale: f32,
    keys_bg_rotation: f32,
    keys_bg_shape: &'a str,
    bg_layer: i32,
    trail_layer: i32,
    key_layer: i32,
    particle_count: u32,
    particle_min_size: u32,
    particle_max_size: u32,
    particle_spread: u32,
    particle_speed: f32,
    particle_life: f32,
    particle_gravity: f32,
    particle_rgb: bool,
    particle_shape: &'a str,
    key_scales: &'a [f32],
    key_rotations: &'a [f32],
    key_shapes: &'a [String],
    key_offsets_x: &'a [i32],
    key_offsets_y: &'a [i32],
    lock_trails: bool,
    trail_scales: &'a [f32],
    trail_rotations: &'a [f32],
    trail_shapes: &'a [String],
    trail_offsets_x: &'a [i32],
    trail_offsets_y: &'a [i32],
    debug_overlay_guides: bool,
}

fn broadcast_bindings(config: &Config) {
    let mut val = serde_json::to_value(config).unwrap_or_default();
    if let Some(obj) = val.as_object_mut() {
        obj.insert("event".to_string(), serde_json::json!("bindings"));
    }
    let _ = KEY_SENDER.send(val.to_string());
}

fn broadcast_config_update(config: &Config, scopes: &[&str]) {
    let mut val = serde_json::to_value(config).unwrap_or_default();
    if let Some(obj) = val.as_object_mut() {
        obj.insert("event".to_string(), serde_json::json!("config-updated"));
        obj.insert("changed_scopes".to_string(), serde_json::json!(scopes));
    }
    let _ = KEY_SENDER.send(val.to_string());
}

fn msd_runtime_fingerprint(config: &Config) -> String {
    serde_json::json!({
        "tosu_port": config.tosu_port,
        "bg_opacity": config.bg_opacity,
        "scale": config.scale,
        "show_radar": config.show_radar,
        "visible_skills": config.visible_skills,
    }).to_string()
}

fn hitcounter_runtime_fingerprint(config: &Config) -> String {
    serde_json::json!({
        "tosu_port": config.tosu_port,
        "hitcounter_opacity": config.hitcounter_opacity,
        "hitcounter_scale": config.hitcounter_scale,
        "hitcounter_bg_color": config.hitcounter_bg_color,
        "hitcounter_border_style": config.hitcounter_border_style,
        "hitcounter_border_color": config.hitcounter_border_color,
        "hitcounter_orientation": config.hitcounter_orientation,
        "hitcounter_text_color": config.hitcounter_text_color,
        "hitcounter_font": config.hitcounter_font,
        "hitcounter_position_x": config.hitcounter_position_x,
        "hitcounter_position_y": config.hitcounter_position_y,
        "hitcounter_padding": config.hitcounter_padding,
        "hitcounter_gap": config.hitcounter_gap,
        "hitcounter_item_width": config.hitcounter_item_width,
        "hitcounter_item_height": config.hitcounter_item_height,
        "hitcounter_item_radius": config.hitcounter_item_radius,
        "hitcounter_label_size": config.hitcounter_label_size,
        "hitcounter_value_size": config.hitcounter_value_size,
        "hitcounter_dot_size": config.hitcounter_dot_size,
        "hitcounter_glow_strength": config.hitcounter_glow_strength,
        "hitcounter_labels": config.hitcounter_labels,
        "hitcounter_colors": config.hitcounter_colors,
        "hitcounter_item_scales": config.hitcounter_item_scales,
        "hitcounter_item_offsets_x": config.hitcounter_item_offsets_x,
        "hitcounter_item_offsets_y": config.hitcounter_item_offsets_y,
        "hitcounter_label_offsets_x": config.hitcounter_label_offsets_x,
        "hitcounter_label_offsets_y": config.hitcounter_label_offsets_y,
        "hitcounter_value_offsets_x": config.hitcounter_value_offsets_x,
        "hitcounter_value_offsets_y": config.hitcounter_value_offsets_y,
        "hitcounter_dot_offsets_x": config.hitcounter_dot_offsets_x,
        "hitcounter_dot_offsets_y": config.hitcounter_dot_offsets_y,
    }).to_string()
}

fn changed_overlay_scopes(previous: &Config, next: &Config) -> Vec<&'static str> {
    let mut scopes = Vec::new();

    if msd_runtime_fingerprint(previous) != msd_runtime_fingerprint(next) {
        scopes.push("msdconverter");
    }

    if hitcounter_runtime_fingerprint(previous) != hitcounter_runtime_fingerprint(next) {
        scopes.push("HitCounter");
    }

    if keystroke_runtime_fingerprint(previous) != keystroke_runtime_fingerprint(next) {
        scopes.push("ManiaKeystrokes");
    }

    scopes
}

fn keystroke_runtime_fingerprint(config: &Config) -> String {
    serde_json::to_string(&KeystrokeRuntimeFingerprint {
        keys: &config.keys,
        key_labels: &config.key_labels,
        key_colors: &config.key_colors,
        key_size: config.key_size,
        key_height: config.key_height,
        key_gap: config.key_gap,
        show_particles: config.show_particles,
        show_trails: config.show_trails,
        trail_opacity: config.trail_opacity,
        trail_fade: config.trail_fade,
        trail_speed: config.trail_speed,
        trail_height: config.trail_height,
        trail_widths: &config.trail_widths,
        rgb_enabled_keys: &config.rgb_enabled_keys,
        rgb_speed: config.rgb_speed,
        keys_bg_color: &config.keys_bg_color,
        keys_bg_opacity: config.keys_bg_opacity,
        keys_bg_enabled: config.keys_bg_enabled,
        keys_bg_offset_x: config.keys_bg_offset_x,
        keys_bg_offset_y: config.keys_bg_offset_y,
        keys_bg_width: config.keys_bg_width,
        keys_bg_height: config.keys_bg_height,
        keys_bg_radius: config.keys_bg_radius,
        keys_bg_padding: config.keys_bg_padding,
        keys_bg_scale: config.keys_bg_scale,
        keys_bg_rotation: config.keys_bg_rotation,
        keys_bg_shape: &config.keys_bg_shape,
        bg_layer: config.bg_layer,
        trail_layer: config.trail_layer,
        key_layer: config.key_layer,
        particle_count: config.particle_count,
        particle_min_size: config.particle_min_size,
        particle_max_size: config.particle_max_size,
        particle_spread: config.particle_spread,
        particle_speed: config.particle_speed,
        particle_life: config.particle_life,
        particle_gravity: config.particle_gravity,
        particle_rgb: config.particle_rgb,
        particle_shape: &config.particle_shape,
        key_scales: &config.key_scales,
        key_rotations: &config.key_rotations,
        key_shapes: &config.key_shapes,
        key_offsets_x: &config.key_offsets_x,
        key_offsets_y: &config.key_offsets_y,
        lock_trails: config.lock_trails,
        trail_scales: &config.trail_scales,
        trail_rotations: &config.trail_rotations,
        trail_shapes: &config.trail_shapes,
        trail_offsets_x: &config.trail_offsets_x,
        trail_offsets_y: &config.trail_offsets_y,
        debug_overlay_guides: config.debug_overlay_guides,
    }).unwrap_or_default()
}

#[tauri::command]
fn get_config() -> Config {
    read_config()
}

#[tauri::command]
fn get_default_config() -> Config {
    Config::default()
}

#[tauri::command]
fn save_config(config: Config) {
    let previous = read_config();
    write_config(&config);
    let normalized = read_config();
    let config_changed = previous != normalized;
    let bindings_changed = keystroke_runtime_fingerprint(&previous) != keystroke_runtime_fingerprint(&normalized);
    let changed_scopes = changed_overlay_scopes(&previous, &normalized);

    if previous.keys != normalized.keys {
        update_active_keys(&normalized.keys);
    }

    if previous.osu_songs_path != normalized.osu_songs_path && !normalized.osu_songs_path.is_empty() {
        let path = normalized.osu_songs_path.clone();
        std::thread::spawn(move || rebuild_songs_index(&path));
    }

    if config_changed && !changed_scopes.is_empty() {
        broadcast_config_update(&normalized, &changed_scopes);
    }

    if bindings_changed {
        broadcast_bindings(&normalized);
    }
}

#[tauri::command]
fn check_tosu(port: u16) -> bool {
    std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok()
}

#[tauri::command]
fn get_default_osu_path() -> String {
    detect_osu_songs_path()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default()
}

#[tauri::command]
fn get_default_tosu_path() -> String {
    let config = read_config();
    let configured = if config.tosu_root_path.is_empty() {
        None
    } else {
        Some(PathBuf::from(config.tosu_root_path))
    };

    detect_tosu_root(configured.as_deref())
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default()
}

#[tauri::command]
fn validate_tosu_root(path: String) -> bool {
    is_valid_tosu_root(std::path::Path::new(&path))
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
fn select_tosu_folder(app: tauri::AppHandle) -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();

    let _ = app.run_on_main_thread(move || {
        let folder = rfd::FileDialog::new()
            .set_title("Select TOSU Folder")
            .pick_folder();
        let _ = tx.send(folder);
    });

    let folder = rx.recv().unwrap_or(None);
    folder.map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
fn validate_path(path: String) -> bool {
    is_valid_osu_songs_path(std::path::Path::new(&path))
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
fn save_text_file(app: tauri::AppHandle, default_name: String, contents: String) -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();

    let _ = app.run_on_main_thread(move || {
        let file = rfd::FileDialog::new()
            .set_title("Export layout")
            .set_file_name(&default_name)
            .add_filter("JSON", &["json"])
            .save_file();
        let _ = tx.send(file);
    });

    let path = rx.recv().ok().flatten()?;
    std::fs::write(&path, contents).ok()?;
    Some(path.to_string_lossy().to_string())
}

#[tauri::command]
fn debug_log(msg: String) {
    #[cfg(debug_assertions)]
    println!("[JS DEBUG] {}", msg);
    #[cfg(not(debug_assertions))]
    let _ = msg;
}

#[tauri::command]
fn set_bind_listening_mode(listening: bool) {
    crate::keys::IS_LISTENING_FOR_BIND.store(listening, std::sync::atomic::Ordering::Relaxed);
}

#[tauri::command]
fn detect_tosu_root_path() -> String {
    let config = read_config();
    let configured = if config.tosu_root_path.is_empty() {
        None
    } else {
        Some(PathBuf::from(config.tosu_root_path))
    };

    detect_tosu_root(configured.as_deref())
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default()
}

#[tauri::command]
fn bootstrap_environment(app: tauri::AppHandle) -> BootstrapReport {
    let previous = read_config();
    let report = bootstrap_config(&app);
    let config_changed = previous != report.config;
    let bindings_changed = keystroke_runtime_fingerprint(&previous) != keystroke_runtime_fingerprint(&report.config);
    let changed_scopes = changed_overlay_scopes(&previous, &report.config);

    if previous.keys != report.config.keys {
        update_active_keys(&report.config.keys);
    }
    if config_changed {
        write_config(&report.config);
    }

    if previous.osu_songs_path != report.config.osu_songs_path && !report.config.osu_songs_path.is_empty() {
        let path = report.config.osu_songs_path.clone();
        std::thread::spawn(move || rebuild_songs_index(&path));
    }

    sync_runtime_overlay_files(&report.config);

    if config_changed && !changed_scopes.is_empty() {
        broadcast_config_update(&report.config, &changed_scopes);
    }

    if bindings_changed {
        broadcast_bindings(&report.config);
    }
    report
}

#[tauri::command]
fn install_selected_overlays(
    app: tauri::AppHandle,
    tosu_root: String,
    overlays: Vec<String>,
) -> Result<InstallReport, String> {
    let tosu_path = PathBuf::from(&tosu_root);
    if !is_valid_tosu_root(&tosu_path) {
        return Err("The selected TOSU folder is invalid.".to_string());
    }

    let report = install_selected_overlays_with_progress(&app, &tosu_path, &overlays, |progress| {
        let _ = app.emit("overlay-install-progress", &progress);
    })?;

    let mut config = read_config();
    if config.tosu_root_path != tosu_root {
        config.tosu_root_path = tosu_root;
        write_config(&config);
    } else {
        sync_runtime_overlay_files(&config);
    }

    Ok(report)
}

#[tauri::command]
async fn check_for_updates() -> Result<UpdateStatus, String> {
    let config = read_config();
    check_for_updates_impl(&config).await
}

#[tauri::command]
async fn apply_latest_update(app: tauri::AppHandle) -> Result<String, String> {
    let config = read_config();
    apply_latest_update_impl(app, &config).await
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
            get_default_config,
            save_config,
            check_tosu,
            calculate_map,
            get_log,
            clear_log,
            get_default_osu_path,
            get_default_tosu_path,
            select_folder,
            select_tosu_folder,
            validate_path,
            validate_tosu_root,
            open_url,
            get_tosu_status,
            minimize_window,
            maximize_window,
            close_window,
            save_text_file,
            debug_log,
            set_bind_listening_mode,
            detect_tosu_root_path,
            bootstrap_environment,
            install_selected_overlays,
            check_for_updates,
            apply_latest_update
        ])
        .run(tauri::generate_context!())
} 
