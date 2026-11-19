use std::fs;
#[cfg(target_os = "windows")]
use std::sync::atomic::Ordering;

use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Vec2};

use crate::config::Config;
use crate::keys::end_key_capture;
use crate::theme::{
    apply_exported_theme, apply_preset, export_theme_from_config, find_theme, ConfigBackup,
    ExportedTheme, ThemeScope, BUILTIN_THEMES,
};

#[cfg(target_os = "windows")]
use super::hits_tab::hits_frame_metrics;
#[cfg(target_os = "windows")]
use super::keys_tab::keys_frame_geometry;
use super::{App, AppTab};

pub const COLOR_BG_DEEP: Color32 = Color32::from_rgb(0x0f, 0x0f, 0x10);
pub const COLOR_BG_CANVAS: Color32 = Color32::from_rgb(0x14, 0x14, 0x15);
pub const COLOR_BG_CARD: Color32 = Color32::from_rgb(0x19, 0x19, 0x1b);
pub const COLOR_BG_SURFACE: Color32 = Color32::from_rgb(0x23, 0x23, 0x25);
pub const COLOR_BORDER: Color32 = Color32::from_rgb(0x28, 0x28, 0x2b);
pub const COLOR_TEXT_PRIMARY: Color32 = Color32::from_rgb(0xed, 0xed, 0xed);
pub const COLOR_TEXT_SECONDARY: Color32 = Color32::from_rgb(0xa1, 0xa1, 0xa6);
pub const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(0x85, 0x85, 0x8b);
pub const COLOR_ACCENT: Color32 = Color32::from_rgb(0xe5, 0x88, 0xa6);
pub const COLOR_SUCCESS: Color32 = Color32::from_rgb(0x4c, 0xc3, 0x8a);
pub const COLOR_WARN: Color32 = Color32::from_rgb(0xe5, 0xb5, 0x67);
pub const COLOR_DANGER: Color32 = Color32::from_rgb(0xe5, 0x48, 0x4d);

pub fn parse_hex_color(hex: &str, fallback: Color32) -> Color32 {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&clean[0..2], 16),
            u8::from_str_radix(&clean[2..4], 16),
            u8::from_str_radix(&clean[4..6], 16),
        ) {
            return Color32::from_rgb(r, g, b);
        }
    }
    fallback
}

pub fn shape_rounding(shape: &str, base: f32, w: f32, h: f32) -> f32 {
    match shape {
        "square" => 3.0,
        "pill" | "circle" => (w.min(h) * 0.5).max(base),
        _ => base,
    }
}

pub fn reset_btn_if_changed<T: PartialEq + Clone>(ui: &mut egui::Ui, val: &mut T, def: T) -> bool {
    let dirty = *val != def;
    if ui
        .add_enabled(
            dirty,
            egui::Button::new(egui::RichText::new("r").size(10.5)),
        )
        .clicked()
    {
        *val = def;
        return true;
    }
    false
}

pub fn num_u32(
    ui: &mut egui::Ui,
    label: &str,
    val: &mut u32,
    def: u32,
    safe: std::ops::RangeInclusive<u32>,
    suffix: &str,
    unsafe_mode: bool,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        if unsafe_mode {
            changed |= ui
                .add(
                    egui::DragValue::new(val)
                        .speed(1.0)
                        .range(1..=u32::MAX)
                        .suffix(suffix),
                )
                .changed();
        } else {
            changed |= ui
                .add(egui::Slider::new(val, safe).suffix(suffix))
                .changed();
        }
        changed |= reset_btn_if_changed(ui, val, def);
    });
    changed
}

pub fn num_i32(
    ui: &mut egui::Ui,
    label: &str,
    val: &mut i32,
    def: i32,
    safe: std::ops::RangeInclusive<i32>,
    suffix: &str,
    unsafe_mode: bool,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        if unsafe_mode {
            changed |= ui
                .add(
                    egui::DragValue::new(val)
                        .speed(1.0)
                        .range(i32::MIN..=i32::MAX)
                        .suffix(suffix),
                )
                .changed();
        } else {
            changed |= ui.add(egui::Slider::new(val, safe)).changed();
        }
        changed |= reset_btn_if_changed(ui, val, def);
    });
    changed
}

