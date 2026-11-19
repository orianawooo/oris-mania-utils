use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicU64, Ordering};
use std::sync::RwLock;

pub const CONFIG_VERSION: u32 = 6;
pub static CONFIG_REV: AtomicU64 = AtomicU64::new(1);
pub static LIVE_TOSU_PORT: AtomicU16 = AtomicU16::new(24050);
pub static LIVE_COMPANION_PORT: AtomicU16 = AtomicU16::new(24051);
pub static ACTIVE_COMPANION_PORT: AtomicU16 = AtomicU16::new(24051);
pub static LIVE_BROWSER_OVERLAYS: AtomicBool = AtomicBool::new(cfg!(target_os = "windows"));
pub static LIVE_HITCOUNTER_POLL_MS: AtomicU32 = AtomicU32::new(16);

fn default_tosu_host() -> String {
    "127.0.0.1".to_string()
}

fn default_companion_port() -> u16 {
    24051
}

fn default_hitcounter_poll_ms() -> u32 {
    16
}

pub fn normalize_host_str(raw: &str) -> String {
    let trimmed = raw
        .trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_start_matches("ws://")
        .trim_start_matches("wss://")
        .trim_end_matches('/');
    let host_only = match trimmed.split_once('/') {
        Some((h, _)) => h,
        None => trimmed,
    };
    let host_only = match host_only.rsplit_once(':') {
        Some((h, port_str)) if !h.contains(':') && port_str.chars().all(|c| c.is_ascii_digit()) => {
            h
        }
        _ => host_only,
    };
    match host_only.trim() {
        "" | "0.0.0.0" | "::" | "[::]" => "127.0.0.1".to_string(),
        other => other.to_string(),
    }
}

pub fn runtime_tosu_host() -> String {
    if let Ok(guard) = LIVE_TOSU_HOST.read() {
        let h = guard.trim();
        if !h.is_empty() {
            return h.to_string();
        }
    }
    "127.0.0.1".to_string()
}

pub fn runtime_hitcounter_poll_ms() -> u32 {
    LIVE_HITCOUNTER_POLL_MS
        .load(Ordering::Relaxed)
        .clamp(8, 250)
}

fn default_enable_browser_overlays() -> bool {
    cfg!(target_os = "windows")
}

fn default_enable_tosu_ingame_overlay() -> bool {
    cfg!(target_os = "windows")
}

fn default_keys_style() -> String {
    "dock-tint".to_string()
}

fn default_hitcounter_style() -> String {
    "radar-twin".to_string()
}

fn default_true() -> bool {
    true
}

pub fn runtime_tosu_port() -> u16 {
    LIVE_TOSU_PORT.load(Ordering::Relaxed).max(1024)
}

pub fn runtime_companion_port() -> u16 {
    LIVE_COMPANION_PORT.load(Ordering::Relaxed).max(1024)
}

pub fn runtime_active_companion_port() -> u16 {
    let active = ACTIVE_COMPANION_PORT.load(Ordering::Relaxed);
    if active >= 1024 {
        active
    } else {
        runtime_companion_port()
    }
}

pub fn set_active_companion_port(port: u16) {
    if port >= 1024 {
        ACTIVE_COMPANION_PORT.store(port, Ordering::Relaxed);
    }
}

