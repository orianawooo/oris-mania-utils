pub mod app;
pub mod bootstrap;
pub mod calc;
pub mod config;
pub mod dan;
pub mod keys;
#[cfg(target_os = "linux")]
pub mod linux_overlay;
#[cfg(target_os = "linux")]
pub mod overlay_draw;
pub mod proxy;
pub mod theme;
pub mod tosu_manager;

pub use app::run_app;
pub use bootstrap::{
    detect_osu_songs_path, detect_tosu_root, extract_embedded_overlay_files,
    is_valid_osu_songs_path, is_valid_tosu_root, resolve_tosu_root, target_radar_path,
};
pub use calc::{calculate_map_internal, rebuild_songs_index, SkillRatings};
pub use config::{
    config_path, load_config_from_disk, read_config, sync_runtime_overlay_files, write_config,
    Config,
};
pub use dan::{
    calculate_combined_dan, default_dan_tier_color, format_dan_tier, DanResult, RawChartNote,
};
pub use keys::{
    ensure_ws_server_started, start_key_listener, start_key_server, update_active_keys,
};
pub use proxy::{start_tosu_proxy, TOSU_CONNECTED};

pub static HIT_POLL_TRIGGER: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

lazy_static::lazy_static! {
    pub static ref KEY_SENDER: tokio::sync::broadcast::Sender<String> = {
        let (tx, _) = tokio::sync::broadcast::channel(256);
        tx
    };
}

pub fn run() -> Result<(), eframe::Error> {
    app::run_app()
}