pub fn num_f32(
    ui: &mut egui::Ui,
    label: &str,
    val: &mut f32,
    def: f32,
    safe: std::ops::RangeInclusive<f32>,
    unsafe_bounds: (f32, f32),
    unsafe_mode: bool,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        if unsafe_mode {
            changed |= ui
                .add(
                    egui::DragValue::new(val)
                        .speed(0.05)
                        .range(unsafe_bounds.0..=unsafe_bounds.1),
                )
                .changed();
        } else {
            changed |= ui.add(egui::Slider::new(val, safe)).changed();
        }
        let dirty = (*val - def).abs() > 0.001;
        if ui
            .add_enabled(
                dirty,
                egui::Button::new(egui::RichText::new("r").size(10.5)),
            )
            .clicked()
        {
            *val = def;
            changed = true;
        }
    });
    changed
}

pub fn draw_tosu_offline_pill(ui: &mut egui::Ui, scale: f32) {
    egui::Frame::none()
        .fill(Color32::from_rgb(0x26, 0x16, 0x1a))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x7f, 0x29, 0x35)))
        .rounding(Rounding::same(4.0 * scale))
        .inner_margin(egui::Margin::symmetric(6.0 * scale, 2.0 * scale))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (drect, _) =
                    ui.allocate_exact_size(Vec2::splat(6.0 * scale), egui::Sense::hover());
                ui.painter().circle_filled(
                    drect.center(),
                    2.5 * scale,
                    Color32::from_rgb(0xef, 0x44, 0x44),
                );
                ui.label(
                    egui::RichText::new("tosu! offline")
                        .monospace()
                        .size(9.5 * scale)
                        .color(Color32::from_rgb(0xfc, 0xa5, 0xa5)),
                );
            });
        });
}