pub fn runtime_browser_overlays() -> bool {
    LIVE_BROWSER_OVERLAYS.load(Ordering::Relaxed)
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct Config {
    pub config_version: u32,
    pub osu_songs_path: String,
    pub tosu_root_path: String,
    #[serde(default = "default_tosu_host")]
    pub tosu_host: String,
    pub tosu_port: u16,
    #[serde(default = "default_companion_port")]
    pub companion_port: u16,
    pub show_keys_overlay: bool,
    pub show_radar_overlay: bool,
    pub show_hits_overlay: bool,
    pub lock_overlays: bool,
    pub bg_opacity: f32,
    pub accent_color: String,
    pub scale: f32,
    pub keys_pos_x: i32,
    pub keys_pos_y: i32,
    pub radar_pos_x: i32,
    pub radar_pos_y: i32,
    pub radar_radius: u32,
    pub radar_bg_color: String,
    pub show_radar: bool,
    pub show_particles: bool,
    pub visible_skills: BTreeMap<String, bool>,
    pub keys: Vec<String>,
    pub key_color_outer: String,
    pub key_color_inner: String,
    pub key_size: u32,
    pub key_height: u32,
    pub key_gap: u32,
    pub key_radius: u32,
    pub show_hold_ms: bool,
    pub show_trails: bool,
    pub trail_opacity: f32,
    pub trail_fade: f32,
    pub hitcounter_opacity: f32,
    pub hitcounter_scale: f32,
    pub hitcounter_bg_color: String,
    pub hitcounter_border_style: String,
    pub hitcounter_border_color: String,
    pub hitcounter_orientation: String,
    pub hitcounter_text_color: String,
    pub hitcounter_font: String,
    pub hitcounter_position_x: i32,
    pub hitcounter_position_y: i32,
    pub hitcounter_padding: u32,
    pub hitcounter_gap: u32,
    pub hitcounter_item_width: u32,
    pub hitcounter_item_height: u32,
    pub hitcounter_item_radius: u32,
    pub hitcounter_label_size: f32,
    pub hitcounter_value_size: f32,
    pub hitcounter_dot_size: u32,
    pub hitcounter_glow_strength: f32,
    pub hitcounter_labels: Vec<String>,
    pub hitcounter_colors: Vec<String>,
    pub hitcounter_item_scales: Vec<f32>,
    pub hitcounter_item_offsets_x: Vec<i32>,
    pub hitcounter_item_offsets_y: Vec<i32>,
    pub hitcounter_label_offsets_x: Vec<i32>,
    pub hitcounter_label_offsets_y: Vec<i32>,
    pub hitcounter_value_offsets_x: Vec<i32>,
    pub hitcounter_value_offsets_y: Vec<i32>,
    pub hitcounter_dot_offsets_x: Vec<i32>,
    pub hitcounter_dot_offsets_y: Vec<i32>,
    pub trail_speed: f32,
    pub trail_height: u32,
    pub trail_widths: Vec<u32>,
    pub key_labels: Vec<String>,
    pub rgb_enabled_keys: Vec<bool>,
    pub rgb_speed: f32,
    pub keys_bg_color: String,
    pub keys_bg_opacity: f32,
    pub keys_bg_enabled: bool,
    pub keys_bg_offset_x: i32,
    pub keys_bg_offset_y: i32,
    pub keys_bg_width: u32,
    pub keys_bg_height: u32,
    pub keys_bg_radius: u32,
    pub keys_bg_padding: u32,
    pub keys_bg_scale: f32,
    pub keys_bg_rotation: f32,
    pub keys_bg_shape: String,
    pub bg_layer: i32,
    pub trail_layer: i32,
    pub key_layer: i32,
    pub particle_count: u32,
    pub particle_min_size: u32,
    pub particle_max_size: u32,
    pub particle_spread: u32,
    pub particle_speed: f32,
    pub particle_life: f32,
    pub particle_gravity: f32,
    pub particle_rgb: bool,
    pub particle_shape: String,
    pub key_scales: Vec<f32>,
    pub key_rotations: Vec<f32>,
    pub key_shapes: Vec<String>,
    pub key_offsets_x: Vec<i32>,
    pub key_offsets_y: Vec<i32>,
    pub lock_trails: bool,
    pub trail_scales: Vec<f32>,
    pub trail_rotations: Vec<f32>,
    pub trail_shapes: Vec<String>,
    pub trail_offsets_x: Vec<i32>,
    pub has_run_before: bool,
    pub key_colors: Vec<String>,
    pub trail_offsets_y: Vec<i32>,
    pub debug_overlay_guides: bool,
    pub unsafe_numbers: bool,
    pub preview_rate: f32,
    pub key_inactive_color: String,
    pub key_border_color: String,
    pub keys_border_color: String,
    pub keys_show_accent_bar: bool,
    pub keys_font_size: f32,
    pub per_key_colors: bool,
    pub radar_border_color: String,
    pub radar_text_color: String,
    pub radar_show_header: bool,
    pub radar_show_badge: bool,
    pub radar_show_dots: bool,
    pub radar_fill_opacity: f32,
    pub radar_max_value: f32,
    pub radar_show_dan: bool,
    pub radar_dan_engine: String,
    pub radar_dan_color_mode: String,
    pub radar_dan_color: String,
    pub radar_dan_badge_bg: String,
    pub radar_dan_size: u32,
    pub radar_dan_show_numeric: bool,
    pub radar_dan_show_breakdown: bool,
    pub radar_show_map_bg: bool,
    pub radar_map_bg_opacity: f32,
    pub radar_auto_bg_theme: bool,
    pub radar_show_strain_graph: bool,
    pub radar_graph_height: u32,
    pub radar_graph_fill_opacity: f32,
    pub radar_graph_color_mode: String,
    pub radar_graph_color: String,
    pub radar_graph_show_cursor: bool,
    pub radar_width: u32,
    pub radar_height: u32,
    pub radar_auto_height: bool,
    pub link_overlay_themes: bool,
    pub active_theme_id: String,
    pub keys_theme_id: String,
    pub radar_theme_id: String,
    pub hits_theme_id: String,
    pub keys_auto_bg_theme: bool,
    pub hitcounter_auto_bg_theme: bool,
    pub hitcounter_item_bg_color: String,
    pub hitcounter_show_accent_bar: bool,
    pub hitcounter_show_item_border: bool,
    pub hitcounter_item_border_color: String,
    pub hitcounter_border_width: f32,
    pub window_width: f32,
    pub window_height: f32,
    pub float_keys_open: bool,
    pub float_radar_open: bool,
    pub float_hits_open: bool,
    #[serde(default = "default_enable_browser_overlays")]
    pub enable_browser_overlays: bool,
    #[serde(default = "default_keys_style")]
    pub keys_style: String,
    #[serde(default = "default_hitcounter_style")]
    pub hitcounter_style: String,
    #[serde(default = "default_true")]
    pub hitcounter_show_ratio: bool,
    #[serde(default)]
    pub show_tosu_console: bool,
    #[serde(default = "default_enable_tosu_ingame_overlay")]
    pub enable_tosu_ingame_overlay: bool,
    #[serde(default = "default_hitcounter_poll_ms")]
    pub hitcounter_poll_ms: u32,
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

        let (default_hit_x, default_hit_y) = default_hit_positions("vertical", 140, 20, 2);

        Config {
            config_version: CONFIG_VERSION,
            osu_songs_path: String::new(),
            tosu_root_path: String::new(),
            tosu_host: default_tosu_host(),
            tosu_port: 24050,
            companion_port: default_companion_port(),
            show_keys_overlay: true,
            show_radar_overlay: true,
            show_hits_overlay: true,
            lock_overlays: false,
            bg_opacity: 0.92,
            accent_color: "#e588a6".to_string(),
            scale: 1.0,
            keys_pos_x: 360,
            keys_pos_y: 24,
            radar_pos_x: 24,
            radar_pos_y: 24,
            radar_radius: 12,
            radar_bg_color: "#141415".to_string(),
            show_radar: true,
            show_particles: false,
            visible_skills,
            keys: vec![
                "KeyD".to_string(),
                "KeyF".to_string(),
                "KeyJ".to_string(),
                "KeyK".to_string(),
            ],
            key_color_outer: "#e588a6".to_string(),
            key_color_inner: "#d47695".to_string(),
            key_size: 54,
            key_height: 54,
            key_gap: 0,
            key_radius: 6,
            show_hold_ms: true,
            show_trails: true,
            trail_opacity: 0.88,
            trail_fade: 0.35,
            hitcounter_opacity: 0.92,
            hitcounter_scale: 1.0,
            hitcounter_bg_color: "#141415".to_string(),
            hitcounter_border_style: "solid".to_string(),
            hitcounter_border_color: "#28282b".to_string(),
            hitcounter_orientation: "vertical".to_string(),
            hitcounter_text_color: "#a1a1a6".to_string(),
            hitcounter_font: "Inter".to_string(),
            hitcounter_position_x: 24,
            hitcounter_position_y: 260,
            hitcounter_padding: 10,
            hitcounter_gap: 2,
            hitcounter_item_width: 140,
            hitcounter_item_height: 20,
            hitcounter_item_radius: 8,
            hitcounter_label_size: 11.5,
            hitcounter_value_size: 12.0,
            hitcounter_dot_size: 0,
            hitcounter_glow_strength: 0.0,
            hitcounter_labels: vec![
                "max".to_string(),
                "300".to_string(),
                "200".to_string(),
                "100".to_string(),
                "50".to_string(),
                "miss".to_string(),
            ],
            hitcounter_colors: vec![
                "#ededed".to_string(),
                "#e5b567".to_string(),
                "#4cc38a".to_string(),
                "#6cb6ff".to_string(),
                "#d291e4".to_string(),
                "#e5484d".to_string(),
            ],
            hitcounter_item_scales: vec![1.0; 6],
            hitcounter_item_offsets_x: default_hit_x,
            hitcounter_item_offsets_y: default_hit_y,
            hitcounter_label_offsets_x: vec![0; 6],
            hitcounter_label_offsets_y: vec![0; 6],
            hitcounter_value_offsets_x: vec![0; 6],
            hitcounter_value_offsets_y: vec![0; 6],
            hitcounter_dot_offsets_x: vec![0; 6],
            hitcounter_dot_offsets_y: vec![0; 6],
            trail_speed: 10.0,
            trail_height: 360,
            trail_widths: vec![46, 46, 46, 46],
            key_labels: vec![
                "D".to_string(),
                "F".to_string(),
                "J".to_string(),
                "K".to_string(),
            ],
            rgb_enabled_keys: vec![false, false, false, false],
            rgb_speed: 1.0,
            keys_bg_color: "#141415".to_string(),
            keys_bg_opacity: 0.92,
            keys_bg_enabled: false,
            keys_bg_offset_x: 0,
            keys_bg_offset_y: 0,
            keys_bg_width: 0,
            keys_bg_height: 0,
            keys_bg_radius: 8,
            keys_bg_padding: 8,
            keys_bg_scale: 1.0,
            keys_bg_rotation: 0.0,
            keys_bg_shape: "rounded".to_string(),
            bg_layer: 6,
            trail_layer: 8,
            key_layer: 10,
            particle_count: 6,
            particle_min_size: 1,
            particle_max_size: 3,
            particle_spread: 6,
            particle_speed: 6.0,
            particle_life: 0.8,
            particle_gravity: 0.2,
            particle_rgb: false,
            particle_shape: "square".to_string(),
            key_scales: vec![1.0, 1.0, 1.0, 1.0],
            key_rotations: vec![0.0, 0.0, 0.0, 0.0],
            key_shapes: vec!["rounded".to_string(); 4],
            key_offsets_x: vec![0, 0, 0, 0],
            key_offsets_y: vec![0, 0, 0, 0],
            lock_trails: true,
            trail_scales: vec![1.0, 1.0, 1.0, 1.0],
            trail_rotations: vec![0.0, 0.0, 0.0, 0.0],
            trail_shapes: vec!["rounded".to_string(); 4],
            trail_offsets_x: vec![0, 0, 0, 0],
            has_run_before: false,
            key_colors: vec![
                "#e588a6".to_string(),
                "#d47695".to_string(),
                "#d47695".to_string(),
                "#e588a6".to_string(),
            ],
            trail_offsets_y: vec![0, 0, 0, 0],
            debug_overlay_guides: false,
            unsafe_numbers: false,
            preview_rate: 1.0,
            key_inactive_color: "#141416".to_string(),
            key_border_color: "#28282b".to_string(),
            keys_border_color: "#28282b".to_string(),
            keys_show_accent_bar: false,
            keys_font_size: 15.0,
            per_key_colors: false,
            radar_border_color: "#28282b".to_string(),
            radar_text_color: "#a1a1a6".to_string(),
            radar_show_header: true,
            radar_show_badge: true,
            radar_show_dots: true,
            radar_fill_opacity: 0.22,
            radar_max_value: 38.0,
            radar_show_dan: true,
            radar_dan_engine: "combined".to_string(),
            radar_dan_color_mode: "tier".to_string(),
            radar_dan_color: "#f472b6".to_string(),
            radar_dan_badge_bg: "#1d1d21".to_string(),
            radar_dan_size: 16,
            radar_dan_show_numeric: true,
            radar_dan_show_breakdown: true,
            radar_show_map_bg: true,
            radar_map_bg_opacity: 0.28,
            radar_auto_bg_theme: true,
            radar_show_strain_graph: true,
            radar_graph_height: 34,
            radar_graph_fill_opacity: 0.24,
            radar_graph_color_mode: "accent".to_string(),
            radar_graph_color: "#e588a6".to_string(),
            radar_graph_show_cursor: true,
            radar_width: 312,
            radar_height: 320,
            radar_auto_height: true,
            link_overlay_themes: true,
            active_theme_id: "rose-graphite".to_string(),
            keys_theme_id: "rose-graphite".to_string(),
            radar_theme_id: "rose-graphite".to_string(),
            hits_theme_id: "rose-graphite".to_string(),
            keys_auto_bg_theme: true,
            hitcounter_auto_bg_theme: true,
            hitcounter_item_bg_color: "#1b1b1f".to_string(),
            hitcounter_show_accent_bar: false,
            hitcounter_show_item_border: false,
            hitcounter_item_border_color: "#28282b".to_string(),
            hitcounter_border_width: 1.0,
            window_width: 980.0,
            window_height: 660.0,
            float_keys_open: false,
            float_radar_open: false,
            float_hits_open: false,
            enable_browser_overlays: default_enable_browser_overlays(),
            keys_style: default_keys_style(),
            hitcounter_style: default_hitcounter_style(),
            hitcounter_show_ratio: true,
            show_tosu_console: false,
            enable_tosu_ingame_overlay: default_enable_tosu_ingame_overlay(),
            hitcounter_poll_ms: default_hitcounter_poll_ms(),
        }
    }
}

