use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub osu_songs_path: String,
    pub tosu_port: u16,
    pub bg_opacity: f32,
    pub accent_color: String,
    pub scale: f32,
    pub show_radar: bool,
    pub show_particles: bool,
    pub visible_skills: BTreeMap<String, bool>,
    pub keys: Vec<String>,
    pub key_color_outer: String,
    pub key_color_inner: String,
    pub key_size: u32,
    pub key_gap: u32,
    pub show_trails: bool,
    pub trail_opacity: f32,
    pub trail_fade: f32,
    pub hitcounter_opacity: f32,
    pub hitcounter_scale: f32,
    pub hitcounter_bg_color: String,
    pub hitcounter_border_style: String,
    pub hitcounter_orientation: String,
    pub hitcounter_text_color: String,
    pub trail_speed: f32,
    pub trail_height: u32,
    pub trail_widths: Vec<u32>,
    pub key_labels: Vec<String>,
    pub rgb_enabled_keys: Vec<bool>,
    pub rgb_speed: f32,
    pub keys_bg_color: String,
    pub keys_bg_opacity: f32,
    pub key_offsets_x: Vec<i32>,
    pub key_offsets_y: Vec<i32>,
    pub lock_trails: bool,
    pub trail_offsets_x: Vec<i32>,
    pub has_run_before: bool,
    pub key_height: u32,
    pub key_colors: Vec<String>,
    pub trail_offsets_y: Vec<i32>,
}

impl Default for Config {
    fn default() -> Self {
        let mut visible_skills = BTreeMap::new();
        visible_skills.insert("stream".to_string(), true);
        visible_skills.insert("jumpstream".to_string(), true);
        visible_skills.insert("handstream".to_string(), true);
        visible_skills.insert("stamina".to_string(), true);
        visible_skills.insert("jackspeed".to_string(), true);
        visible_skills.insert("chordjack".to_string(), true);
        visible_skills.insert("technical".to_string(), true);

        Config {
            osu_songs_path: String::new(),
            tosu_port: 24050,
            bg_opacity: 0.85,
            accent_color: "#a67c52".to_string(),
            scale: 1.0,
            show_radar: true,
            show_particles: true,
            visible_skills,
            keys: vec!["KeyD".to_string(), "KeyF".to_string(), "KeyJ".to_string(), "KeyK".to_string()],
            key_color_outer: "#00d2ff".to_string(),
            key_color_inner: "#ff007f".to_string(),
            key_size: 60,
            hitcounter_opacity: 0.85,
            hitcounter_scale: 1.0,
            key_gap: 10,
            show_trails: true,
            trail_opacity: 0.6,
            trail_fade: 0.0,
            hitcounter_bg_color: "#000000".to_string(),
            hitcounter_border_style: "none".to_string(),
            hitcounter_orientation: "vertical".to_string(),
            hitcounter_text_color: "#ffffff".to_string(),
            trail_speed: 6.0,
            trail_height: 800,
            trail_widths: vec![50, 50, 50, 50],
            key_labels: vec!["D".to_string(), "F".to_string(), "J".to_string(), "K".to_string()],
            rgb_enabled_keys: vec![false, false, false, false],
            rgb_speed: 1.0,
            keys_bg_color: "#0a0a12".to_string(),
            keys_bg_opacity: 0.7,
            key_offsets_x: vec![0, 0, 0, 0],
            key_offsets_y: vec![0, 0, 0, 0],
            lock_trails: true,
            trail_offsets_x: vec![0, 0, 0, 0],
            has_run_before: false,
            key_height: 60,
            key_colors: vec![
                "#00d2ff".to_string(),
                "#ff007f".to_string(),
                "#ff007f".to_string(),
                "#00d2ff".to_string(),
            ],
            trail_offsets_y: vec![0, 0, 0, 0],
        }
    }
}

lazy_static::lazy_static! {
    static ref CONFIG_CACHE: RwLock<Config> = RwLock::new(Config::default());
}

pub fn config_path() -> PathBuf {
    let base = dirs_next::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("oris-mania-utils").join("config.json")
}

pub fn read_config() -> Config {
    if let Ok(cache) = CONFIG_CACHE.read() {
        return cache.clone();
    }
    Config::default()
}

pub fn load_config_from_disk() -> Config {
    let path = config_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(config) = serde_json::from_str::<Config>(&content) {
            if let Ok(mut cache) = CONFIG_CACHE.write() {
                *cache = config.clone();
            }
            return config;
        }
    }
    Config::default()
}

pub fn write_config(config: &Config) {
    if let Ok(mut cache) = CONFIG_CACHE.write() {
        *cache = config.clone();
    }

    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(content) = serde_json::to_string_pretty(config) {
        let _ = fs::write(&path, &content);

        let overlay_path = crate::proxy::get_overlay_path().join("config.json");
        let _ = fs::write(overlay_path, content);
    }
}