impl App {
    pub(super) fn export_backup_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("oris-mania-utils-backup.json")
            .add_filter("json", &["json"])
            .save_file()
        {
            let backup = ConfigBackup::from_config(&self.config);
            match serde_json::to_string_pretty(&backup) {
                Ok(raw) => match fs::write(&path, raw) {
                    Ok(_) => self.set_toast("backup exported"),
                    Err(e) => self.set_toast(&format!("export error: {}", e)),
                },
                Err(e) => self.set_toast(&format!("serialize error: {}", e)),
            }
        }
    }

    pub(super) fn import_backup_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("json", &["json"])
            .pick_file()
        {
            match fs::read_to_string(&path) {
                Ok(raw) => {
                    let parsed = serde_json::from_str::<ConfigBackup>(&raw)
                        .map(|b| b.config)
                        .or_else(|_| serde_json::from_str::<Config>(&raw));
                    match parsed {
                        Ok(mut cfg) => {
                            cfg.normalize();
                            self.float_keys = cfg.float_keys_open;
                            self.float_radar = cfg.float_radar_open;
                            self.float_hits = cfg.float_hits_open;
                            self.config = cfg;
                            self.mark_dirty();
                            self.set_toast("backup imported (compatible normalized)");
                        }
                        Err(e) => self.set_toast(&format!("invalid backup file: {}", e)),
                    }
                }
                Err(e) => self.set_toast(&format!("read error: {}", e)),
            }
        }
    }

    pub(super) fn export_theme_dialog(&mut self, scope: ThemeScope) {
        let name = self.current_theme_name(scope);
        let slug = name.replace(' ', "-");
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(format!("oris-theme-{}.json", slug))
            .add_filter("json", &["json"])
            .save_file()
        {
            let exported = export_theme_from_config(&self.config, scope);
            match serde_json::to_string_pretty(&exported) {
                Ok(raw) => match fs::write(&path, raw) {
                    Ok(_) => self.set_toast("theme exported"),
                    Err(e) => self.set_toast(&format!("export error: {}", e)),
                },
                Err(e) => self.set_toast(&format!("serialize error: {}", e)),
            }
        }
    }

    pub(super) fn import_theme_dialog(&mut self, scope: ThemeScope) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("json", &["json"])
            .pick_file()
        {
            match fs::read_to_string(&path) {
                Ok(raw) => match serde_json::from_str::<ExportedTheme>(&raw) {
                    Ok(theme) => {
                        let apply_scope = if self.config.link_overlay_themes {
                            ThemeScope::All
                        } else {
                            scope
                        };
                        apply_exported_theme(&mut self.config, &theme, apply_scope);
                        self.mark_dirty();
                        self.set_toast(&format!("imported theme: {}", theme.name));
                    }
                    Err(e) => self.set_toast(&format!("invalid theme json: {}", e)),
                },
                Err(e) => self.set_toast(&format!("read error: {}", e)),
            }
        }
    }

    fn current_theme_name(&self, scope: ThemeScope) -> String {
        let id = if self.config.link_overlay_themes {
            self.config.active_theme_id.as_str()
        } else {
            match scope {
                ThemeScope::Keys => self.config.keys_theme_id.as_str(),
                ThemeScope::Radar => self.config.radar_theme_id.as_str(),
                ThemeScope::Hits => self.config.hits_theme_id.as_str(),
                ThemeScope::All => self.config.active_theme_id.as_str(),
            }
        };
        find_theme(id)
            .map(|t| t.name.to_string())
            .unwrap_or_else(|| "custom".to_string())
    }

    pub(super) fn draw_theme_dropdown(
        &mut self,
        ui: &mut egui::Ui,
        scope: ThemeScope,
        accent: Color32,
    ) {
        let cur_id = if self.config.link_overlay_themes {
            self.config.active_theme_id.clone()
        } else {
            match scope {
                ThemeScope::Keys => self.config.keys_theme_id.clone(),
                ThemeScope::Radar => self.config.radar_theme_id.clone(),
                ThemeScope::Hits => self.config.hits_theme_id.clone(),
                ThemeScope::All => self.config.active_theme_id.clone(),
            }
        };
        let active_preset = find_theme(&cur_id).unwrap_or(&BUILTIN_THEMES[0]);
        let p_acc = parse_hex_color(active_preset.accent, accent);
        let p_sec = parse_hex_color(active_preset.accent_secondary, p_acc);
        let p_bg = parse_hex_color(active_preset.bg, COLOR_BG_CANVAS);
        let p_bdr = parse_hex_color(active_preset.border, COLOR_BORDER);
        let is_open = self
            .theme_popup
            .map(|(s, _, _)| s == scope)
            .unwrap_or(false);

        ui.horizontal(|ui| {
            let btn_w = (ui.available_width() - 78.0).clamp(150.0, 250.0);
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(btn_w, 28.0), egui::Sense::click());
            let p = ui.painter_at(rect);
            let fill = if resp.hovered() || is_open {
                Color32::from_rgb(0x28, 0x28, 0x2c)
            } else {
                COLOR_BG_SURFACE
            };
            let stroke = if is_open {
                accent
            } else if resp.hovered() {
                p_acc
            } else {
                COLOR_BORDER
            };
            p.rect_filled(rect, Rounding::same(6.0), fill);
            p.rect_stroke(rect, Rounding::same(6.0), Stroke::new(1.0_f32, stroke));

            let sw_rect = Rect::from_min_size(
                Pos2::new(rect.min.x + 8.0, rect.min.y + 6.0),
                Vec2::new(34.0, 16.0),
            );
            p.rect_filled(sw_rect, Rounding::same(4.0), p_bg);
            p.rect_stroke(sw_rect, Rounding::same(4.0), Stroke::new(1.0_f32, p_bdr));
            p.circle_filled(
                Pos2::new(sw_rect.min.x + 11.0, sw_rect.center().y),
                4.2,
                p_acc,
            );
            p.circle_filled(
                Pos2::new(sw_rect.min.x + 23.0, sw_rect.center().y),
                4.2,
                p_sec,
            );

            p.text(
                Pos2::new(sw_rect.max.x + 8.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                active_preset.name,
                egui::FontId::proportional(12.0),
                COLOR_TEXT_PRIMARY,
            );
            p.text(
                Pos2::new(rect.max.x - 10.0, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                if is_open { "^" } else { "v" },
                egui::FontId::monospace(11.0),
                COLOR_TEXT_MUTED,
            );

            if resp.clicked() {
                if is_open {
                    self.theme_popup = None;
                } else {
                    self.theme_popup = Some((
                        scope,
                        Pos2::new(rect.min.x, rect.max.y + 4.0),
                        btn_w.max(220.0),
                    ));
                }
            }

            if ui.button("apply all").clicked() {
                if let Some(t) = find_theme(&cur_id) {
                    apply_preset(&mut self.config, ThemeScope::All, t);
                    self.mark_dirty();
                    self.set_toast(&format!("applied {} to all overlays", t.name));
                }
            }
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut self.config.link_overlay_themes, "link all overlays")
                .changed()
            {
                if self.config.link_overlay_themes {
                    if let Some(t) = find_theme(&cur_id) {
                        apply_preset(&mut self.config, ThemeScope::All, t);
                    }
                }
                self.mark_dirty();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("import").clicked() {
                    self.import_theme_dialog(scope);
                }
                if ui.small_button("export").clicked() {
                    self.export_theme_dialog(scope);
                }
            });
        });
    }

    pub(super) fn render_theme_popup(&mut self, ctx: &egui::Context, accent: Color32) {
        let Some((scope, pos, width)) = self.theme_popup else {
            return;
        };
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.theme_popup = None;
            return;
        }
        let cur_id = if self.config.link_overlay_themes {
            self.config.active_theme_id.as_str()
        } else {
            match scope {
                ThemeScope::Keys => self.config.keys_theme_id.as_str(),
                ThemeScope::Radar => self.config.radar_theme_id.as_str(),
                ThemeScope::Hits => self.config.hits_theme_id.as_str(),
                ThemeScope::All => self.config.active_theme_id.as_str(),
            }
        }
        .to_string();

        let area_resp = egui::Area::new(egui::Id::new("unified_theme_dropdown_popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(COLOR_BG_CARD)
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(6.0)
                    .show(ui, |ui| {
                        ui.set_width(width);
                        egui::ScrollArea::vertical()
                            .id_salt("theme_popup_list")
                            .max_height(260.0)
                            .show(ui, |ui| {
                                for preset in BUILTIN_THEMES {
                                    let selected = preset.id == cur_id;
                                    let p_acc = parse_hex_color(preset.accent, accent);
                                    let p_sec = parse_hex_color(preset.accent_secondary, p_acc);
                                    let p_bg = parse_hex_color(preset.bg, COLOR_BG_CANVAS);
                                    let p_bdr = parse_hex_color(preset.border, COLOR_BORDER);

                                    let (rect, row_resp) = ui.allocate_exact_size(
                                        Vec2::new(width - 12.0, 28.0),
                                        egui::Sense::click(),
                                    );
                                    let p = ui.painter_at(rect);
                                    if selected || row_resp.hovered() {
                                        let row_fill = if selected {
                                            Color32::from_rgb(0x28, 0x28, 0x2e)
                                        } else {
                                            COLOR_BG_SURFACE
                                        };
                                        p.rect_filled(rect, Rounding::same(5.0), row_fill);
                                    }

                                    let sw_rect = Rect::from_min_size(
                                        Pos2::new(rect.min.x + 6.0, rect.min.y + 6.0),
                                        Vec2::new(34.0, 16.0),
                                    );
                                    p.rect_filled(sw_rect, Rounding::same(4.0), p_bg);
                                    p.rect_stroke(
                                        sw_rect,
                                        Rounding::same(4.0),
                                        Stroke::new(1.0_f32, p_bdr),
                                    );
                                    p.circle_filled(
                                        Pos2::new(sw_rect.min.x + 11.0, sw_rect.center().y),
                                        4.0,
                                        p_acc,
                                    );
                                    p.circle_filled(
                                        Pos2::new(sw_rect.min.x + 23.0, sw_rect.center().y),
                                        4.0,
                                        p_sec,
                                    );

                                    p.text(
                                        Pos2::new(sw_rect.max.x + 8.0, rect.center().y),
                                        egui::Align2::LEFT_CENTER,
                                        preset.name,
                                        egui::FontId::proportional(12.0),
                                        if selected { p_acc } else { COLOR_TEXT_PRIMARY },
                                    );
                                    if selected {
                                        p.circle_filled(
                                            Pos2::new(rect.max.x - 10.0, rect.center().y),
                                            3.0,
                                            p_acc,
                                        );
                                    }

                                    if row_resp.clicked() {
                                        let target = if self.config.link_overlay_themes {
                                            ThemeScope::All
                                        } else {
                                            scope
                                        };
                                        apply_preset(&mut self.config, target, preset);
                                        self.mark_dirty();
                                        self.theme_popup = None;
                                    }
                                }
                            });
                    });
            });

        if ctx.input(|i| i.pointer.primary_clicked()) {
            if let Some(ptr) = ctx.input(|i| i.pointer.interact_pos()) {
                if !area_resp.response.rect.contains(ptr)
                    && !Rect::from_min_size(Pos2::new(pos.x, pos.y - 34.0), Vec2::new(width, 34.0))
                        .contains(ptr)
                {
                    self.theme_popup = None;
                }
            }
        }
    }

    pub(super) fn ui_top_bar(&mut self, ui: &mut egui::Ui, accent: Color32) {
        if self.logo_tex.is_none() {
            let logo_bytes =
                include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/logo.png"));
            if let Ok(img) = image::load_from_memory(logo_bytes) {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let cimg = egui::ColorImage::from_rgba_unmultiplied(
                    [w as usize, h as usize],
                    &rgba.into_raw(),
                );
                self.logo_tex = Some(ui.ctx().load_texture(
                    "app_logo_icon",
                    cimg,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }
        ui.horizontal(|ui| {
            if let Some(tex) = &self.logo_tex {
                ui.add(egui::Image::new(tex).fit_to_exact_size(Vec2::new(20.0, 20.0)));
            }
            let title_resp = ui.add(
                egui::Label::new(
                    egui::RichText::new("ori's mania utils")
                        .size(15.0)
                        .color(COLOR_TEXT_PRIMARY)
                        .strong(),
                )
                .sense(egui::Sense::drag()),
            );
            if title_resp.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if title_resp.double_clicked() {
                let is_max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
            }

            ui.add_space(20.0);

            let tabs = [
                (AppTab::Keys, "keys"),
                (AppTab::Radar, "radar"),
                (AppTab::Hits, "hits"),
                (AppTab::Setup, "setup"),
            ];

            for (tab, label) in tabs {
                let is_active = self.tab == tab;
                let (fill, stroke, text_color) = if is_active {
                    (accent, Stroke::new(1.0_f32, accent), COLOR_BG_DEEP)
                } else {
                    (
                        COLOR_BG_SURFACE,
                        Stroke::new(1.0_f32, COLOR_BORDER),
                        COLOR_TEXT_SECONDARY,
                    )
                };

                let tab_btn =
                    egui::Button::new(egui::RichText::new(label).size(13.0).color(text_color))
                        .fill(fill)
                        .stroke(stroke)
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(60.0, 28.0));

                if ui.add(tab_btn).clicked() {
                    if self.key_rebind_idx.is_some() {
                        end_key_capture();
                        self.key_rebind_idx = None;
                    }
                    self.tab = tab;
                }
            }

            let avail = (ui.available_width() - 280.0).max(0.0);
            if avail > 10.0 {
                let (_, drag_spacer) =
                    ui.allocate_exact_size(Vec2::new(avail, 28.0), egui::Sense::drag());
                if drag_spacer.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if drag_spacer.double_clicked() {
                    let is_max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (close_rect, close_resp) =
                    ui.allocate_exact_size(Vec2::new(26.0, 26.0), egui::Sense::click());
                let close_bg = if close_resp.hovered() {
                    Color32::from_rgb(0x4a, 0x1d, 0x26)
                } else {
                    COLOR_BG_SURFACE
                };
                let close_fg = if close_resp.hovered() {
                    Color32::from_rgb(0xfc, 0xa5, 0xa5)
                } else {
                    COLOR_TEXT_PRIMARY
                };
                ui.painter().rect(
                    close_rect,
                    Rounding::same(6.0),
                    close_bg,
                    Stroke::new(1.0_f32, COLOR_BORDER),
                );
                let cc = close_rect.center();
                let cr = 4.0;
                ui.painter().line_segment(
                    [
                        Pos2::new(cc.x - cr, cc.y - cr),
                        Pos2::new(cc.x + cr, cc.y + cr),
                    ],
                    Stroke::new(1.4_f32, close_fg),
                );
                ui.painter().line_segment(
                    [
                        Pos2::new(cc.x + cr, cc.y - cr),
                        Pos2::new(cc.x - cr, cc.y + cr),
                    ],
                    Stroke::new(1.4_f32, close_fg),
                );
                if close_resp.clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }

                let is_max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                let (max_rect, max_resp) =
                    ui.allocate_exact_size(Vec2::new(26.0, 26.0), egui::Sense::click());
                let max_bg = if max_resp.hovered() {
                    Color32::from_rgb(0x28, 0x28, 0x2c)
                } else {
                    COLOR_BG_SURFACE
                };
                let max_fg = if max_resp.hovered() {
                    COLOR_TEXT_PRIMARY
                } else {
                    COLOR_TEXT_SECONDARY
                };
                ui.painter().rect(
                    max_rect,
                    Rounding::same(6.0),
                    max_bg,
                    Stroke::new(1.0_f32, COLOR_BORDER),
                );
                let mc = max_rect.center();
                if is_max {
                    let back =
                        Rect::from_center_size(Pos2::new(mc.x + 1.4, mc.y - 1.4), Vec2::splat(7.0));
                    let front =
                        Rect::from_center_size(Pos2::new(mc.x - 1.4, mc.y + 1.4), Vec2::splat(7.0));
                    ui.painter().rect_stroke(
                        back,
                        Rounding::same(1.0),
                        Stroke::new(1.2_f32, max_fg),
                    );
                    ui.painter().rect(
                        front,
                        Rounding::same(1.0),
                        max_bg,
                        Stroke::new(1.2_f32, max_fg),
                    );
                } else {
                    let sq = Rect::from_center_size(mc, Vec2::splat(8.5));
                    ui.painter()
                        .rect_stroke(sq, Rounding::same(1.2), Stroke::new(1.3_f32, max_fg));
                }
                if max_resp.clicked() {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                }

                let (min_rect, min_resp) =
                    ui.allocate_exact_size(Vec2::new(26.0, 26.0), egui::Sense::click());
                let min_bg = if min_resp.hovered() {
                    Color32::from_rgb(0x28, 0x28, 0x2c)
                } else {
                    COLOR_BG_SURFACE
                };
                let min_fg = if min_resp.hovered() {
                    COLOR_TEXT_PRIMARY
                } else {
                    COLOR_TEXT_SECONDARY
                };
                ui.painter().rect(
                    min_rect,
                    Rounding::same(6.0),
                    min_bg,
                    Stroke::new(1.0_f32, COLOR_BORDER),
                );
                let nc = min_rect.center();
                ui.painter().line_segment(
                    [
                        Pos2::new(nc.x - 4.5, nc.y + 2.0),
                        Pos2::new(nc.x + 4.5, nc.y + 2.0),
                    ],
                    Stroke::new(1.4_f32, min_fg),
                );
                if min_resp.clicked() {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }

                ui.add_space(6.0);

                let sync_text_col = if self.config.enable_browser_overlays {
                    COLOR_TEXT_PRIMARY
                } else {
                    COLOR_TEXT_MUTED
                };
                let sync_btn = ui
                    .add_enabled(
                        self.config.enable_browser_overlays,
                        egui::Button::new(
                            egui::RichText::new("sync overlays")
                                .size(11.5)
                                .color(sync_text_col),
                        )
                        .fill(COLOR_BG_SURFACE)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(88.0, 26.0)),
                    )
                    .on_disabled_hover_text(
                        "disabled when browser overlays are off (enable in setup for obs)",
                    );
                if sync_btn.clicked() {
                    self.sync_overlays();
                }

                ui.add_space(6.0);

                let (status_color, status_text) = if self.tosu_running {
                    (COLOR_SUCCESS, "tosu live")
                } else if self.tosu_installed {
                    (COLOR_WARN, "tosu idle")
                } else {
                    (COLOR_TEXT_MUTED, "tosu off")
                };

                egui::Frame::none()
                    .fill(COLOR_BG_CARD)
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::splat(6.0), egui::Sense::hover());
                            ui.painter().circle_filled(rect.center(), 3.0, status_color);
                            ui.label(
                                egui::RichText::new(status_text)
                                    .size(11.0)
                                    .color(COLOR_TEXT_SECONDARY),
                            );
                        });
                    });
            });
        });
    }

    #[cfg(target_os = "windows")]
    pub(super) fn render_floating_overlays(&mut self, ctx: &egui::Context, accent: Color32) {
        let locked = self.config.lock_overlays;
        let tosu_ok = crate::proxy::TOSU_CONNECTED.load(Ordering::Relaxed);

        if self.float_keys && self.config.show_keys_overlay {
            let geom = keys_frame_geometry(&self.config, false);
            let w = (geom.frame_w + 16.0).max(if tosu_ok { 40.0 } else { 126.0 });
            let h = geom.frame_h + 64.0 * geom.scale + if tosu_ok { 0.0 } else { 24.0 };

            let mut builder = egui::ViewportBuilder::default()
                .with_app_id("oris-mania-utils")
                .with_title("keys")
                .with_inner_size([w, h])
                .with_decorations(false)
                .with_transparent(true)
                .with_always_on_top()
                .with_mouse_passthrough(locked);

            if self.config.keys_pos_x > 0 || self.config.keys_pos_y > 0 {
                builder = builder
                    .with_position([self.config.keys_pos_x as f32, self.config.keys_pos_y as f32]);
            }

            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("oris_float_keys"),
                builder,
                |vctx, _| {
                    let resp = egui::CentralPanel::default()
                        .frame(
                            egui::Frame::none()
                                .fill(Color32::TRANSPARENT)
                                .inner_margin(4.0),
                        )
                        .show(vctx, |ui| {
                            ui.add_space(58.0 * geom.scale);
                            self.draw_keys_overlay_widget(ui, false);
                        })
                        .response;

                    if !locked && resp.interact(egui::Sense::drag()).drag_started() {
                        vctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }
                },
            );
        }

        if self.float_radar && self.config.show_radar_overlay {
            let r_sc = self.config.scale.clamp(0.55, 1.35);
            let rw = (self.config.radar_width as f32).clamp(150.0, 1600.0) * r_sc + 12.0;
            let graph_extra = if self.config.radar_show_strain_graph {
                (self.config.radar_graph_height as f32).clamp(16.0, 90.0) * r_sc + 12.0
            } else {
                0.0
            };
            let rh = if !self.config.radar_auto_height {
                (self.config.radar_height as f32).clamp(140.0, 1600.0) * r_sc + 12.0
            } else {
                312.0 * r_sc + graph_extra
            };
            let mut builder = egui::ViewportBuilder::default()
                .with_app_id("oris-mania-utils")
                .with_title("radar")
                .with_inner_size([rw, rh])
                .with_decorations(false)
                .with_transparent(true)
                .with_always_on_top()
                .with_mouse_passthrough(locked);

            if self.config.radar_pos_x > 0 || self.config.radar_pos_y > 0 {
                builder = builder.with_position([
                    self.config.radar_pos_x as f32,
                    self.config.radar_pos_y as f32,
                ]);
            }

            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("oris_float_radar"),
                builder,
                |vctx, _| {
                    let resp = egui::CentralPanel::default()
                        .frame(
                            egui::Frame::none()
                                .fill(Color32::TRANSPARENT)
                                .inner_margin(4.0),
                        )
                        .show(vctx, |ui| {
                            self.draw_radar_overlay_widget(ui, accent);
                        })
                        .response;

                    if !locked && resp.interact(egui::Sense::drag()).drag_started() {
                        vctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }
                },
            );
        }

        if self.float_hits && self.config.show_hits_overlay {
            let hm = hits_frame_metrics(&self.config, tosu_ok);
            let ratio_h = if !hm.is_dock && !hm.is_horizontal && self.config.hitcounter_show_ratio {
                22.0 * hm.scale
            } else {
                0.0
            };
            let off_h = if tosu_ok { 0.0 } else { 18.0 * hm.scale };
            let rows_h = if hm.is_horizontal {
                hm.item_h
            } else if hm.is_grid {
                3.0 * hm.item_h + 8.0 * hm.scale + ratio_h
            } else {
                6.0 * hm.item_h + 5.0 * hm.gap + ratio_h
            };
            let w = (hm.draw_w + 2.0 * hm.pad + 12.0).max(if tosu_ok {
                40.0
            } else {
                118.0 * hm.scale
            });
            let h = rows_h + 2.0 * hm.pad + 12.0 + off_h;

            let mut builder = egui::ViewportBuilder::default()
                .with_app_id("oris-mania-utils")
                .with_title("hits")
                .with_inner_size([w, h])
                .with_decorations(false)
                .with_transparent(true)
                .with_always_on_top()
                .with_mouse_passthrough(locked);

            if self.config.hitcounter_position_x > 0 || self.config.hitcounter_position_y > 0 {
                builder = builder.with_position([
                    self.config.hitcounter_position_x as f32,
                    self.config.hitcounter_position_y as f32,
                ]);
            }

            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("oris_float_hits"),
                builder,
                |vctx, _| {
                    let resp = egui::CentralPanel::default()
                        .frame(
                            egui::Frame::none()
                                .fill(Color32::TRANSPARENT)
                                .inner_margin(4.0),
                        )
                        .show(vctx, |ui| {
                            self.draw_hits_overlay_widget(ui);
                        })
                        .response;

                    if !locked && resp.interact(egui::Sense::drag()).drag_started() {
                        vctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }
                },
            );
        }
    }
}