impl Config {
    pub fn normalize(&mut self) {
        const DEFAULT_KEYS: [&str; 4] = ["KeyD", "KeyF", "KeyJ", "KeyK"];
        const DEFAULT_LABELS: [&str; 4] = ["D", "F", "J", "K"];
        const DEFAULT_HIT_LABELS: [&str; 6] = ["max", "300", "200", "100", "50", "miss"];
        const DEFAULT_HIT_COLORS: [&str; 6] = [
            "#ededed", "#e5b567", "#4cc38a", "#6cb6ff", "#d291e4", "#e5484d",
        ];

        if self.config_version < CONFIG_VERSION {
            let old_bg = self.hitcounter_bg_color.to_lowercase();
            if old_bg == "#2b1730"
                || old_bg == "#10141f"
                || old_bg == "#0f1320"
                || old_bg == "#180f17"
                || old_bg.starts_with("rgba(")
            {
                self.hitcounter_bg_color = "#141415".to_string();
                self.hitcounter_border_color = "#28282b".to_string();
                self.hitcounter_text_color = "#a1a1a6".to_string();
                self.hitcounter_opacity = 0.92;
                self.hitcounter_padding = 8;
                self.hitcounter_gap = 4;
                self.hitcounter_item_width = 132;
                self.hitcounter_item_height = 28;
                self.hitcounter_item_radius = 6;
                self.hitcounter_label_size = 12.0;
                self.hitcounter_value_size = 13.0;
                self.hitcounter_dot_size = 0;
                self.hitcounter_glow_strength = 0.0;
                self.hitcounter_labels = DEFAULT_HIT_LABELS.iter().map(|s| s.to_string()).collect();
                self.hitcounter_colors = DEFAULT_HIT_COLORS.iter().map(|s| s.to_string()).collect();
            }

            let old_keys_bg = self.keys_bg_color.to_lowercase();
            if old_keys_bg == "#2b1730" || old_keys_bg == "#14141a" || old_keys_bg == "#180f17" {
                self.keys_bg_color = "#141415".to_string();
                self.keys_bg_opacity = 0.92;
                self.keys_bg_radius = 10;
                self.keys_bg_padding = 10;
            }

            if self.key_color_outer == "#00d2ff" && self.key_color_inner == "#ff007f" {
                self.key_color_outer = "#e588a6".to_string();
                self.key_color_inner = "#d47695".to_string();
            }

            if self.accent_color == "#ff8ab3" || self.accent_color == "#a67c52" {
                self.accent_color = "#e588a6".to_string();
            }

            if self.key_labels.iter().all(|l| l == "meow") {
                self.key_labels = DEFAULT_LABELS.iter().map(|s| s.to_string()).collect();
            }

            if self.config_version <= 4 {
                self.keys_style = default_keys_style();
                self.keys_bg_enabled = false;
                self.keys_show_accent_bar = false;
                self.key_gap = 0;
                if self.key_radius == 5 || self.key_radius == 8 {
                    self.key_radius = 6;
                }
                self.hitcounter_style = default_hitcounter_style();
                self.hitcounter_orientation = "vertical".to_string();
                self.hitcounter_show_accent_bar = false;
                self.hitcounter_show_item_border = false;
                self.hitcounter_dot_size = 0;
                self.hitcounter_show_ratio = true;
                self.hitcounter_item_width = 140;
                self.hitcounter_item_height = 20;
                self.hitcounter_gap = 2;
                self.enable_tosu_ingame_overlay = default_enable_tosu_ingame_overlay();
            }

            if self.config_version <= 5 {
                if self.trail_opacity <= 0.66 {
                    self.trail_opacity = 0.88;
                }
                if self.trail_fade <= 0.16 {
                    self.trail_fade = 0.35;
                }
                if (self.trail_speed - 6.0).abs() < 0.05 {
                    self.trail_speed = 10.0;
                }
                if self.hitcounter_poll_ms == 0 {
                    self.hitcounter_poll_ms = 16;
                }
            }

            self.config_version = CONFIG_VERSION;
        }

        self.tosu_host = normalize_host_str(&self.tosu_host);
        if self.tosu_port < 1024 {
            self.tosu_port = 24050;
        }
        if self.companion_port < 1024 {
            self.companion_port = 24051;
        }
        if self.companion_port == self.tosu_port {
            self.companion_port = self.tosu_port.saturating_add(1).max(1025);
        }
        if self.hitcounter_poll_ms == 0 {
            self.hitcounter_poll_ms = 16;
        } else {
            self.hitcounter_poll_ms = self.hitcounter_poll_ms.clamp(8, 250);
        }
        if !self.trail_opacity.is_finite() {
            self.trail_opacity = 0.88;
        } else {
            self.trail_opacity = self.trail_opacity.clamp(0.05, 1.0);
        }
        if !self.trail_fade.is_finite() {
            self.trail_fade = 0.35;
        } else {
            self.trail_fade = self.trail_fade.clamp(0.0, 1.0);
        }
        if !self.trail_speed.is_finite() || self.trail_speed < 0.5 {
            self.trail_speed = 10.0;
        } else {
            self.trail_speed = self.trail_speed.clamp(0.5, 60.0);
        }

        if self.keys_style == "crisp" {
            self.keys_style = "tiles".to_string();
        }
        if !matches!(
            self.keys_style.as_str(),
            "dock-tint" | "dock" | "tiles" | "flat"
        ) {
            self.keys_style = default_keys_style();
        }
        if matches!(self.hitcounter_style.as_str(), "telemetry" | "cards") {
            self.hitcounter_style = "radar-twin".to_string();
        }
        if !matches!(
            self.hitcounter_style.as_str(),
            "radar-twin" | "grid-2x3" | "dock" | "dock-ratio" | "hud"
        ) {
            self.hitcounter_style = default_hitcounter_style();
        }
        match self.hitcounter_style.as_str() {
            "dock" | "dock-ratio" => self.hitcounter_orientation = "horizontal".to_string(),
            "radar-twin" | "grid-2x3" => self.hitcounter_orientation = "vertical".to_string(),
            _ => {}
        }

        ensure_len(&mut self.keys, 4, "KeyD".to_string());
        for (i, key) in self.keys.iter_mut().enumerate() {
            if key.trim().is_empty() {
                *key = DEFAULT_KEYS[i].to_string();
            }
        }

        ensure_len(&mut self.key_labels, 4, "D".to_string());
        for (i, label) in self.key_labels.iter_mut().enumerate() {
            if label.trim().is_empty() {
                *label = DEFAULT_LABELS[i].to_string();
            }
        }

        if !self.per_key_colors || self.key_colors.len() != 4 {
            let outer = self.key_color_outer.clone();
            let inner = self.key_color_inner.clone();
            self.key_colors = vec![outer.clone(), inner.clone(), inner, outer];
        } else {
            for (i, col) in self.key_colors.iter_mut().enumerate() {
                if col.trim().is_empty() {
                    *col = if i == 0 || i == 3 {
                        self.key_color_outer.clone()
                    } else {
                        self.key_color_inner.clone()
                    };
                }
            }
        }

        ensure_len(&mut self.rgb_enabled_keys, 4, false);
        ensure_len(&mut self.key_offsets_x, 4, 0);
        ensure_len(&mut self.key_offsets_y, 4, 0);
        ensure_len(&mut self.trail_offsets_x, 4, 0);
        ensure_len(&mut self.trail_offsets_y, 4, 0);
        ensure_len(
            &mut self.trail_widths,
            4,
            self.key_size.saturating_sub(8).max(24),
        );
        ensure_len(&mut self.key_scales, 4, 1.0);
        ensure_len(&mut self.key_rotations, 4, 0.0);
        ensure_len(&mut self.key_shapes, 4, "rounded".to_string());
        ensure_len(&mut self.trail_scales, 4, 1.0);
        ensure_len(&mut self.trail_rotations, 4, 0.0);
        ensure_len(&mut self.trail_shapes, 4, "rounded".to_string());

        if !self.keys_bg_scale.is_finite() || self.keys_bg_scale <= 0.05 {
            self.keys_bg_scale = 1.0;
        }
        if !self.scale.is_finite() || self.scale <= 0.05 {
            self.scale = 1.0;
        }
        if !self.hitcounter_scale.is_finite() || self.hitcounter_scale <= 0.05 {
            self.hitcounter_scale = 1.0;
        }
        if !self.preview_rate.is_finite() || self.preview_rate <= 0.1 {
            self.preview_rate = 1.0;
        }
        if !self.keys_font_size.is_finite() || self.keys_font_size <= 4.0 {
            self.keys_font_size = 15.0;
        }
        if !self.radar_fill_opacity.is_finite() || self.radar_fill_opacity < 0.0 {
            self.radar_fill_opacity = 0.22;
        }
        if !self.radar_max_value.is_finite() || self.radar_max_value <= 1.0 {
            self.radar_max_value = 38.0;
        }
        if !self.radar_map_bg_opacity.is_finite() || self.radar_map_bg_opacity < 0.0 {
            self.radar_map_bg_opacity = 0.28;
        }
        if self.radar_dan_engine.trim().is_empty() {
            self.radar_dan_engine = "combined".to_string();
        }
        if self.radar_dan_color_mode.trim().is_empty() {
            self.radar_dan_color_mode = "tier".to_string();
        }
        if self.radar_dan_color.trim().is_empty() {
            self.radar_dan_color = "#f472b6".to_string();
        }
        if self.radar_dan_badge_bg.trim().is_empty() {
            self.radar_dan_badge_bg = "#1d1d21".to_string();
        }
        if self.radar_dan_size < 10 {
            self.radar_dan_size = 16;
        }
        if !self.radar_graph_fill_opacity.is_finite() || self.radar_graph_fill_opacity < 0.0 {
            self.radar_graph_fill_opacity = 0.24;
        }
        if self.radar_graph_height < 14 {
            self.radar_graph_height = 34;
        }
        if self.radar_graph_color_mode.trim().is_empty() {
            self.radar_graph_color_mode = "accent".to_string();
        }
        if self.radar_graph_color.trim().is_empty() {
            self.radar_graph_color = "#e588a6".to_string();
        }
        if self.radar_width < 150 {
            self.radar_width = 312;
        }
        if self.radar_height < 140 {
            self.radar_height = 320;
        }
        if self.active_theme_id.trim().is_empty() {
            self.active_theme_id = "rose-graphite".to_string();
        }
        if self.keys_theme_id.trim().is_empty() {
            self.keys_theme_id = self.active_theme_id.clone();
        }
        if self.radar_theme_id.trim().is_empty() {
            self.radar_theme_id = self.active_theme_id.clone();
        }
        if self.hits_theme_id.trim().is_empty() {
            self.hits_theme_id = self.active_theme_id.clone();
        }
        if self.hitcounter_item_border_color.trim().is_empty() {
            self.hitcounter_item_border_color = "#28282b".to_string();
        }
        if !self.hitcounter_border_width.is_finite() || self.hitcounter_border_width < 0.0 {
            self.hitcounter_border_width = 1.0;
        }
        if !self.window_width.is_finite() || self.window_width < 720.0 {
            self.window_width = 980.0;
        }
        if !self.window_height.is_finite() || self.window_height < 480.0 {
            self.window_height = 660.0;
        }
        if self.radar_bg_color.trim().is_empty() {
            self.radar_bg_color = "#141415".to_string();
        }
        if self.radar_border_color.trim().is_empty() {
            self.radar_border_color = "#28282b".to_string();
        }
        if self.radar_text_color.trim().is_empty() {
            self.radar_text_color = "#a1a1a6".to_string();
        }
        if self.key_inactive_color.trim().is_empty() {
            self.key_inactive_color = "#19191b".to_string();
        }
        if self.key_border_color.trim().is_empty() {
            self.key_border_color = "#28282b".to_string();
        }
        if self.keys_border_color.trim().is_empty() {
            self.keys_border_color = "#28282b".to_string();
        }
        if self.hitcounter_item_bg_color.trim().is_empty() {
            self.hitcounter_item_bg_color = "#19191b".to_string();
        }

        ensure_len(&mut self.hitcounter_labels, 6, "max".to_string());
        for (i, label) in self.hitcounter_labels.iter_mut().enumerate() {
            if label.trim().is_empty() {
                *label = DEFAULT_HIT_LABELS[i].to_string();
            }
        }

        ensure_len(&mut self.hitcounter_colors, 6, "#ededed".to_string());
        for (i, color) in self.hitcounter_colors.iter_mut().enumerate() {
            if color.trim().is_empty() {
                *color = DEFAULT_HIT_COLORS[i].to_string();
            }
        }

        ensure_len(&mut self.hitcounter_item_scales, 6, 1.0);
        let (default_hit_x, default_hit_y) = default_hit_positions(
            &self.hitcounter_orientation,
            self.hitcounter_item_width,
            self.hitcounter_item_height,
            self.hitcounter_gap,
        );
        self.hitcounter_item_offsets_x = default_hit_x;
        self.hitcounter_item_offsets_y = default_hit_y;
        ensure_len(&mut self.hitcounter_label_offsets_x, 6, 0);
        ensure_len(&mut self.hitcounter_label_offsets_y, 6, 0);
        ensure_len(&mut self.hitcounter_value_offsets_x, 6, 0);
        ensure_len(&mut self.hitcounter_value_offsets_y, 6, 0);
        ensure_len(&mut self.hitcounter_dot_offsets_x, 6, 0);
        ensure_len(&mut self.hitcounter_dot_offsets_y, 6, 0);
    }

