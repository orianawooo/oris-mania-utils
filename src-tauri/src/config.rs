use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

pub const CONFIG_VERSION: u32 = 2;

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Config {
    pub config_version: u32,
    pub osu_songs_path: String,
    pub tosu_root_path: String,
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
    pub key_height: u32,
    pub key_colors: Vec<String>,
    pub trail_offsets_y: Vec<i32>,
    pub debug_overlay_guides: bool,
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
            config_version: CONFIG_VERSION,
            osu_songs_path: String::new(),
            tosu_root_path: String::new(),
            tosu_port: 24050,
            bg_opacity: 0.85,
            accent_color: "#ff8ab3".to_string(),
            scale: 1.0,
            show_radar: true,
            show_particles: true,
            visible_skills,
            keys: vec!["KeyD".to_string(), "KeyF".to_string(), "KeyJ".to_string(), "KeyK".to_string()],
            key_color_outer: "#00d2ff".to_string(),
            key_color_inner: "#ff007f".to_string(),
            key_size: 60,
            hitcounter_opacity: 0.82,
            hitcounter_scale: 1.0,
            key_gap: 10,
            show_trails: true,
            trail_opacity: 0.6,
            trail_fade: 0.0,
            hitcounter_bg_color: "#0f1320".to_string(),
            hitcounter_border_style: "solid".to_string(),
            hitcounter_border_color: "#262f47".to_string(),
            hitcounter_orientation: "vertical".to_string(),
            hitcounter_text_color: "#8b8ba8".to_string(),
            hitcounter_font: "Inter".to_string(),
            hitcounter_position_x: 0,
            hitcounter_position_y: 0,
            hitcounter_padding: 10,
            hitcounter_gap: 8,
            hitcounter_item_width: 118,
            hitcounter_item_height: 62,
            hitcounter_item_radius: 10,
            hitcounter_label_size: 10.0,
            hitcounter_value_size: 22.0,
            hitcounter_dot_size: 6,
            hitcounter_glow_strength: 0.12,
            hitcounter_labels: vec![
                "MAX".to_string(),
                "PERF".to_string(),
                "GREAT".to_string(),
                "GOOD".to_string(),
                "BAD".to_string(),
                "MISS".to_string(),
            ],
            hitcounter_colors: vec![
                "#ffffff".to_string(),
                "#fbc531".to_string(),
                "#4cd137".to_string(),
                "#00a8ff".to_string(),
                "#e84118".to_string(),
                "#7f8c8d".to_string(),
            ],
            hitcounter_item_scales: vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
            hitcounter_item_offsets_x: vec![0, 0, 0, 0, 0, 0],
            hitcounter_item_offsets_y: vec![0, 70, 140, 210, 280, 350],
            hitcounter_label_offsets_x: vec![0, 0, 0, 0, 0, 0],
            hitcounter_label_offsets_y: vec![0, 0, 0, 0, 0, 0],
            hitcounter_value_offsets_x: vec![0, 0, 0, 0, 0, 0],
            hitcounter_value_offsets_y: vec![0, 0, 0, 0, 0, 0],
            hitcounter_dot_offsets_x: vec![0, 0, 0, 0, 0, 0],
            hitcounter_dot_offsets_y: vec![0, 0, 0, 0, 0, 0],
            trail_speed: 6.0,
            trail_height: 800,
            trail_widths: vec![50, 50, 50, 50],
            key_labels: vec!["D".to_string(), "F".to_string(), "J".to_string(), "K".to_string()],
            rgb_enabled_keys: vec![false, false, false, false],
            rgb_speed: 1.0,
            keys_bg_color: "#2b1730".to_string(),
            keys_bg_opacity: 0.7,
            keys_bg_enabled: true,
            keys_bg_offset_x: 0,
            keys_bg_offset_y: 0,
            keys_bg_width: 0,
            keys_bg_height: 0,
            keys_bg_radius: 16,
            keys_bg_padding: 15,
            keys_bg_scale: 1.0,
            keys_bg_rotation: 0.0,
            keys_bg_shape: "rounded".to_string(),
            bg_layer: 6,
            trail_layer: 8,
            key_layer: 10,
            particle_count: 8,
            particle_min_size: 1,
            particle_max_size: 4,
            particle_spread: 6,
            particle_speed: 8.0,
            particle_life: 1.0,
            particle_gravity: 0.2,
            particle_rgb: false,
            particle_shape: "square".to_string(),
            key_scales: vec![1.0, 1.0, 1.0, 1.0],
            key_rotations: vec![0.0, 0.0, 0.0, 0.0],
            key_shapes: vec![
                "rounded".to_string(),
                "rounded".to_string(),
                "rounded".to_string(),
                "rounded".to_string(),
            ],
            key_offsets_x: vec![0, 0, 0, 0],
            key_offsets_y: vec![0, 0, 0, 0],
            lock_trails: true,
            trail_scales: vec![1.0, 1.0, 1.0, 1.0],
            trail_rotations: vec![0.0, 0.0, 0.0, 0.0],
            trail_shapes: vec![
                "rounded".to_string(),
                "rounded".to_string(),
                "rounded".to_string(),
                "rounded".to_string(),
            ],
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
            debug_overlay_guides: false,
        }
    }
}

