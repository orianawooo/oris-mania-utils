use std::sync::atomic::Ordering;

use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Vec2};

use crate::config::Config;
use crate::keys::{begin_key_capture, end_key_capture};
use crate::theme::ThemeScope;

use super::input::key_name_to_display;
use super::widgets::{
    draw_tosu_offline_pill, num_f32, num_i32, num_u32, parse_hex_color, shape_rounding,
    COLOR_ACCENT, COLOR_BG_CANVAS, COLOR_BG_CARD, COLOR_BG_DEEP, COLOR_BG_SURFACE, COLOR_BORDER,
    COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY,
};
use super::App;

pub struct KeysFrameGeometry {
    pub is_dock: bool,
    pub is_dock_tint: bool,
    pub is_flat: bool,
    pub scale: f32,
    pub key_w: f32,
    pub key_h: f32,
    pub gap: f32,
    pub pad: f32,
    pub frame_w: f32,
    pub frame_h: f32,
}

pub fn keys_frame_geometry(cfg: &Config, interactive: bool) -> KeysFrameGeometry {
    let style = cfg.keys_style.as_str();
    let is_dock_tint = style == "dock-tint";
    let is_dock = style == "dock" || is_dock_tint;
    let is_flat = style == "flat";
    let scale = if interactive {
        cfg.keys_bg_scale.clamp(0.5, 1.45)
    } else {
        cfg.keys_bg_scale.clamp(0.5, 2.5)
    };
    let key_w = (cfg.key_size as f32).clamp(24.0, 120.0) * scale;
    let key_h = (cfg.key_height as f32).clamp(24.0, 120.0) * scale;
    let gap = if is_dock {
        0.0
    } else {
        (cfg.key_gap as f32).min(36.0) * scale
    };
    let pad = if is_dock {
        0.0
    } else if cfg.keys_bg_enabled {
        (cfg.keys_bg_padding as f32).min(32.0) * scale
    } else {
        1.0
    };
    let frame_w = 4.0 * key_w + 3.0 * gap + 2.0 * pad;
    let frame_h = key_h + 2.0 * pad;
    KeysFrameGeometry {
        is_dock,
        is_dock_tint,
        is_flat,
        scale,
        key_w,
        key_h,
        gap,
        pad,
        frame_w,
        frame_h,
    }
}