    pub fn reset_keys(&mut self) {
        let d = Self::default();
        self.keys_theme_id = d.keys_theme_id;
        self.keys_auto_bg_theme = d.keys_auto_bg_theme;
        self.keys = d.keys;
        self.key_labels = d.key_labels;
        self.key_color_outer = d.key_color_outer;
        self.key_color_inner = d.key_color_inner;
        self.key_colors = d.key_colors;
        self.per_key_colors = d.per_key_colors;
        self.key_inactive_color = d.key_inactive_color;
        self.key_border_color = d.key_border_color;
        self.keys_border_color = d.keys_border_color;
        self.keys_show_accent_bar = d.keys_show_accent_bar;
        self.keys_font_size = d.keys_font_size;
        self.key_size = d.key_size;
        self.key_height = d.key_height;
        self.key_gap = d.key_gap;
        self.key_radius = d.key_radius;
        self.key_shapes = d.key_shapes;
        self.show_hold_ms = d.show_hold_ms;
        self.show_trails = d.show_trails;
        self.trail_opacity = d.trail_opacity;
        self.trail_fade = d.trail_fade;
        self.trail_speed = d.trail_speed;
        self.trail_height = d.trail_height;
        self.trail_widths = d.trail_widths;
        self.trail_shapes = d.trail_shapes;
        self.rgb_enabled_keys = d.rgb_enabled_keys;
        self.rgb_speed = d.rgb_speed;
        self.keys_bg_color = d.keys_bg_color;
        self.keys_bg_opacity = d.keys_bg_opacity;
        self.keys_bg_enabled = d.keys_bg_enabled;
        self.keys_bg_padding = d.keys_bg_padding;
        self.keys_bg_radius = d.keys_bg_radius;
        self.keys_bg_scale = d.keys_bg_scale;
        self.keys_bg_shape = d.keys_bg_shape;
        self.keys_bg_offset_x = d.keys_bg_offset_x;
        self.keys_bg_offset_y = d.keys_bg_offset_y;
        self.keys_pos_x = d.keys_pos_x;
        self.keys_pos_y = d.keys_pos_y;
        self.keys_style = d.keys_style;
    }

