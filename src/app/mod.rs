mod hits_tab;
mod input;
mod keys_tab;
mod radar_tab;
mod setup_tab;
mod widgets;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Pos2, Rounding, Stroke};

use crate::bootstrap::{
    detect_osu_songs_path, detect_tosu_root, extract_embedded_overlay_files, is_valid_tosu_root,
    resolve_tosu_static_dir,
};
use crate::calc::{rebuild_songs_index, SkillRatings};
use crate::config::{load_config_from_disk, write_config, Config};
use crate::keys::{LIVE_KEY_HOLD_MS, LIVE_KEY_STATES};
use crate::proxy::{LIVE_HITS_SEQ, LIVE_OSU_STATE, LIVE_RATINGS_SEQ, TOSU_CONNECTED};
use crate::theme::ThemeScope;
use crate::tosu_manager::{
    ensure_tosu_overlay_env, fetch_latest_tosu_version, get_daniel_status, get_minacalc_status,
    get_tosu_client_status_standalone, managed_tosu_dir_standalone, read_tosu_env_network,
    TosuInstallProgress,
};

use widgets::{
    parse_hex_color, COLOR_ACCENT, COLOR_BG_CANVAS, COLOR_BG_CARD, COLOR_BG_DEEP, COLOR_BG_SURFACE,
    COLOR_BORDER, COLOR_DANGER, COLOR_TEXT_PRIMARY, COLOR_TEXT_SECONDARY, COLOR_WARN,
};

#[derive(PartialEq, Clone, Copy, Debug)]
pub(in crate::app) enum AppTab {
    Keys,
    Radar,
    Hits,
    Setup,
}

pub struct App {
    pub(in crate::app) tab: AppTab,
    pub(in crate::app) config: Config,
    pub(in crate::app) config_dirty: bool,
    pub(in crate::app) last_change: Instant,

    pub(in crate::app) key_rebind_idx: Option<usize>,
    pub(in crate::app) key_pressed: [bool; 4],
    pub(in crate::app) key_hold_ms: [u64; 4],
    pub(in crate::app) test_hold_preview: bool,

    pub(in crate::app) radar_skills: SkillRatings,
    pub(in crate::app) radar_map_title: String,
    pub(in crate::app) radar_map_artist: String,
    pub(in crate::app) radar_map_diff: String,
    pub(in crate::app) radar_bg_tex: Option<(String, egui::TextureHandle)>,
    pub(in crate::app) last_ratings_seq: u64,

    pub(in crate::app) hit_counts: [u32; 6],
    pub(in crate::app) last_hits_seq: u64,

    pub(in crate::app) tosu_running: bool,
    pub(in crate::app) tosu_installed: bool,
    pub(in crate::app) tosu_installed_ver: String,
    pub(in crate::app) tosu_latest_ver: Arc<Mutex<Option<String>>>,
    pub(in crate::app) install_progress: Arc<Mutex<Option<TosuInstallProgress>>>,
    pub(in crate::app) installing: bool,
    pub(in crate::app) minacalc_ver: String,
    pub(in crate::app) daniel_ver: String,
    pub(in crate::app) engine_dl_msg: Arc<Mutex<Option<String>>>,
    pub(in crate::app) downloading_engine: bool,
    pub(in crate::app) last_poll: Instant,

    pub(in crate::app) float_keys: bool,
    pub(in crate::app) float_radar: bool,
    pub(in crate::app) float_hits: bool,

    pub(in crate::app) theme_popup: Option<(ThemeScope, Pos2, f32)>,
    pub(in crate::app) logo_tex: Option<egui::TextureHandle>,
    pub(in crate::app) last_visuals_accent: Option<Color32>,
    pub(in crate::app) toast: Option<(String, Instant)>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let mut config = load_config_from_disk();
        let mut changed = false;

        if config.osu_songs_path.is_empty() {
            if let Some(songs) = detect_osu_songs_path() {
                config.osu_songs_path = songs.to_string_lossy().to_string();
                changed = true;
            }
        }
        if !config.osu_songs_path.is_empty() {
            rebuild_songs_index(&config.osu_songs_path);
        }