impl App {
    pub(super) fn draw_keys_overlay_widget(&mut self, ui: &mut egui::Ui, interactive: bool) {
        let g = keys_frame_geometry(&self.config, interactive);
        let bg_rad = if g.is_dock {
            8.0 * g.scale
        } else {
            shape_rounding(
                &self.config.keys_bg_shape,
                self.config.keys_bg_radius as f32 * g.scale,
                g.frame_w,
                g.frame_h,
            )
        };
        let key_rad_base = self.config.key_radius as f32 * g.scale;

        let mut outer_color = parse_hex_color(&self.config.key_color_outer, COLOR_ACCENT);
        let mut inner_color = parse_hex_color(
            &self.config.key_color_inner,
            Color32::from_rgb(0xd4, 0x76, 0x95),
        );
        let mut idle_bg = parse_hex_color(
            &self.config.key_inactive_color,
            Color32::from_rgb(0x14, 0x14, 0x15),
        );
        let mut idle_border = parse_hex_color(&self.config.key_border_color, COLOR_BORDER);
        let mut frame_border = parse_hex_color(&self.config.keys_border_color, COLOR_BORDER);
        let mut frame_base = parse_hex_color(&self.config.keys_bg_color, COLOR_BG_CANVAS);
        if self.config.keys_auto_bg_theme && !self.radar_skills.adaptive_accent.is_empty() {
            outer_color = parse_hex_color(&self.radar_skills.adaptive_accent, outer_color);
            inner_color =
                parse_hex_color(&self.radar_skills.adaptive_accent_secondary, outer_color);
            frame_base = parse_hex_color(&self.radar_skills.adaptive_bg, frame_base);
            idle_bg = parse_hex_color(&self.radar_skills.adaptive_item_bg, idle_bg);
            idle_border = parse_hex_color(&self.radar_skills.adaptive_border, idle_border);
            frame_border = idle_border;
        }
        let frame_alpha = (self.config.keys_bg_opacity.clamp(0.0, 1.0) * 255.0) as u8;
        let frame_fill = Color32::from_rgba_unmultiplied(
            frame_base.r(),
            frame_base.g(),
            frame_base.b(),
            frame_alpha,
        );
        let frame_stroke_col = Color32::from_rgba_unmultiplied(
            frame_border.r(),
            frame_border.g(),
            frame_border.b(),
            frame_alpha.max(85),
        );
        let idle_fill = if g.is_dock {
            Color32::TRANSPARENT
        } else if self.config.keys_bg_enabled {
            idle_bg
        } else {
            let ka = (self.config.keys_bg_opacity.clamp(0.25, 1.0) * 255.0) as u8;
            Color32::from_rgba_unmultiplied(idle_bg.r(), idle_bg.g(), idle_bg.b(), ka)
        };

        let mut frame = egui::Frame::none().inner_margin(egui::Margin::same(g.pad));
        if self.config.keys_bg_enabled || g.is_dock {
            frame = frame
                .fill(frame_fill)
                .stroke(Stroke::new(1.0_f32, frame_stroke_col))
                .rounding(Rounding::same(bg_rad));
        }

        let tosu_ok = crate::proxy::TOSU_CONNECTED.load(Ordering::Relaxed);
        let phase_secs = ui.input(|i| i.time) as f32;
        ui.vertical(|ui| {
            frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = g.gap;
                    for i in 0..4 {
                        let key_label = self
                            .config
                            .key_labels
                            .get(i)
                            .filter(|s| !s.is_empty())
                            .cloned()
                            .unwrap_or_else(|| {
                                if i < self.config.keys.len() {
                                    key_name_to_display(&self.config.keys[i])
                                } else {
                                    format!("K{}", i + 1)
                                }
                            });

                        let is_listening = interactive && self.key_rebind_idx == Some(i);
                        let is_pressed =
                            self.key_pressed[i] || (interactive && self.test_hold_preview);
                        let fallback_col = if i == 0 || i == 3 {
                            outer_color
                        } else {
                            inner_color
                        };
                        let base_col = if !self.config.per_key_colors
                            || (self.config.keys_auto_bg_theme
                                && !self.radar_skills.adaptive_accent.is_empty())
                        {
                            fallback_col
                        } else {
                            self.config
                                .key_colors
                                .get(i)
                                .map(|s| parse_hex_color(s, fallback_col))
                                .unwrap_or(fallback_col)
                        };
                        let active_col = if *self.config.rgb_enabled_keys.get(i).unwrap_or(&false)
                            && is_pressed
                        {
                            ui.ctx().request_repaint();
                            let h = (phase_secs * 100.0 * self.config.rgb_speed.max(0.1)
                                + (i as f32) * 35.0)
                                % 360.0;
                            let c = (1.0_f32 - ((h / 60.0) % 2.0 - 1.0).abs()).max(0.0);
                            let (r, g_c, b) = if h < 60.0 {
                                (1.0, c, 0.0)
                            } else if h < 120.0 {
                                (c, 1.0, 0.0)
                            } else if h < 180.0 {
                                (0.0, 1.0, c)
                            } else if h < 240.0 {
                                (0.0, c, 1.0)
                            } else if h < 300.0 {
                                (c, 0.0, 1.0)
                            } else {
                                (1.0, 0.0, c)
                            };
                            Color32::from_rgb(
                                (r * 255.0) as u8,
                                (g_c * 255.0) as u8,
                                (b * 255.0) as u8,
                            )
                        } else {
                            base_col
                        };

                        let sense = if interactive {
                            egui::Sense::click()
                        } else {
                            egui::Sense::hover()
                        };
                        let (rect, resp) =
                            ui.allocate_exact_size(Vec2::new(g.key_w, g.key_h), sense);
                        if interactive && resp.clicked() {
                            if is_listening {
                                end_key_capture();
                                self.key_rebind_idx = None;
                            } else {
                                begin_key_capture();
                                self.key_rebind_idx = Some(i);
                            }
                        }

                        let painter = ui.painter();
                        let y_offset = if is_pressed && !g.is_dock {
                            1.0 * g.scale
                        } else {
                            0.0
                        };
                        let key_rect = Rect::from_min_size(
                            Pos2::new(rect.min.x, rect.min.y + y_offset),
                            Vec2::new(g.key_w, g.key_h),
                        );
                        let kshape = self
                            .config
                            .key_shapes
                            .get(i)
                            .map(|s| s.as_str())
                            .unwrap_or("rounded");
                        let krad = if g.is_dock {
                            if i == 0 || i == 3 {
                                7.0 * g.scale
                            } else {
                                0.0
                            }
                        } else {
                            shape_rounding(kshape, key_rad_base, g.key_w, g.key_h)
                        };

                        if is_pressed && self.config.show_trails {
                            let tw = self
                                .config
                                .trail_widths
                                .get(i)
                                .copied()
                                .map(|v| (v as f32 * g.scale).clamp(6.0, g.key_w + 24.0))
                                .unwrap_or_else(|| (g.key_w - 8.0 * g.scale).max(8.0));
                            let th = (54.0 * g.scale).clamp(24.0, 72.0);
                            let trail_bot = rect.min.y;
                            let trail_top = trail_bot - th;
                            let trail_left = key_rect.center().x - tw * 0.5;
                            let trail_rect = Rect::from_min_max(
                                Pos2::new(trail_left, trail_top),
                                Pos2::new(trail_left + tw, trail_bot),
                            );
                            let bot_a =
                                (self.config.trail_opacity.clamp(0.05, 1.0) * 255.0).round() as u8;
                            let top_a =
                                (self.config.trail_fade.clamp(0.0, 1.0) * 255.0).round() as u8;
                            let top_col = Color32::from_rgba_unmultiplied(
                                active_col.r(),
                                active_col.g(),
                                active_col.b(),
                                top_a,
                            );
                            let bot_col = Color32::from_rgba_unmultiplied(
                                active_col.r(),
                                active_col.g(),
                                active_col.b(),
                                bot_a,
                            );
                            let tshape = self
                                .config
                                .trail_shapes
                                .get(i)
                                .map(|s| s.as_str())
                                .unwrap_or("rounded");
                            let trad = shape_rounding(tshape, 3.0 * g.scale, tw, th)
                                .min(tw * 0.48)
                                .min(th * 0.45);
                            let body_rect = if trad > 0.5 {
                                Rect::from_min_max(
                                    Pos2::new(trail_rect.min.x, trail_rect.min.y + trad),
                                    Pos2::new(trail_rect.max.x, trail_rect.max.y),
                                )
                            } else {
                                trail_rect
                            };
                            let mut mesh = egui::Mesh::default();
                            mesh.colored_vertex(body_rect.left_top(), top_col);
                            mesh.colored_vertex(body_rect.right_top(), top_col);
                            mesh.colored_vertex(body_rect.right_bottom(), bot_col);
                            mesh.colored_vertex(body_rect.left_bottom(), bot_col);
                            mesh.add_triangle(0, 1, 2);
                            mesh.add_triangle(0, 2, 3);
                            painter.add(egui::Shape::mesh(mesh));
                            if trad > 0.5 {
                                painter.rect_filled(
                                    Rect::from_min_max(
                                        trail_rect.min,
                                        Pos2::new(trail_rect.max.x, trail_rect.min.y + trad * 2.0),
                                    ),
                                    Rounding {
                                        nw: trad,
                                        ne: trad,
                                        sw: 0.0,
                                        se: 0.0,
                                    },
                                    top_col,
                                );
                            }
                        }

                        let fill = if is_pressed {
                            if g.is_dock_tint {
                                Color32::from_rgba_unmultiplied(
                                    active_col.r(),
                                    active_col.g(),
                                    active_col.b(),
                                    78,
                                )
                            } else {
                                Color32::from_rgba_unmultiplied(
                                    active_col.r(),
                                    active_col.g(),
                                    active_col.b(),
                                    248,
                                )
                            }
                        } else if is_listening {
                            COLOR_BG_SURFACE
                        } else {
                            idle_fill
                        };
                        if fill != Color32::TRANSPARENT {
                            painter.rect_filled(key_rect, Rounding::same(krad), fill);
                        }
                        if is_pressed && g.is_dock_tint {
                            let rim_l = if i == 0 {
                                key_rect.min.x + 4.0 * g.scale
                            } else {
                                key_rect.min.x + 1.0 * g.scale
                            };
                            let rim_r = if i == 3 {
                                key_rect.max.x - 4.0 * g.scale
                            } else {
                                key_rect.max.x - 1.0 * g.scale
                            };
                            painter.rect_filled(
                                Rect::from_min_max(
                                    Pos2::new(rim_l, key_rect.min.y),
                                    Pos2::new(
                                        rim_r.max(rim_l + 4.0),
                                        key_rect.min.y + 2.0 * g.scale,
                                    ),
                                ),
                                Rounding::same(1.0 * g.scale),
                                Color32::from_rgba_unmultiplied(
                                    active_col.r(),
                                    active_col.g(),
                                    active_col.b(),
                                    195,
                                ),
                            );
                        }

                        if g.is_dock {
                            if i > 0 {
                                painter.line_segment(
                                    [
                                        Pos2::new(key_rect.min.x, key_rect.min.y + 2.0 * g.scale),
                                        Pos2::new(key_rect.min.x, key_rect.max.y - 2.0 * g.scale),
                                    ],
                                    Stroke::new(
                                        1.0_f32,
                                        Color32::from_rgba_unmultiplied(255, 255, 255, 16),
                                    ),
                                );
                            }
                        } else if !g.is_flat {
                            let stroke_col = if is_listening || is_pressed {
                                active_col
                            } else {
                                idle_border
                            };
                            painter.rect_stroke(
                                key_rect,
                                Rounding::same(krad),
                                Stroke::new(1.0_f32, stroke_col),
                            );
                        }

                        if self.config.keys_show_accent_bar && !is_pressed && !g.is_dock {
                            let bar_w = (g.key_w - 10.0 * g.scale).max(6.0);
                            let bar_rect = Rect::from_min_size(
                                Pos2::new(
                                    key_rect.center().x - bar_w * 0.5,
                                    key_rect.min.y + 2.5 * g.scale,
                                ),
                                Vec2::new(bar_w, 2.0 * g.scale),
                            );
                            painter.rect_filled(
                                bar_rect,
                                Rounding::same(1.0),
                                Color32::from_rgba_unmultiplied(
                                    active_col.r(),
                                    active_col.g(),
                                    active_col.b(),
                                    205,
                                ),
                            );
                        }

                        let display_text = if is_listening {
                            "...".to_string()
                        } else {
                            key_label
                        };
                        let text_col = if is_pressed {
                            if g.is_dock_tint {
                                active_col
                            } else {
                                COLOR_BG_DEEP
                            }
                        } else {
                            COLOR_TEXT_PRIMARY
                        };
                        let font_px = self.config.keys_font_size.clamp(8.0, 28.0) * g.scale;

                        if self.config.show_hold_ms {
                            painter.text(
                                Pos2::new(key_rect.center().x, key_rect.min.y + g.key_h * 0.36),
                                egui::Align2::CENTER_CENTER,
                                display_text,
                                egui::FontId::proportional(font_px),
                                text_col,
                            );
                            let ms_col = if is_pressed {
                                if g.is_dock_tint {
                                    Color32::from_rgba_unmultiplied(237, 237, 237, 185)
                                } else {
                                    Color32::from_rgba_unmultiplied(15, 15, 16, 200)
                                }
                            } else {
                                COLOR_TEXT_MUTED
                            };
                            painter.text(
                                Pos2::new(key_rect.center().x, key_rect.min.y + g.key_h * 0.68),
                                egui::Align2::CENTER_CENTER,
                                format!("{}ms", self.key_hold_ms[i]),
                                egui::FontId::monospace((9.5_f32 * g.scale).min(font_px * 0.70)),
                                ms_col,
                            );
                        } else {
                            painter.text(
                                key_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                display_text,
                                egui::FontId::proportional(font_px),
                                text_col,
                            );
                        }
                    }
                });
            });

            if !tosu_ok {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_space(((g.frame_w - 104.0) * 0.5).max(0.0));
                    draw_tosu_offline_pill(ui, 1.0);
                });
            }
        });
    }

    pub(super) fn ui_keys_tab(&mut self, ui: &mut egui::Ui, accent: Color32) {
        let unsafe_mode = self.config.unsafe_numbers;
        ui.columns(2, |columns| {
            columns[0].vertical(|ui| {
                egui::Frame::none()
                    .fill(COLOR_BG_CARD)
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("keystrokes overlay")
                                    .size(14.0)
                                    .strong()
                                    .color(COLOR_TEXT_PRIMARY),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .selectable_label(self.float_keys, "float window")
                                        .clicked()
                                    {
                                        self.float_keys = !self.float_keys;
                                        self.config.float_keys_open = self.float_keys;
                                        self.mark_dirty();
                                    }
                                    if ui
                                        .checkbox(&mut self.config.show_keys_overlay, "enabled")
                                        .changed()
                                    {
                                        self.mark_dirty();
                                    }
                                },
                            );
                        });
                        ui.horizontal(|ui| {
                            let (hint_text, hint_col) = if let Some(idx) = self.key_rebind_idx {
                                (
                                    format!(
                                        "listening key {}: press any key (esc cancels)",
                                        idx + 1
                                    ),
                                    accent,
                                )
                            } else {
                                (
                                    "click any keycap below to rebind".to_string(),
                                    COLOR_TEXT_MUTED,
                                )
                            };
                            ui.label(egui::RichText::new(hint_text).size(11.5).color(hint_col));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .selectable_label(self.test_hold_preview, "preview press")
                                        .clicked()
                                    {
                                        self.test_hold_preview = !self.test_hold_preview;
                                    }
                                },
                            );
                        });
                        ui.add_space(if self.config.show_trails { 64.0 } else { 24.0 });

                        let geom = keys_frame_geometry(&self.config, true);
                        ui.horizontal(|ui| {
                            ui.add_space(((ui.available_width() - geom.frame_w) / 2.0).max(0.0));
                            self.draw_keys_overlay_widget(ui, true);
                        });
                        ui.add_space(16.0);
                    });
            });

            columns[1].vertical(|ui| {
                egui::Frame::none()
                    .fill(COLOR_BG_CARD)
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("keys_cfg_scroll")
                            .max_height((ui.available_height() - 12.0).max(240.0))
                            .show(ui, |ui| {
                                let def_cfg = Config::default();
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("theme & colors")
                                            .size(13.0)
                                            .strong()
                                            .color(COLOR_TEXT_PRIMARY),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.button("reset keys").clicked() {
                                                self.config.reset_keys();
                                                self.mark_dirty();
                                                self.set_toast(
                                                    "keystrokes settings reset to defaults",
                                                );
                                            }
                                        },
                                    );
                                });
                                ui.add_space(6.0);
                                let mut changed = false;
                                ui.horizontal_wrapped(|ui| {
                                    ui.label("preset");
                                    for (st, lbl) in [
                                        ("dock-tint", "dock tint"),
                                        ("dock", "dock solid"),
                                        ("tiles", "separate"),
                                        ("flat", "flat"),
                                    ] {
                                        if ui
                                            .selectable_label(self.config.keys_style == st, lbl)
                                            .clicked()
                                        {
                                            self.config.keys_style = st.to_string();
                                            self.config.keys_bg_enabled = false;
                                            self.config.keys_show_accent_bar = false;
                                            if st == "dock" || st == "dock-tint" {
                                                self.config.key_gap = 0;
                                            } else if self.config.key_gap == 0 {
                                                self.config.key_gap = 6;
                                            }
                                            changed = true;
                                        }
                                    }
                                });
                                ui.add_space(4.0);
                                self.draw_theme_dropdown(ui, ThemeScope::Keys, accent);
                                if ui
                                    .checkbox(
                                        &mut self.config.keys_auto_bg_theme,
                                        "adapt colors from map background",
                                    )
                                    .changed()
                                {
                                    changed = true;
                                }

                                ui.separator();
                                ui.label(
                                    egui::RichText::new("geometry & trails")
                                        .size(13.0)
                                        .strong()
                                        .color(COLOR_TEXT_PRIMARY),
                                );
                                ui.add_space(4.0);

                                let is_dock_mode =
                                    matches!(self.config.keys_style.as_str(), "dock" | "dock-tint");
                                changed |= num_u32(
                                    ui,
                                    "key width",
                                    &mut self.config.key_size,
                                    def_cfg.key_size,
                                    32..=110,
                                    "px",
                                    unsafe_mode,
                                );
                                changed |= num_u32(
                                    ui,
                                    "key height",
                                    &mut self.config.key_height,
                                    def_cfg.key_height,
                                    32..=110,
                                    "px",
                                    unsafe_mode,
                                );
                                if !is_dock_mode {
                                    changed |= num_u32(
                                        ui,
                                        "key gap",
                                        &mut self.config.key_gap,
                                        6,
                                        0..=24,
                                        "px",
                                        unsafe_mode,
                                    );
                                    changed |= num_u32(
                                        ui,
                                        "key radius",
                                        &mut self.config.key_radius,
                                        def_cfg.key_radius,
                                        0..=28,
                                        "px",
                                        unsafe_mode,
                                    );
                                }
                                changed |= num_f32(
                                    ui,
                                    "font size",
                                    &mut self.config.keys_font_size,
                                    def_cfg.keys_font_size,
                                    10.0..=24.0,
                                    (6.0, 64.0),
                                    unsafe_mode,
                                );
                                changed |= num_f32(
                                    ui,
                                    "overlay scale",
                                    &mut self.config.keys_bg_scale,
                                    def_cfg.keys_bg_scale,
                                    0.5..=2.0,
                                    (0.1, 10.0),
                                    unsafe_mode,
                                );

                                ui.horizontal(|ui| {
                                    if ui
                                        .checkbox(&mut self.config.show_hold_ms, "hold ms")
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    if ui
                                        .checkbox(&mut self.config.show_trails, "trails")
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    if !is_dock_mode
                                        && ui
                                            .checkbox(
                                                &mut self.config.keys_bg_enabled,
                                                "outer frame",
                                            )
                                            .changed()
                                    {
                                        changed = true;
                                    }
                                });

                                if self.config.show_trails {
                                    changed |= num_f32(
                                        ui,
                                        "trail opacity",
                                        &mut self.config.trail_opacity,
                                        def_cfg.trail_opacity,
                                        0.05..=1.0,
                                        (0.0, 1.0),
                                        false,
                                    );
                                    changed |= num_f32(
                                        ui,
                                        "trail fade",
                                        &mut self.config.trail_fade,
                                        def_cfg.trail_fade,
                                        0.0..=1.0,
                                        (0.0, 1.0),
                                        false,
                                    );
                                    changed |= num_f32(
                                        ui,
                                        "trail speed",
                                        &mut self.config.trail_speed,
                                        def_cfg.trail_speed,
                                        1.0..=24.0,
                                        (0.5, 60.0),
                                        unsafe_mode,
                                    );
                                    changed |= num_u32(
                                        ui,
                                        "trail height",
                                        &mut self.config.trail_height,
                                        def_cfg.trail_height,
                                        80..=900,
                                        "px",
                                        unsafe_mode,
                                    );
                                }

                                ui.add_space(4.0);
                                egui::CollapsingHeader::new("advanced layout & colors")
                                    .id_salt("keys_advanced_hdr")
                                    .default_open(false)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("outer hex");
                                            if ui
                                                .add(
                                                    egui::TextEdit::singleline(
                                                        &mut self.config.key_color_outer,
                                                    )
                                                    .desired_width(68.0),
                                                )
                                                .changed()
                                            {
                                                changed = true;
                                            }
                                            ui.label("inner hex");
                                            if ui
                                                .add(
                                                    egui::TextEdit::singleline(
                                                        &mut self.config.key_color_inner,
                                                    )
                                                    .desired_width(68.0),
                                                )
                                                .changed()
                                            {
                                                changed = true;
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("idle bg");
                                            if ui
                                                .add(
                                                    egui::TextEdit::singleline(
                                                        &mut self.config.key_inactive_color,
                                                    )
                                                    .desired_width(68.0),
                                                )
                                                .changed()
                                            {
                                                changed = true;
                                            }
                                            ui.label("key border");
                                            if ui
                                                .add(
                                                    egui::TextEdit::singleline(
                                                        &mut self.config.key_border_color,
                                                    )
                                                    .desired_width(68.0),
                                                )
                                                .changed()
                                            {
                                                changed = true;
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("frame bg");
                                            if ui
                                                .add(
                                                    egui::TextEdit::singleline(
                                                        &mut self.config.keys_bg_color,
                                                    )
                                                    .desired_width(68.0),
                                                )
                                                .changed()
                                            {
                                                changed = true;
                                            }
                                            ui.label("frame border");
                                            if ui
                                                .add(
                                                    egui::TextEdit::singleline(
                                                        &mut self.config.keys_border_color,
                                                    )
                                                    .desired_width(68.0),
                                                )
                                                .changed()
                                            {
                                                changed = true;
                                            }
                                        });
                                        if ui
                                            .checkbox(
                                                &mut self.config.keys_show_accent_bar,
                                                "top hit-line",
                                            )
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                        if ui
                                            .checkbox(
                                                &mut self.config.per_key_colors,
                                                "custom per-key settings",
                                            )
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                        if self.config.per_key_colors {
                                            for i in 0..4 {
                                                ui.horizontal(|ui| {
                                                    ui.label(format!("k{}", i + 1));
                                                    if i < self.config.key_labels.len()
                                                        && ui
                                                            .add(
                                                                egui::TextEdit::singleline(
                                                                    &mut self.config.key_labels[i],
                                                                )
                                                                .desired_width(36.0),
                                                            )
                                                            .changed()
                                                    {
                                                        changed = true;
                                                    }
                                                    if i < self.config.key_colors.len()
                                                        && ui
                                                            .add(
                                                                egui::TextEdit::singleline(
                                                                    &mut self.config.key_colors[i],
                                                                )
                                                                .desired_width(68.0),
                                                            )
                                                            .changed()
                                                    {
                                                        changed = true;
                                                    }
                                                    if i < self.config.trail_widths.len()
                                                        && ui
                                                            .add(
                                                                egui::DragValue::new(
                                                                    &mut self.config.trail_widths
                                                                        [i],
                                                                )
                                                                .speed(1.0)
                                                                .range(
                                                                    8..=if unsafe_mode {
                                                                        9999
                                                                    } else {
                                                                        140
                                                                    },
                                                                )
                                                                .suffix("px"),
                                                            )
                                                            .changed()
                                                    {
                                                        changed = true;
                                                    }
                                                    if i < self.config.rgb_enabled_keys.len()
                                                        && ui
                                                            .checkbox(
                                                                &mut self.config.rgb_enabled_keys
                                                                    [i],
                                                                "rgb",
                                                            )
                                                            .changed()
                                                    {
                                                        changed = true;
                                                    }
                                                });
                                            }
                                        }
                                        changed |= num_u32(
                                            ui,
                                            "frame padding",
                                            &mut self.config.keys_bg_padding,
                                            def_cfg.keys_bg_padding,
                                            0..=28,
                                            "px",
                                            unsafe_mode,
                                        );
                                        changed |= num_u32(
                                            ui,
                                            "frame radius",
                                            &mut self.config.keys_bg_radius,
                                            def_cfg.keys_bg_radius,
                                            0..=28,
                                            "px",
                                            unsafe_mode,
                                        );
                                        changed |= num_f32(
                                            ui,
                                            "bg opacity",
                                            &mut self.config.keys_bg_opacity,
                                            def_cfg.keys_bg_opacity,
                                            0.0..=1.0,
                                            (0.0, 1.0),
                                            false,
                                        );
                                        changed |= num_f32(
                                            ui,
                                            "rgb wave speed",
                                            &mut self.config.rgb_speed,
                                            def_cfg.rgb_speed,
                                            0.2..=5.0,
                                            (0.1, 30.0),
                                            unsafe_mode,
                                        );
                                        ui.horizontal(|ui| {
                                            ui.label("key shape");
                                            let cur_shape = self
                                                .config
                                                .key_shapes
                                                .first()
                                                .cloned()
                                                .unwrap_or_else(|| "rounded".to_string());
                                            for sh in ["rounded", "square", "pill"] {
                                                if ui
                                                    .selectable_label(cur_shape == sh, sh)
                                                    .clicked()
                                                {
                                                    self.config.key_shapes =
                                                        vec![sh.to_string(); 4];
                                                    changed = true;
                                                }
                                            }
                                        });
                                        changed |= num_i32(
                                            ui,
                                            "position x",
                                            &mut self.config.keys_pos_x,
                                            def_cfg.keys_pos_x,
                                            -400..=3840,
                                            "px",
                                            unsafe_mode,
                                        );
                                        changed |= num_i32(
                                            ui,
                                            "position y",
                                            &mut self.config.keys_pos_y,
                                            def_cfg.keys_pos_y,
                                            -400..=2160,
                                            "px",
                                            unsafe_mode,
                                        );
                                        changed |= num_i32(
                                            ui,
                                            "offset x",
                                            &mut self.config.keys_bg_offset_x,
                                            def_cfg.keys_bg_offset_x,
                                            -400..=400,
                                            "px",
                                            unsafe_mode,
                                        );
                                        changed |= num_i32(
                                            ui,
                                            "offset y",
                                            &mut self.config.keys_bg_offset_y,
                                            def_cfg.keys_bg_offset_y,
                                            -400..=400,
                                            "px",
                                            unsafe_mode,
                                        );
                                    });

                                if changed {
                                    self.mark_dirty();
                                }
                            });
                    });
            });
        });
    }
}