    pub fn reset_radar(&mut self) {
        let d = Self::default();
        self.accent_color = d.accent_color;
        self.radar_bg_color = d.radar_bg_color;
        self.radar_border_color = d.radar_border_color;
        self.radar_text_color = d.radar_text_color;
        self.bg_opacity = d.bg_opacity;
        self.scale = d.scale;
        self.radar_radius = d.radar_radius;
        self.radar_pos_x = d.radar_pos_x;
        self.radar_pos_y = d.radar_pos_y;
        self.show_radar = d.show_radar;
        self.radar_show_header = d.radar_show_header;
        self.radar_show_badge = d.radar_show_badge;
        self.radar_show_dots = d.radar_show_dots;
        self.radar_fill_opacity = d.radar_fill_opacity;
        self.radar_max_value = d.radar_max_value;
        self.radar_show_dan = d.radar_show_dan;
        self.radar_dan_engine = d.radar_dan_engine;
        self.radar_dan_color_mode = d.radar_dan_color_mode;
        self.radar_dan_color = d.radar_dan_color;
        self.radar_dan_badge_bg = d.radar_dan_badge_bg;
        self.radar_dan_size = d.radar_dan_size;
        self.radar_dan_show_numeric = d.radar_dan_show_numeric;
        self.radar_dan_show_breakdown = d.radar_dan_show_breakdown;
        self.radar_show_map_bg = d.radar_show_map_bg;
        self.radar_map_bg_opacity = d.radar_map_bg_opacity;
        self.radar_auto_bg_theme = d.radar_auto_bg_theme;
        self.radar_show_strain_graph = d.radar_show_strain_graph;
        self.radar_graph_height = d.radar_graph_height;
        self.radar_graph_fill_opacity = d.radar_graph_fill_opacity;
        self.radar_graph_color_mode = d.radar_graph_color_mode;
        self.radar_graph_color = d.radar_graph_color;
        self.radar_graph_show_cursor = d.radar_graph_show_cursor;
        self.radar_width = d.radar_width;
        self.radar_height = d.radar_height;
        self.radar_auto_height = d.radar_auto_height;
        self.radar_theme_id = d.radar_theme_id;
        self.visible_skills = d.visible_skills;
    }