        if config.tosu_root_path.is_empty() {
            if let Some(tosu) = detect_tosu_root(None) {
                config.tosu_root_path = tosu.to_string_lossy().to_string();
                changed = true;
            }
        }

        if !config.tosu_root_path.is_empty() {
            let root = PathBuf::from(&config.tosu_root_path);
            if is_valid_tosu_root(&root) {
                let (env_host, env_port) = read_tosu_env_network(&root);
                if let Some(h) = env_host {
                    if config.tosu_host == "127.0.0.1" && h != config.tosu_host {
                        config.tosu_host = h;
                        changed = true;
                    }
                }
                if let Some(p) = env_port {
                    if config.tosu_port == 24050 && p != config.tosu_port {
                        config.tosu_port = p;
                        changed = true;
                    }
                }
                if config.enable_browser_overlays {
                    ensure_tosu_overlay_env(&root);
                    let static_dir = resolve_tosu_static_dir(&root);
                    let _ = fs::create_dir_all(&static_dir);
                    let _ = extract_embedded_overlay_files(&static_dir);
                }
            }
        }

        if changed {
            write_config(&config);
        } else {
            crate::config::sync_runtime_overlay_files(&config);
        }

        let latest_ver_arc = Arc::new(Mutex::new(None));
        {
            let arc_clone = Arc::clone(&latest_ver_arc);
            std::thread::spawn(move || {
                if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    if let Some(tag) = rt.block_on(fetch_latest_tosu_version()) {
                        if let Ok(mut lock) = arc_clone.lock() {
                            *lock = Some(tag);
                        }
                    }
                }
            });
        }

        let initial_skills = SkillRatings {
            overall: 26.50,
            stream: 28.20,
            jumpstream: 25.10,
            handstream: 23.40,
            stamina: 26.90,
            jackspeed: 22.80,
            chordjack: 21.50,
            technical: 24.30,
            map_type: "Stream Map".to_string(),
            dan: crate::dan::DanResult {
                level: 13.42,
                tier: 13,
                symbol: "γ".to_string(),
                name: "gamma".to_string(),
                variant: "".to_string(),
                sub_label: "mid".to_string(),
                display: "γ · gamma mid".to_string(),
                xdc1_dan: 13.55,
                xdc1_points: 34.20,
                xdc1_rice_points: 33.80,
                companella_dan: 13.21,
                interlude: 7.42,
                sunny: 8.15,
                strain_curve: vec![
                    0.25, 0.35, 0.55, 0.48, 0.72, 0.88, 0.64, 0.42, 0.68, 0.94, 0.78, 0.52, 0.82,
                    0.60, 0.34,
                ],
                duration_ms: 120000,
            },
            bg_path: String::new(),
            adaptive_accent: String::new(),
            adaptive_accent_secondary: String::new(),
            adaptive_bg: String::new(),
            adaptive_item_bg: String::new(),
            adaptive_border: String::new(),
        };

        let mc_status = get_minacalc_status();
        let dan_status = get_daniel_status();

        let fk = config.float_keys_open;
        let fr = config.float_radar_open;
        let fh = config.float_hits_open;
        let mut app = Self {
            tab: AppTab::Keys,
            config,
            config_dirty: false,
            last_change: Instant::now(),

            key_rebind_idx: None,
            key_pressed: [false; 4],
            key_hold_ms: [0; 4],
            test_hold_preview: false,

            radar_skills: initial_skills,
            radar_map_title: "Frontier Explorer".to_string(),
            radar_map_artist: "kiwi_iwata".to_string(),
            radar_map_diff: "Another".to_string(),
            radar_bg_tex: None,
            last_ratings_seq: 0,

            hit_counts: [482, 94, 18, 5, 1, 3],
            last_hits_seq: 0,

            tosu_running: false,
            tosu_installed: false,
            tosu_installed_ver: String::new(),
            tosu_latest_ver: latest_ver_arc,
            install_progress: Arc::new(Mutex::new(None)),
            installing: false,
            minacalc_ver: mc_status.installed_version,
            daniel_ver: dan_status.installed_version,
            engine_dl_msg: Arc::new(Mutex::new(None)),
            downloading_engine: false,
            last_poll: Instant::now() - Duration::from_secs(10),

            float_keys: fk,
            float_radar: fr,
            float_hits: fh,

            theme_popup: None,
            logo_tex: None,
            last_visuals_accent: None,
            toast: None,
        };

        app.poll_tosu_status();
        app
    }

    pub(super) fn set_toast(&mut self, msg: &str) {
        self.toast = Some((msg.to_string(), Instant::now()));
    }

    pub(super) fn mark_dirty(&mut self) {
        crate::config::update_config_cache(&self.config);
        self.config_dirty = true;
        self.last_change = Instant::now();
    }

    pub(super) fn poll_tosu_status(&mut self) {
        let status = get_tosu_client_status_standalone();
        self.tosu_installed = status.installed;
        self.tosu_running = status.running;
        self.tosu_installed_ver = status.installed_version;
        if status.installed && self.config.tosu_root_path.is_empty() && !status.path.is_empty() {
            self.config.tosu_root_path = status.path;
            self.mark_dirty();
        }
        self.last_poll = Instant::now();
    }

    pub(super) fn sync_overlays(&mut self) {
        if !self.config.enable_browser_overlays {
            self.set_toast("browser overlays disabled (enable in setup for obs)");
            return;
        }
        let tosu_root = if !self.config.tosu_root_path.is_empty() {
            PathBuf::from(&self.config.tosu_root_path)
        } else if let Some(detected) = detect_tosu_root(None) {
            self.config.tosu_root_path = detected.to_string_lossy().to_string();
            detected
        } else {
            managed_tosu_dir_standalone()
        };

        ensure_tosu_overlay_env(&tosu_root);
        let static_dir = resolve_tosu_static_dir(&tosu_root);
        match fs::create_dir_all(&static_dir) {
            Ok(_) => match extract_embedded_overlay_files(&static_dir) {
                Ok(count) => {
                    write_config(&self.config);
                    self.set_toast(&format!("synced {} overlay files to tosu/static", count));
                }
                Err(err) => {
                    self.set_toast(&format!("sync error: {}", err));
                }
            },
            Err(err) => {
                self.set_toast(&format!("folder error: {}", err));
            }
        }
    }

    fn sync_live_state(&mut self, ctx: &egui::Context) {
        let mut needs_repaint = false;

        for i in 0..4 {
            let down = LIVE_KEY_STATES[i].load(Ordering::Relaxed);
            let ms = LIVE_KEY_HOLD_MS[i].load(Ordering::Relaxed);
            if down != self.key_pressed[i] {
                self.key_pressed[i] = down;
                needs_repaint = true;
            }
            if ms != self.key_hold_ms[i] {
                self.key_hold_ms[i] = ms;
                needs_repaint = true;
            }
        }

        let seq = LIVE_RATINGS_SEQ.load(Ordering::Relaxed);
        let hits_seq = LIVE_HITS_SEQ.load(Ordering::Relaxed);
        let live_connected = TOSU_CONNECTED.load(Ordering::Relaxed);
        if let Ok(live) = LIVE_OSU_STATE.read() {
            if !live.song_title.is_empty()
                && (live.song_title != self.radar_map_title
                    || live.song_artist != self.radar_map_artist
                    || live.song_difficulty != self.radar_map_diff)
            {
                self.radar_map_title = live.song_title.clone();
                self.radar_map_artist = live.song_artist.clone();
                self.radar_map_diff = live.song_difficulty.clone();
                needs_repaint = true;
            }
            if let Some(ratings) = &live.ratings {
                if seq != self.last_ratings_seq
                    || (ratings.overall - self.radar_skills.overall).abs() > 0.001
                    || (ratings.dan.level - self.radar_skills.dan.level).abs() > 0.001
                {
                    self.radar_skills = ratings.clone();
                    self.last_ratings_seq = seq;
                    needs_repaint = true;
                }
            }
            if self.radar_skills.bg_path != live.bg_path {
                self.radar_skills.bg_path = live.bg_path.clone();
                needs_repaint = true;
            }
            if (live_connected || self.tosu_running)
                && (hits_seq != self.last_hits_seq || live.hits != self.hit_counts)
            {
                self.hit_counts = live.hits;
                self.last_hits_seq = hits_seq;
                needs_repaint = true;
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(mut shared) = crate::linux_overlay::OVERLAY_SHARED.lock() {
                shared.float_keys = self.float_keys;
                shared.float_radar = self.float_radar;
                shared.float_hits = self.float_hits;
                shared.title = self.radar_map_title.clone();
                shared.artist = self.radar_map_artist.clone();
                shared.diff = self.radar_map_diff.clone();
                shared.ratings = self.radar_skills.clone();
                shared.hit_counts = self.hit_counts;

                let mut dragged_any = false;
                if let Some((x, y)) = shared.dragged_pos[0].take() {
                    self.config.keys_pos_x = x;
                    self.config.keys_pos_y = y;
                    dragged_any = true;
                }
                if let Some((x, y)) = shared.dragged_pos[1].take() {
                    self.config.radar_pos_x = x;
                    self.config.radar_pos_y = y;
                    dragged_any = true;
                }
                if let Some((x, y)) = shared.dragged_pos[2].take() {
                    self.config.hitcounter_position_x = x;
                    self.config.hitcounter_position_y = y;
                    dragged_any = true;
                }
                if dragged_any {
                    drop(shared);
                    self.mark_dirty();
                    needs_repaint = true;
                }
            }
        }

        #[cfg(target_os = "windows")]
        let win_float = self.float_keys || self.float_radar || self.float_hits;
        #[cfg(not(target_os = "windows"))]
        let win_float = false;

        if needs_repaint || self.key_pressed.iter().any(|&p| p) || win_float {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else if live_connected {
            ctx.request_repaint_after(Duration::from_millis(150));
        } else {
            ctx.request_repaint_after(Duration::from_millis(400));
        }
    }

    fn handle_frameless_resize(&mut self, ctx: &egui::Context) {
        let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let screen = ctx.screen_rect();
        if !is_max
            && screen.width() >= 760.0
            && screen.height() >= 500.0
            && ((self.config.window_width - screen.width()).abs() > 2.0
                || (self.config.window_height - screen.height()).abs() > 2.0)
        {
            self.config.window_width = screen.width().clamp(760.0, 3840.0);
            self.config.window_height = screen.height().clamp(500.0, 2160.0);
            self.mark_dirty();
        }
        if is_max {
            return;
        }
        let (pos_opt, pressed) =
            ctx.input(|i| (i.pointer.hover_pos(), i.pointer.primary_pressed()));
        let Some(pos) = pos_opt else { return };
        let m = 6.0;
        let left = pos.x <= screen.min.x + m;
        let right = pos.x >= screen.max.x - m;
        let top = pos.y <= screen.min.y + m;
        let bottom = pos.y >= screen.max.y - m;
        let dir = match (left, right, top, bottom) {
            (true, _, true, _) => Some(egui::ResizeDirection::NorthWest),
            (_, true, true, _) => Some(egui::ResizeDirection::NorthEast),
            (true, _, _, true) => Some(egui::ResizeDirection::SouthWest),
            (_, true, _, true) => Some(egui::ResizeDirection::SouthEast),
            (true, _, _, _) => Some(egui::ResizeDirection::West),
            (_, true, _, _) => Some(egui::ResizeDirection::East),
            (_, _, true, _) => Some(egui::ResizeDirection::North),
            (_, _, _, true) => Some(egui::ResizeDirection::South),
            _ => None,
        };
        if let Some(d) = dir {
            let cursor = match d {
                egui::ResizeDirection::North | egui::ResizeDirection::South => {
                    egui::CursorIcon::ResizeVertical
                }
                egui::ResizeDirection::East | egui::ResizeDirection::West => {
                    egui::CursorIcon::ResizeHorizontal
                }
                egui::ResizeDirection::NorthWest | egui::ResizeDirection::SouthEast => {
                    egui::CursorIcon::ResizeNwSe
                }
                egui::ResizeDirection::NorthEast | egui::ResizeDirection::SouthWest => {
                    egui::CursorIcon::ResizeNeSw
                }
            };
            ctx.set_cursor_icon(cursor);
            if pressed {
                ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(d));
            }
        }
    }
}

