use crate::config::Config;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeScope {
    Keys,
    Radar,
    Hits,
    All,
}

#[derive(Clone, Debug)]
pub struct OverlayThemePreset {
    pub id: &'static str,
    pub name: &'static str,
    pub accent: &'static str,
    pub accent_secondary: &'static str,
    pub bg: &'static str,
    pub item_bg: &'static str,
    pub border: &'static str,
    pub key_border: &'static str,
    pub text: &'static str,
    pub dan_color_mode: &'static str,
    pub hit_colors: [&'static str; 6],
    pub bg_opacity: f32,
    pub radar_fill_opacity: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportedTheme {
    pub format: String,
    pub schema_version: u32,
    pub name: String,
    pub id: String,
    pub accent: String,
    pub accent_secondary: String,
    pub bg: String,
    pub item_bg: String,
    pub border: String,
    pub key_border: String,
    pub text: String,
    pub dan_color_mode: String,
    pub hit_colors: Vec<String>,
    pub bg_opacity: f32,
    pub radar_fill_opacity: f32,
    pub show_accent_bar: bool,
    pub hit_dot_size: u32,
    pub hit_glow_strength: f32,
}

impl Default for ExportedTheme {
    fn default() -> Self {
        Self {
            format: "oris-mania-theme".to_string(),
            schema_version: 1,
            name: "custom theme".to_string(),
            id: "custom".to_string(),
            accent: "#e588a6".to_string(),
            accent_secondary: "#d47695".to_string(),
            bg: "#141415".to_string(),
            item_bg: "#19191b".to_string(),
            border: "#28282b".to_string(),
            key_border: "#28282b".to_string(),
            text: "#a1a1a6".to_string(),
            dan_color_mode: "accent".to_string(),
            hit_colors: vec![
                "#ededed".to_string(),
                "#e5b567".to_string(),
                "#4cc38a".to_string(),
                "#6cb6ff".to_string(),
                "#d291e4".to_string(),
                "#e5484d".to_string(),
            ],
            bg_opacity: 0.92,
            radar_fill_opacity: 0.22,
            show_accent_bar: false,
            hit_dot_size: 0,
            hit_glow_strength: 0.0,
        }
    }
}

impl ExportedTheme {
    pub fn normalize(&mut self) {
        if self.format.trim().is_empty() {
            self.format = "oris-mania-theme".to_string();
        }
        if self.name.trim().is_empty() {
            self.name = "imported theme".to_string();
        }
        if self.id.trim().is_empty() {
            self.id = self.name.to_lowercase().replace(' ', "-");
        }
        let d = Self::default();
        if self.accent.trim().is_empty() {
            self.accent = d.accent;
        }
        if self.accent_secondary.trim().is_empty() {
            self.accent_secondary = self.accent.clone();
        }
        if self.bg.trim().is_empty() {
            self.bg = d.bg;
        }
        if self.item_bg.trim().is_empty() {
            self.item_bg = d.item_bg;
        }
        if self.border.trim().is_empty() {
            self.border = d.border;
        }
        if self.key_border.trim().is_empty() {
            self.key_border = self.border.clone();
        }
        if self.text.trim().is_empty() {
            self.text = d.text;
        }
        if self.dan_color_mode.trim().is_empty() {
            self.dan_color_mode = "accent".to_string();
        }
        while self.hit_colors.len() < 6 {
            let idx = self.hit_colors.len();
            self.hit_colors.push(d.hit_colors[idx].clone());
        }
        if self.hit_colors.len() > 6 {
            self.hit_colors.truncate(6);
        }
        if !self.bg_opacity.is_finite() {
            self.bg_opacity = 0.92;
        }
        self.bg_opacity = self.bg_opacity.clamp(0.05, 1.0);
        if !self.radar_fill_opacity.is_finite() {
            self.radar_fill_opacity = 0.22;
        }
        self.radar_fill_opacity = self.radar_fill_opacity.clamp(0.0, 1.0);
        if !self.hit_glow_strength.is_finite() {
            self.hit_glow_strength = 0.0;
        }
        self.hit_glow_strength = self.hit_glow_strength.clamp(0.0, 1.0);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfigBackup {
    pub format: String,
    pub schema_version: u32,
    pub config: Config,
}

impl ConfigBackup {
    pub fn from_config(cfg: &Config) -> Self {
        Self {
            format: "oris-mania-backup".to_string(),
            schema_version: crate::config::CONFIG_VERSION,
            config: cfg.clone(),
        }
    }
}

impl Default for ConfigBackup {
    fn default() -> Self {
        Self {
            format: "oris-mania-backup".to_string(),
            schema_version: crate::config::CONFIG_VERSION,
            config: Config::default(),
        }
    }
}

pub const BUILTIN_THEMES: &[OverlayThemePreset] = &[
    OverlayThemePreset {
        id: "rose-graphite",
        name: "rose",
        accent: "#e588a6",
        accent_secondary: "#d47695",
        bg: "#141415",
        item_bg: "#19191b",
        border: "#28282b",
        key_border: "#28282b",
        text: "#a1a1a6",
        dan_color_mode: "accent",
        hit_colors: [
            "#ededed", "#e5b567", "#4cc38a", "#6cb6ff", "#d291e4", "#e5484d",
        ],
        bg_opacity: 0.92,
        radar_fill_opacity: 0.22,
    },
    OverlayThemePreset {
        id: "sober-cobalt",
        name: "cobalt",
        accent: "#7aa2f7",
        accent_secondary: "#5c85e6",
        bg: "#12141a",
        item_bg: "#191c24",
        border: "#262b38",
        key_border: "#2c3242",
        text: "#9ca3b8",
        dan_color_mode: "accent",
        hit_colors: [
            "#f1f5f9", "#7aa2f7", "#38bdf8", "#818cf8", "#a78bfa", "#f87171",
        ],
        bg_opacity: 0.92,
        radar_fill_opacity: 0.24,
    },
    OverlayThemePreset {
        id: "amber-warm",
        name: "amber",
        accent: "#e5a55d",
        accent_secondary: "#cf904a",
        bg: "#161513",
        item_bg: "#1f1c17",
        border: "#2e2a24",
        key_border: "#363028",
        text: "#a89f91",
        dan_color_mode: "accent",
        hit_colors: [
            "#fef3c7", "#fbbf24", "#e5a55d", "#6ee7b7", "#93c5fd", "#f87171",
        ],
        bg_opacity: 0.92,
        radar_fill_opacity: 0.24,
    },
    OverlayThemePreset {
        id: "monochrome",
        name: "monochrome",
        accent: "#ededed",
        accent_secondary: "#b8b8c0",
        bg: "#0f0f10",
        item_bg: "#171719",
        border: "#2a2a2d",
        key_border: "#323236",
        text: "#9a9aa0",
        dan_color_mode: "accent",
        hit_colors: [
            "#ffffff", "#d4d4d8", "#a1a1aa", "#71717a", "#52525b", "#ef4444",
        ],
        bg_opacity: 0.92,
        radar_fill_opacity: 0.18,
    },
    OverlayThemePreset {
        id: "cyan-prism",
        name: "cyan",
        accent: "#00ffd5",
        accent_secondary: "#38bdf8",
        bg: "#0d1217",
        item_bg: "#121a22",
        border: "#1e293b",
        key_border: "#26364d",
        text: "#94a3b8",
        dan_color_mode: "accent",
        hit_colors: [
            "#ecfeff", "#00ffd5", "#38bdf8", "#818cf8", "#c084fc", "#fb7185",
        ],
        bg_opacity: 0.90,
        radar_fill_opacity: 0.28,
    },
    OverlayThemePreset {
        id: "velvet-dusk",
        name: "violet",
        accent: "#c084fc",
        accent_secondary: "#e879f9",
        bg: "#14101b",
        item_bg: "#1c1626",
        border: "#2e2440",
        key_border: "#3a2d52",
        text: "#a79ab8",
        dan_color_mode: "accent",
        hit_colors: [
            "#faf5ff", "#e879f9", "#c084fc", "#818cf8", "#60a5fa", "#fb7185",
        ],
        bg_opacity: 0.92,
        radar_fill_opacity: 0.26,
    },
    OverlayThemePreset {
        id: "crimson-reform",
        name: "crimson",
        accent: "#ff5a5a",
        accent_secondary: "#fb7185",
        bg: "#121113",
        item_bg: "#1a1719",
        border: "#332428",
        key_border: "#402c32",
        text: "#a6999d",
        dan_color_mode: "tier",
        hit_colors: [
            "#ffffff", "#fb7185", "#fda4af", "#f59e0b", "#c084fc", "#ef4444",
        ],
        bg_opacity: 0.93,
        radar_fill_opacity: 0.24,
    },
    OverlayThemePreset {
        id: "emerald-matcha",
        name: "sage",
        accent: "#4cc38a",
        accent_secondary: "#34d399",
        bg: "#101613",
        item_bg: "#16201b",
        border: "#23332b",
        key_border: "#2c4036",
        text: "#95aba0",
        dan_color_mode: "accent",
        hit_colors: [
            "#ecfdf5", "#6ee7b7", "#34d399", "#38bdf8", "#a78bfa", "#f87171",
        ],
        bg_opacity: 0.92,
        radar_fill_opacity: 0.24,
    },
    OverlayThemePreset {
        id: "obsidian-gold",
        name: "gold",
        accent: "#fbbf24",
        accent_secondary: "#f59e0b",
        bg: "#14120e",
        item_bg: "#1e1b14",
        border: "#383020",
        key_border: "#473c28",
        text: "#aba290",
        dan_color_mode: "accent",
        hit_colors: [
            "#fffbeb", "#fde047", "#fbbf24", "#60a5fa", "#c084fc", "#ef4444",
        ],
        bg_opacity: 0.93,
        radar_fill_opacity: 0.24,
    },
    OverlayThemePreset {
        id: "arctic-slate",
        name: "slate",
        accent: "#38bdf8",
        accent_secondary: "#818cf8",
        bg: "#0f1318",
        item_bg: "#161c24",
        border: "#243040",
        key_border: "#2d3c50",
        text: "#94a3b8",
        dan_color_mode: "accent",
        hit_colors: [
            "#f0f9ff", "#38bdf8", "#34d399", "#818cf8", "#c084fc", "#fb7185",
        ],
        bg_opacity: 0.88,
        radar_fill_opacity: 0.26,
    },
];

pub fn find_theme(id: &str) -> Option<&'static OverlayThemePreset> {
    BUILTIN_THEMES.iter().find(|t| t.id == id || t.name == id)
}

pub fn apply_preset_to_keys(cfg: &mut Config, t: &OverlayThemePreset) {
    cfg.keys_theme_id = t.id.to_string();
    cfg.per_key_colors = false;
    cfg.key_color_outer = t.accent.to_string();
    cfg.key_color_inner = t.accent_secondary.to_string();
    cfg.key_colors = vec![
        t.accent.to_string(),
        t.accent_secondary.to_string(),
        t.accent_secondary.to_string(),
        t.accent.to_string(),
    ];
    cfg.keys_bg_color = t.bg.to_string();
    cfg.key_inactive_color = t.item_bg.to_string();
    cfg.keys_border_color = t.border.to_string();
    cfg.key_border_color = t.key_border.to_string();
    cfg.keys_bg_opacity = t.bg_opacity;
}

pub fn apply_preset_to_radar(cfg: &mut Config, t: &OverlayThemePreset) {
    cfg.radar_theme_id = t.id.to_string();
    cfg.accent_color = t.accent.to_string();
    cfg.radar_bg_color = t.bg.to_string();
    cfg.radar_border_color = t.border.to_string();
    cfg.radar_text_color = t.text.to_string();
    cfg.radar_dan_badge_bg = t.item_bg.to_string();
    cfg.radar_dan_color_mode = t.dan_color_mode.to_string();
    cfg.radar_dan_color = t.accent.to_string();
    cfg.radar_graph_color = t.accent.to_string();
    cfg.radar_graph_color_mode = if t.dan_color_mode == "tier" {
        "dan".to_string()
    } else {
        "accent".to_string()
    };
    cfg.bg_opacity = t.bg_opacity;
    cfg.radar_fill_opacity = t.radar_fill_opacity;
}

pub fn apply_preset_to_hits(cfg: &mut Config, t: &OverlayThemePreset) {
    cfg.hits_theme_id = t.id.to_string();
    cfg.hitcounter_bg_color = t.bg.to_string();
    cfg.hitcounter_item_bg_color = t.item_bg.to_string();
    cfg.hitcounter_border_color = t.border.to_string();
    cfg.hitcounter_item_border_color = t.key_border.to_string();
    cfg.hitcounter_text_color = t.text.to_string();
    cfg.hitcounter_opacity = t.bg_opacity;
    cfg.hitcounter_colors = t.hit_colors.iter().map(|s| s.to_string()).collect();
}

pub fn apply_preset(cfg: &mut Config, scope: ThemeScope, t: &OverlayThemePreset) {
    if cfg.link_overlay_themes || scope == ThemeScope::All {
        cfg.active_theme_id = t.id.to_string();
        apply_preset_to_keys(cfg, t);
        apply_preset_to_radar(cfg, t);
        apply_preset_to_hits(cfg, t);
    } else {
        match scope {
            ThemeScope::Keys => apply_preset_to_keys(cfg, t),
            ThemeScope::Radar => apply_preset_to_radar(cfg, t),
            ThemeScope::Hits => apply_preset_to_hits(cfg, t),
            ThemeScope::All => {}
        }
    }
}

pub fn export_theme_from_config(cfg: &Config, scope: ThemeScope) -> ExportedTheme {
    match scope {
        ThemeScope::Keys => ExportedTheme {
            name: cfg.keys_theme_id.clone(),
            id: cfg.keys_theme_id.clone(),
            accent: cfg.key_color_outer.clone(),
            accent_secondary: cfg.key_color_inner.clone(),
            bg: cfg.keys_bg_color.clone(),
            item_bg: cfg.key_inactive_color.clone(),
            border: cfg.keys_border_color.clone(),
            key_border: cfg.key_border_color.clone(),
            text: cfg.radar_text_color.clone(),
            dan_color_mode: cfg.radar_dan_color_mode.clone(),
            hit_colors: cfg.hitcounter_colors.clone(),
            bg_opacity: cfg.keys_bg_opacity,
            radar_fill_opacity: cfg.radar_fill_opacity,
            show_accent_bar: cfg.keys_show_accent_bar,
            hit_dot_size: cfg.hitcounter_dot_size,
            hit_glow_strength: cfg.hitcounter_glow_strength,
            ..Default::default()
        },
        ThemeScope::Radar => ExportedTheme {
            name: cfg.radar_theme_id.clone(),
            id: cfg.radar_theme_id.clone(),
            accent: cfg.accent_color.clone(),
            accent_secondary: cfg.key_color_inner.clone(),
            bg: cfg.radar_bg_color.clone(),
            item_bg: cfg.radar_dan_badge_bg.clone(),
            border: cfg.radar_border_color.clone(),
            key_border: cfg.radar_border_color.clone(),
            text: cfg.radar_text_color.clone(),
            dan_color_mode: cfg.radar_dan_color_mode.clone(),
            hit_colors: cfg.hitcounter_colors.clone(),
            bg_opacity: cfg.bg_opacity,
            radar_fill_opacity: cfg.radar_fill_opacity,
            show_accent_bar: cfg.hitcounter_show_accent_bar,
            hit_dot_size: cfg.hitcounter_dot_size,
            hit_glow_strength: cfg.hitcounter_glow_strength,
            ..Default::default()
        },
        ThemeScope::Hits => ExportedTheme {
            name: cfg.hits_theme_id.clone(),
            id: cfg.hits_theme_id.clone(),
            accent: cfg
                .hitcounter_colors
                .get(1)
                .cloned()
                .unwrap_or_else(|| cfg.accent_color.clone()),
            accent_secondary: cfg
                .hitcounter_colors
                .get(2)
                .cloned()
                .unwrap_or_else(|| cfg.key_color_inner.clone()),
            bg: cfg.hitcounter_bg_color.clone(),
            item_bg: cfg.hitcounter_item_bg_color.clone(),
            border: cfg.hitcounter_border_color.clone(),
            key_border: cfg.hitcounter_item_border_color.clone(),
            text: cfg.hitcounter_text_color.clone(),
            dan_color_mode: cfg.radar_dan_color_mode.clone(),
            hit_colors: cfg.hitcounter_colors.clone(),
            bg_opacity: cfg.hitcounter_opacity,
            radar_fill_opacity: cfg.radar_fill_opacity,
            show_accent_bar: cfg.hitcounter_show_accent_bar,
            hit_dot_size: cfg.hitcounter_dot_size,
            hit_glow_strength: cfg.hitcounter_glow_strength,
            ..Default::default()
        },
        ThemeScope::All => ExportedTheme {
            name: cfg.active_theme_id.clone(),
            id: cfg.active_theme_id.clone(),
            accent: cfg.accent_color.clone(),
            accent_secondary: cfg.key_color_inner.clone(),
            bg: cfg.radar_bg_color.clone(),
            item_bg: cfg.hitcounter_item_bg_color.clone(),
            border: cfg.radar_border_color.clone(),
            key_border: cfg.key_border_color.clone(),
            text: cfg.radar_text_color.clone(),
            dan_color_mode: cfg.radar_dan_color_mode.clone(),
            hit_colors: cfg.hitcounter_colors.clone(),
            bg_opacity: cfg.bg_opacity,
            radar_fill_opacity: cfg.radar_fill_opacity,
            show_accent_bar: cfg.hitcounter_show_accent_bar,
            hit_dot_size: cfg.hitcounter_dot_size,
            hit_glow_strength: cfg.hitcounter_glow_strength,
            ..Default::default()
        },
    }
}

pub fn apply_exported_theme(cfg: &mut Config, theme: &ExportedTheme, scope: ThemeScope) {
    let mut t = theme.clone();
    t.normalize();
    let apply_keys = |c: &mut Config| {
        c.keys_theme_id = t.id.clone();
        c.per_key_colors = false;
        c.key_color_outer = t.accent.clone();
        c.key_color_inner = t.accent_secondary.clone();
        c.key_colors = vec![
            t.accent.clone(),
            t.accent_secondary.clone(),
            t.accent_secondary.clone(),
            t.accent.clone(),
        ];
        c.keys_bg_color = t.bg.clone();
        c.key_inactive_color = t.item_bg.clone();
        c.keys_border_color = t.border.clone();
        c.key_border_color = t.key_border.clone();
        c.keys_bg_opacity = t.bg_opacity;
        c.keys_show_accent_bar = t.show_accent_bar;
    };
    let apply_radar = |c: &mut Config| {
        c.radar_theme_id = t.id.clone();
        c.accent_color = t.accent.clone();
        c.radar_bg_color = t.bg.clone();
        c.radar_border_color = t.border.clone();
        c.radar_text_color = t.text.clone();
        c.radar_dan_badge_bg = t.item_bg.clone();
        c.radar_dan_color_mode = t.dan_color_mode.clone();
        c.radar_dan_color = t.accent.clone();
        c.radar_graph_color = t.accent.clone();
        c.bg_opacity = t.bg_opacity;
        c.radar_fill_opacity = t.radar_fill_opacity;
    };
    let apply_hits = |c: &mut Config| {
        c.hits_theme_id = t.id.clone();
        c.hitcounter_bg_color = t.bg.clone();
        c.hitcounter_item_bg_color = t.item_bg.clone();
        c.hitcounter_border_color = t.border.clone();
        c.hitcounter_item_border_color = t.key_border.clone();
        c.hitcounter_text_color = t.text.clone();
        c.hitcounter_opacity = t.bg_opacity;
        c.hitcounter_colors = t.hit_colors.clone();
        c.hitcounter_show_accent_bar = t.show_accent_bar;
        c.hitcounter_dot_size = t.hit_dot_size;
        c.hitcounter_glow_strength = t.hit_glow_strength;
    };

    if cfg.link_overlay_themes || scope == ThemeScope::All {
        cfg.active_theme_id = t.id.clone();
        apply_keys(cfg);
        apply_radar(cfg);
        apply_hits(cfg);
    } else {
        match scope {
            ThemeScope::Keys => apply_keys(cfg),
            ThemeScope::Radar => apply_radar(cfg),
            ThemeScope::Hits => apply_hits(cfg),
            ThemeScope::All => {}
        }
    }
}