    pub fn reset_hits(&mut self) {
        let d = Self::default();
        self.hitcounter_orientation = d.hitcounter_orientation;
        self.hitcounter_bg_color = d.hitcounter_bg_color;
        self.hitcounter_item_bg_color = d.hitcounter_item_bg_color;
        self.hitcounter_border_style = d.hitcounter_border_style;
        self.hitcounter_border_color = d.hitcounter_border_color;
        self.hitcounter_item_border_color = d.hitcounter_item_border_color;
        self.hitcounter_show_item_border = d.hitcounter_show_item_border;
        self.hitcounter_show_accent_bar = d.hitcounter_show_accent_bar;
        self.hitcounter_border_width = d.hitcounter_border_width;
        self.hitcounter_glow_strength = d.hitcounter_glow_strength;
        self.hitcounter_auto_bg_theme = d.hitcounter_auto_bg_theme;
        self.hits_theme_id = d.hits_theme_id;
        self.hitcounter_text_color = d.hitcounter_text_color;
        self.hitcounter_opacity = d.hitcounter_opacity;
        self.hitcounter_scale = d.hitcounter_scale;
        self.hitcounter_position_x = d.hitcounter_position_x;
        self.hitcounter_position_y = d.hitcounter_position_y;
        self.hitcounter_padding = d.hitcounter_padding;
        self.hitcounter_gap = d.hitcounter_gap;
        self.hitcounter_item_width = d.hitcounter_item_width;
        self.hitcounter_item_height = d.hitcounter_item_height;
        self.hitcounter_item_radius = d.hitcounter_item_radius;
        self.hitcounter_label_size = d.hitcounter_label_size;
        self.hitcounter_value_size = d.hitcounter_value_size;
        self.hitcounter_dot_size = d.hitcounter_dot_size;
        self.hitcounter_labels = d.hitcounter_labels;
        self.hitcounter_colors = d.hitcounter_colors;
        self.hitcounter_style = d.hitcounter_style;
        self.hitcounter_show_ratio = d.hitcounter_show_ratio;
        self.hitcounter_poll_ms = d.hitcounter_poll_ms;
    }
}