impl eframe::App for App {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        write_config(&self.config);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.config_dirty && self.last_change.elapsed() >= Duration::from_millis(120) {
            write_config(&self.config);
            crate::keys::update_active_keys(&self.config.keys);
            if self.config.enable_browser_overlays || crate::KEY_SENDER.receiver_count() > 0 {
                if let Ok(mut val) = serde_json::to_value(&self.config) {
                    if let Some(obj) = val.as_object_mut() {
                        obj.insert("event".to_string(), serde_json::json!("config-updated"));
                        obj.insert(
                            "active_companion_port".to_string(),
                            serde_json::json!(crate::config::runtime_active_companion_port()),
                        );
                        obj.insert(
                            "changed_scopes".to_string(),
                            serde_json::json!(["radar", "hits", "keys"]),
                        );
                    }
                    if let Ok(msg) = serde_json::to_string(&val) {
                        let _ = crate::KEY_SENDER.send(msg);
                    }
                }
            }
            self.config_dirty = false;
        }

        if self.last_poll.elapsed() >= Duration::from_millis(2500) {
            self.poll_tosu_status();
        }

        if self.installing {
            let mut finished_msg: Option<(bool, String)> = None;
            if let Ok(mut lock) = self.install_progress.lock() {
                if let Some(p) = lock.clone() {
                    if p.stage == "complete" || p.stage == "error" {
                        *lock = None;
                        finished_msg = Some((p.stage == "error", p.message));
                    }
                }
            }
            if let Some((is_err, msg)) = finished_msg {
                self.installing = false;
                self.poll_tosu_status();
                if is_err {
                    self.set_toast(&format!("tosu install failed: {}", msg));
                } else {
                    self.set_toast(&msg);
                }
            }
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        if self.downloading_engine {
            let mut done_msg: Option<String> = None;
            if let Ok(mut lock) = self.engine_dl_msg.lock() {
                done_msg = lock.take();
            }
            if let Some(msg) = done_msg {
                self.downloading_engine = false;
                self.minacalc_ver = get_minacalc_status().installed_version;
                self.daniel_ver = get_daniel_status().installed_version;
                self.set_toast(&msg);
            }
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        let accent = parse_hex_color(&self.config.accent_color, COLOR_ACCENT);

        if self.last_visuals_accent != Some(accent) {
            self.last_visuals_accent = Some(accent);
            ctx.set_visuals(egui::Visuals {
                dark_mode: true,
                override_text_color: Some(COLOR_TEXT_PRIMARY),
                panel_fill: COLOR_BG_CANVAS,
                window_fill: COLOR_BG_CARD,
                faint_bg_color: COLOR_BG_SURFACE,
                extreme_bg_color: COLOR_BG_DEEP,
                code_bg_color: COLOR_BG_SURFACE,
                warn_fg_color: COLOR_WARN,
                error_fg_color: COLOR_DANGER,
                window_rounding: Rounding::same(10.0),
                window_stroke: Stroke::new(1.0_f32, COLOR_BORDER),
                widgets: egui::style::Widgets {
                    noninteractive: egui::style::WidgetVisuals {
                        bg_fill: COLOR_BG_CARD,
                        weak_bg_fill: COLOR_BG_CARD,
                        bg_stroke: Stroke::new(1.0_f32, COLOR_BORDER),
                        rounding: Rounding::same(6.0),
                        fg_stroke: Stroke::new(1.0_f32, COLOR_TEXT_PRIMARY),
                        expansion: 0.0,
                    },
                    inactive: egui::style::WidgetVisuals {
                        bg_fill: COLOR_BG_SURFACE,
                        weak_bg_fill: COLOR_BG_SURFACE,
                        bg_stroke: Stroke::new(1.0_f32, COLOR_BORDER),
                        rounding: Rounding::same(6.0),
                        fg_stroke: Stroke::new(1.0_f32, COLOR_TEXT_SECONDARY),
                        expansion: 0.0,
                    },
                    hovered: egui::style::WidgetVisuals {
                        bg_fill: Color32::from_rgb(0x2c, 0x2c, 0x30),
                        weak_bg_fill: Color32::from_rgb(0x2c, 0x2c, 0x30),
                        bg_stroke: Stroke::new(1.0_f32, accent),
                        rounding: Rounding::same(6.0),
                        fg_stroke: Stroke::new(1.0_f32, COLOR_TEXT_PRIMARY),
                        expansion: 0.0,
                    },
                    active: egui::style::WidgetVisuals {
                        bg_fill: accent,
                        weak_bg_fill: accent,
                        bg_stroke: Stroke::new(1.0_f32, accent),
                        rounding: Rounding::same(6.0),
                        fg_stroke: Stroke::new(1.0_f32, COLOR_BG_DEEP),
                        expansion: 0.0,
                    },
                    open: egui::style::WidgetVisuals {
                        bg_fill: COLOR_BG_SURFACE,
                        weak_bg_fill: COLOR_BG_SURFACE,
                        bg_stroke: Stroke::new(1.0_f32, COLOR_BORDER),
                        rounding: Rounding::same(6.0),
                        fg_stroke: Stroke::new(1.0_f32, COLOR_TEXT_PRIMARY),
                        expansion: 0.0,
                    },
                },
                selection: egui::style::Selection {
                    bg_fill: accent,
                    stroke: Stroke::new(1.0_f32, COLOR_BG_DEEP),
                },
                ..Default::default()
            });
        }

        self.handle_input(ctx);
        self.sync_live_state(ctx);
        self.handle_frameless_resize(ctx);

        egui::TopBottomPanel::top("top_bar")
            .frame(
                egui::Frame::none()
                    .fill(COLOR_BG_DEEP)
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .inner_margin(egui::Margin::symmetric(16.0, 10.0)),
            )
            .show(ctx, |ui| {
                self.ui_top_bar(ui, accent);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(COLOR_BG_CANVAS).inner_margin(16.0))
            .show(ctx, |ui| {
                match self.tab {
                    AppTab::Keys => self.ui_keys_tab(ui, accent),
                    AppTab::Radar => self.ui_radar_tab(ui, accent),
                    AppTab::Hits => self.ui_hits_tab(ui, accent),
                    AppTab::Setup => self.ui_setup_tab(ui, accent),
                }

                if let Some((msg, created)) = &self.toast {
                    if created.elapsed() < Duration::from_secs(4) {
                        ui.add_space(8.0);
                        egui::Frame::none()
                            .fill(COLOR_BG_CARD)
                            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                            .rounding(Rounding::same(6.0))
                            .inner_margin(egui::Margin::symmetric(12.0, 6.0))
                            .show(ui, |ui| {
                                ui.colored_label(COLOR_TEXT_PRIMARY, msg);
                            });
                    }
                }
            });

        #[cfg(target_os = "windows")]
        self.render_floating_overlays(ctx, accent);

        self.render_theme_popup(ctx, accent);
    }
}

pub fn run_app() -> Result<(), eframe::Error> {
    let initial_cfg = load_config_from_disk();
    #[cfg(target_os = "linux")]
    crate::linux_overlay::start_linux_overlay_thread();

    crate::keys::start_key_listener();
    let start_browser_ws = initial_cfg.enable_browser_overlays;
    std::thread::spawn(move || {
        if let Ok(rt) = tokio::runtime::Runtime::new() {
            crate::keys::register_tokio_handle(rt.handle().clone());
            rt.block_on(async move {
                tokio::spawn(crate::proxy::start_tosu_proxy());
                if start_browser_ws {
                    tokio::spawn(crate::keys::start_key_server());
                }
                std::future::pending::<()>().await;
            });
        }
    });

    #[cfg(target_os = "linux")]
    crate::bootstrap::ensure_linux_desktop_entry();

    let init_w = initial_cfg.window_width.clamp(760.0, 3840.0);
    let init_h = initial_cfg.window_height.clamp(500.0, 2160.0);

    let mut viewport = egui::ViewportBuilder::default()
        .with_app_id("oris-mania-utils")
        .with_inner_size([init_w, init_h])
        .with_min_inner_size([760.0, 500.0])
        .with_resizable(true)
        .with_title("ori's mania utils")
        .with_decorations(false)
        .with_transparent(true);

    let logo_bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/logo.png"));
    if let Ok(img) = image::load_from_memory(logo_bytes) {
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        viewport = viewport.with_icon(Arc::new(egui::IconData {
            rgba: rgba.into_raw(),
            width: w,
            height: h,
        }));
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "ori's mania utils",
        options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}