impl Config {
    fn normalize(&mut self) {
        self.config_version = CONFIG_VERSION;
        const DEFAULT_KEYS: [&str; 4] = ["KeyD", "KeyF", "KeyJ", "KeyK"];
        const DEFAULT_LABELS: [&str; 4] = ["D", "F", "J", "K"];
        const DEFAULT_COLORS: [&str; 4] = ["#00d2ff", "#ff007f", "#ff007f", "#00d2ff"];
        const DEFAULT_SHAPE: &str = "rounded";
        const DEFAULT_HIT_LABELS: [&str; 6] = ["MAX", "PERF", "GREAT", "GOOD", "BAD", "MISS"];
        const DEFAULT_HIT_COLORS: [&str; 6] = ["#ffffff", "#fbc531", "#4cd137", "#00a8ff", "#e84118", "#7f8c8d"];
        const LEGACY_HIT_COLORS: [&str; 6] = ["#ffffff", "#ffcf6c", "#8ee0a0", "#94d5ff", "#ff8b8b", "#ac97a4"];
        const LEGACY_HIT_X: [i32; 6] = [0, 128, 0, 128, 0, 128];
        const LEGACY_HIT_Y: [i32; 6] = [0, 0, 82, 82, 164, 164];
        const CLEAN_HIT_X: [i32; 6] = [0, 0, 0, 0, 0, 0];
        const CLEAN_HIT_Y: [i32; 6] = [0, 78, 156, 234, 312, 390];

        let uses_legacy_hit_style =
            self.hitcounter_bg_color == "#2b1730"
                && self.hitcounter_text_color == "#ffffff"
                && self.hitcounter_border_style == "none"
                && self.hitcounter_border_color == "#e6bfd4"
                && self.hitcounter_font == "Nunito"
                && self.hitcounter_padding == 16
                && self.hitcounter_gap == 10
                && self.hitcounter_item_width == 118
                && self.hitcounter_item_height == 72
                && self.hitcounter_item_radius == 16
                && self.hitcounter_dot_size == 8
                && (self.hitcounter_glow_strength - 0.35).abs() < f32::EPSILON;
        let uses_clean_hit_style =
            self.hitcounter_bg_color == "#10141f"
                && self.hitcounter_text_color == "#b7bfd4"
                && self.hitcounter_border_style == "solid"
                && self.hitcounter_border_color == "#2b3347"
                && self.hitcounter_font == "Urbanist"
                && self.hitcounter_padding == 12
                && self.hitcounter_gap == 8
                && self.hitcounter_item_width == 116
                && self.hitcounter_item_height == 70
                && self.hitcounter_item_radius == 12
                && self.hitcounter_dot_size == 6
                && (self.hitcounter_glow_strength - 0.18).abs() < f32::EPSILON;

        if uses_legacy_hit_style || uses_clean_hit_style {
            let had_legacy_positions =
                self.hitcounter_item_offsets_x == LEGACY_HIT_X
                    && self.hitcounter_item_offsets_y == LEGACY_HIT_Y;
            let had_clean_positions =
                self.hitcounter_item_offsets_x == CLEAN_HIT_X
                    && self.hitcounter_item_offsets_y == CLEAN_HIT_Y;

            self.hitcounter_opacity = 0.82;
            self.hitcounter_bg_color = "#0f1320".to_string();
            self.hitcounter_text_color = "#8b8ba8".to_string();
            self.hitcounter_border_style = "solid".to_string();
            self.hitcounter_border_color = "#262f47".to_string();
            self.hitcounter_font = "Inter".to_string();
            self.hitcounter_padding = 10;
            self.hitcounter_gap = 8;
            self.hitcounter_item_width = 118;
            self.hitcounter_item_height = 62;
            self.hitcounter_item_radius = 10;
            self.hitcounter_label_size = 10.0;
            self.hitcounter_value_size = 22.0;
            self.hitcounter_dot_size = 6;
            self.hitcounter_glow_strength = 0.12;

            if self.hitcounter_colors.iter().map(|color| color.as_str()).eq(LEGACY_HIT_COLORS.iter().copied()) {
                self.hitcounter_colors = DEFAULT_HIT_COLORS.iter().map(|color| color.to_string()).collect();
            }

            if had_legacy_positions || had_clean_positions {
                let (xs, ys) = default_hit_positions(
                    &self.hitcounter_orientation,
                    self.hitcounter_item_width,
                    self.hitcounter_item_height,
                    self.hitcounter_gap,
                );
                self.hitcounter_item_offsets_x = xs;
                self.hitcounter_item_offsets_y = ys;
            }
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

        ensure_len(&mut self.key_colors, 4, "#00d2ff".to_string());
        for (i, color) in self.key_colors.iter_mut().enumerate() {
            if color.trim().is_empty() {
                *color = DEFAULT_COLORS[i].to_string();
            }
        }

        ensure_len(&mut self.rgb_enabled_keys, 4, false);
        ensure_len(&mut self.key_offsets_x, 4, 0);
        ensure_len(&mut self.key_offsets_y, 4, 0);
        ensure_len(&mut self.trail_offsets_x, 4, 0);
        ensure_len(&mut self.trail_offsets_y, 4, 0);
        ensure_len(&mut self.trail_widths, 4, 50);

        ensure_len(&mut self.key_scales, 4, 1.0);
        for scale in &mut self.key_scales {
            if !scale.is_finite() || *scale <= 0.0 {
                *scale = 1.0;
            }
        }

        ensure_len(&mut self.key_rotations, 4, 0.0);
        for rotation in &mut self.key_rotations {
            if !rotation.is_finite() {
                *rotation = 0.0;
            }
        }

        ensure_len(&mut self.key_shapes, 4, DEFAULT_SHAPE.to_string());
        for shape in &mut self.key_shapes {
            if shape.trim().is_empty() {
                *shape = DEFAULT_SHAPE.to_string();
            }
        }

        ensure_len(&mut self.trail_scales, 4, 1.0);
        for scale in &mut self.trail_scales {
            if !scale.is_finite() || *scale <= 0.0 {
                *scale = 1.0;
            }
        }

        ensure_len(&mut self.trail_rotations, 4, 0.0);
        for rotation in &mut self.trail_rotations {
            if !rotation.is_finite() {
                *rotation = 0.0;
            }
        }

        ensure_len(&mut self.trail_shapes, 4, DEFAULT_SHAPE.to_string());
        for shape in &mut self.trail_shapes {
            if shape.trim().is_empty() {
                *shape = DEFAULT_SHAPE.to_string();
            }
        }

        if !self.keys_bg_scale.is_finite() || self.keys_bg_scale <= 0.0 {
            self.keys_bg_scale = 1.0;
        }
        if !self.keys_bg_rotation.is_finite() {
            self.keys_bg_rotation = 0.0;
        }
        if self.keys_bg_shape.trim().is_empty() {
            self.keys_bg_shape = DEFAULT_SHAPE.to_string();
        }

        if self.hitcounter_font.trim().is_empty() {
            self.hitcounter_font = "Inter".to_string();
        }
        if self.hitcounter_border_color.trim().is_empty() {
            self.hitcounter_border_color = "#262f47".to_string();
        }
        if !self.hitcounter_scale.is_finite() || self.hitcounter_scale <= 0.0 {
            self.hitcounter_scale = 1.0;
        }
        if !self.hitcounter_label_size.is_finite() || self.hitcounter_label_size <= 0.0 {
            self.hitcounter_label_size = 10.0;
        }
        if !self.hitcounter_value_size.is_finite() || self.hitcounter_value_size <= 0.0 {
            self.hitcounter_value_size = 22.0;
        }
        if !self.hitcounter_glow_strength.is_finite() || self.hitcounter_glow_strength < 0.0 {
            self.hitcounter_glow_strength = 0.12;
        }

        ensure_len(&mut self.hitcounter_labels, 6, "MAX".to_string());
        for (i, label) in self.hitcounter_labels.iter_mut().enumerate() {
            if label.trim().is_empty() {
                *label = DEFAULT_HIT_LABELS[i].to_string();
            }
        }

        ensure_len(&mut self.hitcounter_colors, 6, "#ffffff".to_string());
        for (i, color) in self.hitcounter_colors.iter_mut().enumerate() {
            if color.trim().is_empty() {
                *color = DEFAULT_HIT_COLORS[i].to_string();
            }
        }

        ensure_len(&mut self.hitcounter_item_scales, 6, 1.0);
        for scale in &mut self.hitcounter_item_scales {
            if !scale.is_finite() || *scale <= 0.0 {
                *scale = 1.0;
            }
        }

        let (default_hit_x, default_hit_y) = default_hit_positions(
            &self.hitcounter_orientation,
            self.hitcounter_item_width,
            self.hitcounter_item_height,
            self.hitcounter_gap,
        );

        pad_or_truncate_from_defaults(&mut self.hitcounter_item_offsets_x, &default_hit_x);
        pad_or_truncate_from_defaults(&mut self.hitcounter_item_offsets_y, &default_hit_y);
        ensure_len(&mut self.hitcounter_label_offsets_x, 6, 0);
        ensure_len(&mut self.hitcounter_label_offsets_y, 6, 0);
        ensure_len(&mut self.hitcounter_value_offsets_x, 6, 0);
        ensure_len(&mut self.hitcounter_value_offsets_y, 6, 0);
        ensure_len(&mut self.hitcounter_dot_offsets_x, 6, 0);
        ensure_len(&mut self.hitcounter_dot_offsets_y, 6, 0);
    }
}

fn ensure_len<T: Clone>(values: &mut Vec<T>, len: usize, default: T) {
    if values.len() < len {
        values.resize(len, default);
    } else if values.len() > len {
        values.truncate(len);
    }
}

fn pad_or_truncate_from_defaults(values: &mut Vec<i32>, defaults: &[i32]) {
    if values.len() < defaults.len() {
        for index in values.len()..defaults.len() {
            values.push(defaults[index]);
        }
    } else if values.len() > defaults.len() {
        values.truncate(defaults.len());
    }
}

fn default_hit_positions(orientation: &str, item_width: u32, item_height: u32, gap: u32) -> (Vec<i32>, Vec<i32>) {
    let step_x = item_width as i32 + gap as i32;
    let step_y = item_height as i32 + gap as i32;
    let mut xs = Vec::with_capacity(6);
    let mut ys = Vec::with_capacity(6);

    for index in 0..6 {
        if orientation == "horizontal" {
            xs.push(index as i32 * step_x);
            ys.push(0);
        } else {
            xs.push(0);
            ys.push(index as i32 * step_y);
        }
    }

    (xs, ys)
}

lazy_static::lazy_static! {
    static ref CONFIG_CACHE: RwLock<Config> = RwLock::new(Config::default());
    static ref CONFIG_SERIALIZED_CACHE: RwLock<String> = RwLock::new(String::new());
}

pub fn config_path() -> PathBuf {
    let base = dirs_next::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("oris-mania-utils").join("config.json")
}

fn runtime_msdconverter_path(config: &Config) -> Option<PathBuf> {
    if config.tosu_root_path.trim().is_empty() {
        return None;
    }

    let tosu_root = PathBuf::from(&config.tosu_root_path);
    if !crate::bootstrap::is_valid_tosu_root(tosu_root.as_path()) {
        return None;
    }

    Some(crate::bootstrap::target_msdconverter_path(&tosu_root))
}

pub fn sync_runtime_overlay_files(config: &Config) {
    let mut normalized = config.clone();
    normalized.normalize();

    let Some(runtime_dir) = runtime_msdconverter_path(&normalized) else {
        return;
    };

    let content = match serde_json::to_string_pretty(&normalized) {
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

pub fn load_config_from_disk() -> Config {
    let path = config_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(mut config) = serde_json::from_str::<Config>(&content) {
            config.normalize();
            if let Ok(mut cache) = CONFIG_CACHE.write() {
                *cache = config.clone();
            }
            if let Ok(serialized) = serde_json::to_string_pretty(&config) {
                if let Ok(mut serialized_cache) = CONFIG_SERIALIZED_CACHE.write() {
                    *serialized_cache = serialized;
                }
            }
            return config;
        }
    }
    let config = Config::default();
    if let Ok(mut cache) = CONFIG_CACHE.write() {
        *cache = config.clone();
    }
    if let Ok(serialized) = serde_json::to_string_pretty(&config) {
        if let Ok(mut serialized_cache) = CONFIG_SERIALIZED_CACHE.write() {
            *serialized_cache = serialized;
        }
    }
    config
}

pub fn write_config(config: &Config) {
    let mut normalized = config.clone();
    normalized.normalize();

    if let Ok(content) = serde_json::to_string_pretty(&normalized) {
        let already_persisted = if let Ok(cache) = CONFIG_SERIALIZED_CACHE.read() {
            *cache == content
        } else {
            false
        };

        if let Ok(mut cache) = CONFIG_CACHE.write() {
            *cache = normalized.clone();
        }

        let path = config_path();
        let local_missing = !path.exists();
        let overlay_config_missing = runtime_msdconverter_path(&normalized)
            .map(|runtime_dir| !runtime_dir.join("config.json").exists())
            .unwrap_or(false);

        if already_persisted && !local_missing && !overlay_config_missing {
            return;
        }

        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, &content);

        sync_runtime_overlay_files(&normalized);

        if let Ok(mut serialized_cache) = CONFIG_SERIALIZED_CACHE.write() {
            *serialized_cache = content;
        }
    }
}