fn ensure_len<T: Clone>(values: &mut Vec<T>, len: usize, default: T) {
    if values.len() < len {
        values.resize(len, default);
    } else if values.len() > len {
        values.truncate(len);
    }
}

fn default_hit_positions(
    orientation: &str,
    item_width: u32,
    item_height: u32,
    gap: u32,
) -> (Vec<i32>, Vec<i32>) {
    let step_x = item_width as i32 + gap as i32;
    let step_y = item_height as i32 + gap as i32;
    let mut xs = Vec::with_capacity(6);
    let mut ys = Vec::with_capacity(6);

    for index in 0..6 {
        if orientation == "horizontal" {
            xs.push(index * step_x);
            ys.push(0);
        } else {
            xs.push(0);
            ys.push(index * step_y);
        }
    }

    (xs, ys)
}

lazy_static::lazy_static! {
    static ref CONFIG_CACHE: RwLock<Config> = RwLock::new(Config::default());
    static ref CONFIG_SERIALIZED_CACHE: RwLock<String> = RwLock::new(String::new());
    static ref LIVE_TOSU_HOST: RwLock<String> = RwLock::new("127.0.0.1".to_string());
}

fn apply_live_runtime_globals(config: &Config) {
    if let Ok(mut host_guard) = LIVE_TOSU_HOST.write() {
        *host_guard = normalize_host_str(&config.tosu_host);
    }
    LIVE_TOSU_PORT.store(config.tosu_port.max(1024), Ordering::Relaxed);
    let comp = config.companion_port.max(1024);
    let prev_comp = LIVE_COMPANION_PORT.swap(comp, Ordering::Relaxed);
    if prev_comp != comp || ACTIVE_COMPANION_PORT.load(Ordering::Relaxed) < 1024 {
        ACTIVE_COMPANION_PORT.store(comp, Ordering::Relaxed);
    }
    LIVE_BROWSER_OVERLAYS.store(config.enable_browser_overlays, Ordering::Relaxed);
    LIVE_HITCOUNTER_POLL_MS.store(config.hitcounter_poll_ms.clamp(8, 250), Ordering::Relaxed);
}

pub fn config_path() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        if let Some(cfg_dir) = dirs_next::config_dir() {
            return cfg_dir.join("oris-mania-utils").join("config.json");
        }
    }
    let base = dirs_next::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("oris-mania-utils").join("config.json")
}

fn mirror_config_path() -> Option<PathBuf> {
    let data_path =
        dirs_next::data_local_dir().map(|d| d.join("oris-mania-utils").join("config.json"));
    if data_path.as_ref() != Some(&config_path()) {
        data_path
    } else {
        None
    }
}

fn legacy_config_candidates() -> Vec<PathBuf> {
    let mut paths = vec![config_path()];
    if let Some(m) = mirror_config_path() {
        paths.push(m);
    }
    if let Some(roaming) = dirs_next::config_dir() {
        let p = roaming.join("oris-mania-utils").join("config.json");
        if !paths.contains(&p) {
            paths.push(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            paths.push(parent.join("config.json"));
            paths.push(parent.join("static").join("radar").join("config.json"));
            paths.push(
                parent
                    .join("static")
                    .join("msdconverter")
                    .join("config.json"),
            );
        }
    }
    paths
}

fn runtime_radar_path(config: &Config) -> Option<PathBuf> {
    if config.tosu_root_path.trim().is_empty() {
        return None;
    }

    let tosu_root = PathBuf::from(&config.tosu_root_path);
    if !crate::bootstrap::is_valid_tosu_root(tosu_root.as_path()) {
        return None;
    }

    Some(crate::bootstrap::target_radar_path(&tosu_root))
}

pub fn sync_runtime_overlay_files(config: &Config) {
    if !config.enable_browser_overlays {
        return;
    }

    let mut normalized = config.clone();
    normalized.normalize();

    let Some(runtime_dir) = runtime_radar_path(&normalized) else {
        return;
    };

    let mut payload = match serde_json::to_value(&normalized) {
        Ok(val) => val,
        Err(_) => return,
    };
    if let Some(obj) = payload.as_object_mut() {
        obj.insert(
            "active_companion_port".to_string(),
            serde_json::json!(runtime_active_companion_port()),
        );
    }
    let content = match serde_json::to_string_pretty(&payload) {
        Ok(content) => content,
        Err(_) => return,
    };

    let _ = fs::create_dir_all(&runtime_dir);
    let _ = fs::write(runtime_dir.join("config.json"), &content);

    let msd_path = runtime_dir.join("msd.json");
    if !msd_path.exists() {
        let seed = serde_json::json!({
            "map_key": "",
            "ratings": {}
        });
        if let Ok(seed_content) = serde_json::to_string(&seed) {
            let _ = fs::write(msd_path, seed_content);
        }
    }
}

pub fn read_config() -> Config {
    if let Ok(cache) = CONFIG_CACHE.read() {
        return cache.clone();
    }
    Config::default()
}

pub fn update_config_cache(config: &Config) {
    let mut normalized = config.clone();
    normalized.normalize();
    apply_live_runtime_globals(&normalized);
    if let Ok(mut cache) = CONFIG_CACHE.write() {
        *cache = normalized;
    }
    CONFIG_REV.fetch_add(1, Ordering::Release);
}

fn atomic_write_file(path: &PathBuf, content: &str) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let tmp = path.with_extension("json.tmp");
    if fs::write(&tmp, content).is_ok() {
        if fs::rename(&tmp, path).is_err() {
            let _ = fs::write(path, content);
            let _ = fs::remove_file(&tmp);
        }
    } else {
        let _ = fs::write(path, content);
    }
}

pub fn load_config_from_disk() -> Config {
    let mut best: Option<(std::time::SystemTime, PathBuf, Config)> = None;
    for candidate in legacy_config_candidates() {
        if let Ok(content) = fs::read_to_string(&candidate) {
            if let Ok(config) = serde_json::from_str::<Config>(&content) {
                let mtime = fs::metadata(&candidate)
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let replace = best
                    .as_ref()
                    .map(|(best_t, _, _)| mtime > *best_t)
                    .unwrap_or(true);
                if replace {
                    best = Some((mtime, candidate, config));
                }
            }
        }
    }

    if let Some((_, candidate, mut config)) = best {
        let needs_migration_save = config.config_version < CONFIG_VERSION
            || candidate != config_path()
            || mirror_config_path().map(|p| !p.exists()).unwrap_or(false);
        config.normalize();
        apply_live_runtime_globals(&config);
        if let Ok(mut cache) = CONFIG_CACHE.write() {
            *cache = config.clone();
        }
        CONFIG_REV.fetch_add(1, Ordering::Release);
        if needs_migration_save {
            write_config(&config);
        } else if let Ok(serialized) = serde_json::to_string_pretty(&config) {
            if let Ok(mut serialized_cache) = CONFIG_SERIALIZED_CACHE.write() {
                *serialized_cache = serialized;
            }
        }
        return config;
    }

    let mut config = Config::default();
    config.normalize();
    apply_live_runtime_globals(&config);
    if let Ok(mut cache) = CONFIG_CACHE.write() {
        *cache = config.clone();
    }
    write_config(&config);
    config
}

pub fn write_config(config: &Config) {
    let mut normalized = config.clone();
    normalized.normalize();
    apply_live_runtime_globals(&normalized);

    if let Ok(content) = serde_json::to_string_pretty(&normalized) {
        let already_persisted = if let Ok(cache) = CONFIG_SERIALIZED_CACHE.read() {
            *cache == content
        } else {
            false
        };

        if let Ok(mut cache) = CONFIG_CACHE.write() {
            *cache = normalized.clone();
        }
        CONFIG_REV.fetch_add(1, Ordering::Release);

        let path = config_path();
        let mirror = mirror_config_path();
        let local_missing = !path.exists() || mirror.as_ref().map(|m| !m.exists()).unwrap_or(false);
        let overlay_config_missing = normalized.enable_browser_overlays
            && runtime_radar_path(&normalized)
                .map(|runtime_dir| !runtime_dir.join("config.json").exists())
                .unwrap_or(false);

        if already_persisted && !local_missing && !overlay_config_missing {
            return;
        }

        atomic_write_file(&path, &content);
        if let Some(m) = mirror {
            atomic_write_file(&m, &content);
        }

        sync_runtime_overlay_files(&normalized);

        if let Ok(mut serialized_cache) = CONFIG_SERIALIZED_CACHE.write() {
            *serialized_cache = content;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v6_defaults_and_legacy_migration_are_consistent() {
        let def = Config::default();
        assert_eq!(def.keys_style, "dock-tint");
        assert_eq!(def.hitcounter_style, "radar-twin");
        assert_eq!(def.hitcounter_dot_size, 0);
        assert!(def.hitcounter_show_ratio);
        assert!((def.trail_opacity - 0.88).abs() < 1e-4);
        assert!((def.trail_fade - 0.35).abs() < 1e-4);
        assert!((def.trail_speed - 10.0).abs() < 1e-4);
        assert_eq!(def.hitcounter_poll_ms, 16);
        assert!(!def.show_tosu_console);

        let mut legacy = Config {
            config_version: 4,
            keys_style: "crisp".to_string(),
            hitcounter_style: "telemetry".to_string(),
            hitcounter_dot_size: 4,
            hitcounter_show_ratio: false,
            trail_opacity: 0.65,
            trail_fade: 0.0,
            trail_speed: 6.0,
            hitcounter_poll_ms: 0,
            ..Default::default()
        };
        legacy.normalize();
        assert_eq!(legacy.config_version, CONFIG_VERSION);
        assert_eq!(legacy.keys_style, "dock-tint");
        assert_eq!(legacy.hitcounter_style, "radar-twin");
        assert_eq!(legacy.hitcounter_dot_size, 0);
        assert!(legacy.hitcounter_show_ratio);
        assert!((legacy.trail_opacity - 0.88).abs() < 1e-4);
        assert!((legacy.trail_fade - 0.35).abs() < 1e-4);
        assert!((legacy.trail_speed - 10.0).abs() < 1e-4);
        assert_eq!(legacy.hitcounter_poll_ms, 16);

        let mut v5_custom = Config {
            config_version: 5,
            trail_opacity: 0.95,
            trail_fade: 0.60,
            trail_speed: 14.0,
            hitcounter_poll_ms: 80,
            ..Default::default()
        };
        v5_custom.normalize();
        assert!((v5_custom.trail_opacity - 0.95).abs() < 1e-4);
        assert!((v5_custom.trail_fade - 0.60).abs() < 1e-4);
        assert!((v5_custom.trail_speed - 14.0).abs() < 1e-4);
        assert_eq!(v5_custom.hitcounter_poll_ms, 80);

        legacy.keys_style = "crisp".to_string();
        legacy.hitcounter_style = "dock-ratio".to_string();
        legacy.tosu_host = "http://0.0.0.0:24050/".to_string();
        legacy.tosu_port = 24060;
        legacy.companion_port = 24060;
        legacy.normalize();
        assert_eq!(legacy.keys_style, "tiles");
        assert_eq!(legacy.hitcounter_style, "dock-ratio");
        assert_eq!(legacy.hitcounter_orientation, "horizontal");
        assert_eq!(legacy.tosu_host, "127.0.0.1");
        assert_eq!(legacy.tosu_port, 24060);
        assert_eq!(legacy.companion_port, 24061);
    }
}
