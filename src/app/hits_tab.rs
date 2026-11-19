use std::path::Path;
use std::sync::atomic::Ordering;

use eframe::egui::{self, Color32, Pos2, Rounding, Stroke, Vec2};

use crate::config::Config;
use crate::proxy::TOSU_CONNECTED;
use crate::theme::ThemeScope;
use crate::tosu_manager::{ensure_tosu_overlay_env, push_tosu_runtime_settings};

use super::widgets::{
    draw_tosu_offline_pill, num_f32, num_i32, num_u32, parse_hex_color, COLOR_ACCENT,
    COLOR_BG_CANVAS, COLOR_BG_CARD, COLOR_BORDER, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY,
};
use super::App;

pub struct HitsFrameMetrics {
    pub is_hud: bool,
    pub is_dock: bool,
    pub is_dock_ratio: bool,
    pub is_grid: bool,
    pub is_horizontal: bool,
    pub scale: f32,
    pub gap: f32,
    pub item_w: f32,
    pub item_h: f32,
    pub pad: f32,
    pub ratio_dock_w: f32,
    pub grid_col_gap: f32,
    pub content_w: f32,
    pub draw_w: f32,
}

pub fn hits_frame_metrics(cfg: &Config, tosu_ok: bool) -> HitsFrameMetrics {
    let style = cfg.hitcounter_style.as_str();
    let is_hud = style == "hud";
    let is_dock = style == "dock" || style == "dock-ratio";
    let is_dock_ratio = style == "dock-ratio";
    let is_grid = style == "grid-2x3";
    let is_horizontal = is_dock || (is_hud && cfg.hitcounter_orientation == "horizontal");
    let scale = cfg.hitcounter_scale.clamp(0.5, 1.5);
    let gap = if is_dock {
        0.0
    } else {
        cfg.hitcounter_gap as f32 * scale
    };
    let item_w = if is_dock {
        (cfg.hitcounter_item_width as f32).clamp(38.0, 96.0) * scale
    } else if is_grid {
        58.0 * scale
    } else {
        (cfg.hitcounter_item_width as f32).max(20.0) * scale
    };
    let item_h = if is_dock {
        (cfg.hitcounter_item_height as f32).clamp(34.0, 72.0) * scale
    } else {
        (cfg.hitcounter_item_height as f32).max(16.0) * scale
    };
    let pad = if is_hud {
        4.0 * scale
    } else if is_dock {
        0.0
    } else {
        cfg.hitcounter_padding as f32 * scale
    };
    let ratio_dock_w = 54.0 * scale;
    let grid_col_gap = 12.0 * scale;
    let content_w = if is_dock {
        6.0 * item_w + if is_dock_ratio { ratio_dock_w } else { 0.0 }
    } else if is_horizontal {
        6.0 * item_w + 5.0 * gap
    } else if is_grid {
        2.0 * item_w + grid_col_gap
    } else {
        item_w
    };
    let draw_w = if tosu_ok {
        content_w
    } else {
        content_w.max(108.0 * scale)
    };
    HitsFrameMetrics {
        is_hud,
        is_dock,
        is_dock_ratio,
        is_grid,
        is_horizontal,
        scale,
        gap,
        item_w,
        item_h,
        pad,
        ratio_dock_w,
        grid_col_gap,
        content_w,
        draw_w,
    }
}

impl App {
    pub(super) fn draw_hits_overlay_widget(&self, ui: &mut egui::Ui) {
        let tosu_ok = crate::proxy::TOSU_CONNECTED.load(Ordering::Relaxed);
        let m = hits_frame_metrics(&self.config, tosu_ok);
        let sc = m.scale;
        let item_rad = self.config.hitcounter_item_radius as f32 * sc;
        let label_sz = self.config.hitcounter_label_size * sc;
        let value_sz = self.config.hitcounter_value_size * sc;

        let default_colors = [
            "#ededed", "#e5b567", "#4cc38a", "#6cb6ff", "#d291e4", "#e5484d",
        ];
        let default_labels = ["max", "300", "200", "100", "50", "miss"];

        let mut outer_base = parse_hex_color(&self.config.hitcounter_bg_color, COLOR_BG_CANVAS);
        let outer_alpha = (self.config.hitcounter_opacity.clamp(0.05, 1.0) * 255.0) as u8;
        let mut outer_border = parse_hex_color(&self.config.hitcounter_border_color, COLOR_BORDER);
        let mut accent_col = parse_hex_color(&self.config.accent_color, COLOR_ACCENT);
        if self.config.hitcounter_auto_bg_theme && !self.radar_skills.adaptive_accent.is_empty() {
            outer_base = parse_hex_color(&self.radar_skills.adaptive_bg, outer_base);
            outer_border = parse_hex_color(&self.radar_skills.adaptive_border, outer_border);
            accent_col = parse_hex_color(&self.radar_skills.adaptive_accent, accent_col);
        }
        let outer_fill = if m.is_hud {
            Color32::TRANSPARENT
        } else {
            Color32::from_rgba_unmultiplied(
                outer_base.r(),
                outer_base.g(),
                outer_base.b(),
                outer_alpha,
            )
        };
        let label_col = parse_hex_color(&self.config.hitcounter_text_color, COLOR_TEXT_MUTED);
        let bw = self.config.hitcounter_border_width.clamp(0.0, 4.0) * sc;
        let border_stroke =
            if m.is_hud || self.config.hitcounter_border_style == "none" || bw <= 0.1 {
                Stroke::NONE
            } else {
                Stroke::new(bw, outer_border)
            };
        let frame_rad = item_rad.clamp(4.0 * sc, 12.0 * sc);

        let ratio_str = if self.hit_counts[1] > 0 {
            format!(
                "{:.2}",
                self.hit_counts[0] as f32 / self.hit_counts[1] as f32
            )
        } else if self.hit_counts[0] > 0 {
            format!("{}", self.hit_counts[0])
        } else {
            "0.00".to_string()
        };

        let outer_frame = egui::Frame::none()
            .fill(outer_fill)
            .stroke(border_stroke)
            .rounding(Rounding::same(frame_rad))
            .inner_margin(egui::Margin::same(m.pad));

        let draw_text_sh = |painter: &egui::Painter,
                            pos: Pos2,
                            align: egui::Align2,
                            txt: &str,
                            font: egui::FontId,
                            col: Color32| {
            if m.is_hud {
                painter.text(
                    Pos2::new(pos.x, pos.y + 1.0),
                    align,
                    txt,
                    font.clone(),
                    Color32::from_rgba_unmultiplied(9, 9, 11, 235),
                );
            }
            painter.text(pos, align, txt, font, col);
        };

        outer_frame.show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_width(m.draw_w);
                ui.spacing_mut().item_spacing = Vec2::ZERO;

                if !tosu_ok {
                    if m.is_hud {
                        ui.horizontal(|ui| {
                            ui.add_space(((m.draw_w - 100.0 * sc) * 0.5).max(0.0));
                            draw_tosu_offline_pill(ui, sc);
                        });
                        ui.add_space(4.0 * sc);
                    } else {
                        let status_h = 18.0 * sc;
                        let (srect, _) = ui.allocate_exact_size(
                            Vec2::new(m.draw_w, status_h),
                            egui::Sense::hover(),
                        );
                        let p = ui.painter();
                        let sy = srect.min.y + 6.5 * sc;
                        let sx = srect.min.x + m.pad.min(8.0 * sc);
                        p.circle_filled(
                            Pos2::new(sx + 4.0 * sc, sy),
                            2.5 * sc,
                            Color32::from_rgb(0xef, 0x44, 0x44),
                        );
                        p.text(
                            Pos2::new(sx + 10.0 * sc, sy),
                            egui::Align2::LEFT_CENTER,
                            "tosu! offline",
                            egui::FontId::monospace(9.5 * sc),
                            Color32::from_rgb(0xfc, 0xa5, 0xa5),
                        );
                        p.line_segment(
                            [
                                Pos2::new(srect.min.x, srect.min.y + 14.5 * sc),
                                Pos2::new(srect.max.x, srect.min.y + 14.5 * sc),
                            ],
                            Stroke::new(
                                1.0_f32,
                                Color32::from_rgba_unmultiplied(
                                    outer_border.r(),
                                    outer_border.g(),
                                    outer_border.b(),
                                    180,
                                ),
                            ),
                        );
                    }
                }

                let inner_x_pad = ((m.draw_w - m.content_w) * 0.5).max(0.0);

                if !m.is_dock && !m.is_horizontal && self.config.hitcounter_show_ratio {
                    let (rrect, _) = ui
                        .allocate_exact_size(Vec2::new(m.draw_w, 22.0 * sc), egui::Sense::hover());
                    let p = ui.painter();
                    let ry = rrect.min.y + 7.5 * sc;
                    let lx = rrect.min.x + inner_x_pad;
                    let rx = lx + m.content_w;
                    draw_text_sh(
                        p,
                        Pos2::new(lx, ry),
                        egui::Align2::LEFT_CENTER,
                        "ratio",
                        egui::FontId::proportional(label_sz),
                        Color32::from_rgb(0xd4, 0xd4, 0xd8),
                    );
                    draw_text_sh(
                        p,
                        Pos2::new(rx, ry),
                        egui::Align2::RIGHT_CENTER,
                        &ratio_str,
                        egui::FontId::monospace(value_sz),
                        accent_col,
                    );
                    if !m.is_hud {
                        let div_y = rrect.min.y + 17.0 * sc;
                        p.line_segment(
                            [Pos2::new(lx, div_y), Pos2::new(rx, div_y)],
                            Stroke::new(
                                1.0_f32,
                                Color32::from_rgba_unmultiplied(255, 255, 255, 18),
                            ),
                        );
                    }
                }

                let draw_cell = |ui: &mut egui::Ui, i: usize, w: f32, h: f32, horiz_cell: bool| {
                    let label = self
                        .config
                        .hitcounter_labels
                        .get(i)
                        .map(|s| s.as_str())
                        .unwrap_or(default_labels[i]);
                    let raw_color = parse_hex_color(
                        self.config
                            .hitcounter_colors
                            .get(i)
                            .map(|s| s.as_str())
                            .unwrap_or(default_colors[i]),
                        COLOR_TEXT_PRIMARY,
                    );
                    let count = self.hit_counts[i];
                    let color = if count == 0 {
                        Color32::from_rgba_unmultiplied(
                            raw_color.r(),
                            raw_color.g(),
                            raw_color.b(),
                            115,
                        )
                    } else {
                        raw_color
                    };
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), egui::Sense::hover());
                    let painter = ui.painter();
                    if !m.is_hud && horiz_cell && i > 0 {
                        let sep_x = rect.min.x - m.gap * 0.5;
                        painter.line_segment(
                            [
                                Pos2::new(sep_x, rect.min.y + 2.0 * sc),
                                Pos2::new(sep_x, rect.max.y - 2.0 * sc),
                            ],
                            Stroke::new(
                                1.0_f32,
                                Color32::from_rgba_unmultiplied(255, 255, 255, 16),
                            ),
                        );
                    }
                    if horiz_cell {
                        draw_text_sh(
                            painter,
                            Pos2::new(rect.center().x, rect.min.y + h * 0.30),
                            egui::Align2::CENTER_CENTER,
                            label,
                            egui::FontId::proportional((label_sz - 1.5 * sc).max(8.5 * sc)),
                            label_col,
                        );
                        draw_text_sh(
                            painter,
                            Pos2::new(rect.center().x, rect.min.y + h * 0.68),
                            egui::Align2::CENTER_CENTER,
                            &format!("{}", count),
                            egui::FontId::monospace(value_sz),
                            color,
                        );
                    } else {
                        let dot_d = self.config.hitcounter_dot_size as f32 * sc;
                        let mut label_x = rect.min.x;
                        if dot_d >= 2.5 && !m.is_grid {
                            let r = dot_d * 0.5;
                            painter.circle_filled(
                                Pos2::new(label_x + r, rect.center().y),
                                r,
                                Color32::from_rgba_unmultiplied(
                                    raw_color.r(),
                                    raw_color.g(),
                                    raw_color.b(),
                                    235,
                                ),
                            );
                            label_x += dot_d + 6.0 * sc;
                        }
                        draw_text_sh(
                            painter,
                            Pos2::new(label_x, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            label,
                            egui::FontId::proportional(label_sz),
                            label_col,
                        );
                        draw_text_sh(
                            painter,
                            Pos2::new(rect.max.x, rect.center().y),
                            egui::Align2::RIGHT_CENTER,
                            &format!("{}", count),
                            egui::FontId::monospace(value_sz),
                            color,
                        );
                    }
                };

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    if inner_x_pad > 0.0 {
                        ui.add_space(inner_x_pad);
                    }
                    if m.is_horizontal {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(m.gap, 0.0);
                            for i in 0..6 {
                                draw_cell(ui, i, m.item_w, m.item_h, true);
                            }
                            if m.is_dock_ratio {
                                let (rrect, _) = ui.allocate_exact_size(
                                    Vec2::new(m.ratio_dock_w, m.item_h),
                                    egui::Sense::hover(),
                                );
                                let p = ui.painter();
                                p.rect_filled(
                                    rrect,
                                    Rounding::ZERO,
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 5),
                                );
                                p.line_segment(
                                    [
                                        Pos2::new(rrect.min.x, rrect.min.y + 2.0 * sc),
                                        Pos2::new(rrect.min.x, rrect.max.y - 2.0 * sc),
                                    ],
                                    Stroke::new(
                                        1.0_f32,
                                        Color32::from_rgba_unmultiplied(255, 255, 255, 32),
                                    ),
                                );
                                draw_text_sh(
                                    p,
                                    Pos2::new(rrect.center().x, rrect.min.y + m.item_h * 0.30),
                                    egui::Align2::CENTER_CENTER,
                                    "ratio",
                                    egui::FontId::proportional((label_sz - 1.5 * sc).max(8.5 * sc)),
                                    label_col,
                                );
                                draw_text_sh(
                                    p,
                                    Pos2::new(rrect.center().x, rrect.min.y + m.item_h * 0.68),
                                    egui::Align2::CENTER_CENTER,
                                    &ratio_str,
                                    egui::FontId::monospace(value_sz),
                                    accent_col,
                                );
                            }
                        });
                    } else if m.is_grid {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(0.0, 4.0 * sc);
                            for row in 0..3 {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing = Vec2::new(m.grid_col_gap, 0.0);
                                    draw_cell(ui, row * 2, m.item_w, m.item_h, false);
                                    draw_cell(ui, row * 2 + 1, m.item_w, m.item_h, false);
                                });
                            }
                        });
                    } else {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(0.0, m.gap);
                            for i in 0..6 {
                                draw_cell(ui, i, m.item_w, m.item_h, false);
                            }
                        });
                    }
                });
            });
        });
    }

    pub(super) fn ui_hits_tab(&mut self, ui: &mut egui::Ui, accent: Color32) {
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
                                egui::RichText::new("hit counter overlay")
                                    .size(14.0)
                                    .strong()
                                    .color(COLOR_TEXT_PRIMARY),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.selectable_label(self.float_hits, "float window").clicked() {
                                    self.float_hits = !self.float_hits;
                                    self.config.float_hits_open = self.float_hits;
                                    self.mark_dirty();
                                }
                                if ui.checkbox(&mut self.config.show_hits_overlay, "enabled").changed() {
                                    self.mark_dirty();
                                }
                            });
                        });
                        ui.label(
                            egui::RichText::new("live judgment breakdown from osu! gameplay")
                                .size(11.5)
                                .color(COLOR_TEXT_MUTED),
                        );
                        ui.add_space(16.0);

                        let tosu_ok = TOSU_CONNECTED.load(Ordering::Relaxed);
                        let m = hits_frame_metrics(&self.config, tosu_ok);
                        ui.horizontal(|ui| {
                            let est_w = m.draw_w + 2.0 * m.pad;
                            let pad_left = ((ui.available_width() - est_w) * 0.5).max(0.0);
                            ui.add_space(pad_left);
                            self.draw_hits_overlay_widget(ui);
                        });
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
                            .id_salt("hits_cfg_scroll")
                            .max_height((ui.available_height() - 12.0).max(240.0))
                            .show(ui, |ui| {
                                let mut changed = false;
                                let unsafe_mode = self.config.unsafe_numbers;
                                let def_cfg = Config::default();

                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("theme & layout style")
                                            .size(13.0)
                                            .strong()
                                            .color(COLOR_TEXT_PRIMARY),
                                    );
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("reset hits").clicked() {
                                            self.config.reset_hits();
                                            self.mark_dirty();
                                            self.set_toast("hit counter settings reset to defaults");
                                        }
                                    });
                                });
                                ui.add_space(4.0);
                                ui.horizontal_wrapped(|ui| {
                                    ui.label("preset");
                                    for (st, lbl) in [
                                        ("radar-twin", "vertical"),
                                        ("grid-2x3", "compact grid"),
                                        ("dock", "horizontal"),
                                        ("dock-ratio", "bar + ratio"),
                                        ("hud", "minimal hud"),
                                    ] {
                                        if ui.selectable_label(self.config.hitcounter_style == st, lbl).clicked() {
                                            self.config.hitcounter_style = st.to_string();
                                            self.config.hitcounter_show_accent_bar = false;
                                            self.config.hitcounter_show_item_border = false;
                                            self.config.hitcounter_dot_size = 0;
                                            match st {
                                                "radar-twin" => {
                                                    self.config.hitcounter_orientation = "vertical".to_string();
                                                    self.config.hitcounter_item_width = 140;
                                                    self.config.hitcounter_item_height = 20;
                                                    self.config.hitcounter_gap = 2;
                                                    self.config.hitcounter_padding = 10;
                                                }
                                                "grid-2x3" => {
                                                    self.config.hitcounter_orientation = "vertical".to_string();
                                                    self.config.hitcounter_item_width = 58;
                                                    self.config.hitcounter_item_height = 20;
                                                    self.config.hitcounter_gap = 4;
                                                    self.config.hitcounter_padding = 10;
                                                }
                                                "dock" | "dock-ratio" => {
                                                    self.config.hitcounter_orientation = "horizontal".to_string();
                                                    self.config.hitcounter_item_width = 46;
                                                    self.config.hitcounter_item_height = 44;
                                                    self.config.hitcounter_gap = 0;
                                                    self.config.hitcounter_padding = 0;
                                                }
                                                _ => {}
                                            }
                                            changed = true;
                                        }
                                    }
                                });
                                if self.config.hitcounter_style == "hud" {
                                    ui.horizontal(|ui| {
                                        let is_vert = self.config.hitcounter_orientation == "vertical";
                                        if ui.selectable_label(is_vert, "vertical").clicked() {
                                            self.config.hitcounter_orientation = "vertical".to_string();
                                            self.config.hitcounter_item_width = 140;
                                            self.config.hitcounter_item_height = 20;
                                            changed = true;
                                        }
                                        if ui.selectable_label(!is_vert, "horizontal").clicked() {
                                            self.config.hitcounter_orientation = "horizontal".to_string();
                                            self.config.hitcounter_item_width = 48;
                                            self.config.hitcounter_item_height = 42;
                                            changed = true;
                                        }
                                    });
                                }
                                ui.add_space(4.0);
                                self.draw_theme_dropdown(ui, ThemeScope::Hits, accent);
                                ui.horizontal(|ui| {
                                    if ui
                                        .checkbox(
                                            &mut self.config.hitcounter_auto_bg_theme,
                                            "adapt colors from map",
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    if ui
                                        .checkbox(&mut self.config.hitcounter_show_ratio, "show ma ratio")
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                });

                                ui.separator();
                                ui.label(
                                    egui::RichText::new("dimensions & layout")
                                        .size(13.0)
                                        .strong()
                                        .color(COLOR_TEXT_PRIMARY),
                                );
                                ui.add_space(4.0);

                                changed |= num_u32(ui, "row width", &mut self.config.hitcounter_item_width, def_cfg.hitcounter_item_width, 44..=220, "px", unsafe_mode);
                                changed |= num_u32(ui, "row height", &mut self.config.hitcounter_item_height, def_cfg.hitcounter_item_height, 16..=72, "px", unsafe_mode);
                                changed |= num_u32(ui, "row gap", &mut self.config.hitcounter_gap, def_cfg.hitcounter_gap, 0..=18, "px", unsafe_mode);
                                changed |= num_u32(ui, "frame padding", &mut self.config.hitcounter_padding, def_cfg.hitcounter_padding, 0..=24, "px", unsafe_mode);
                                changed |= num_u32(ui, "corner radius", &mut self.config.hitcounter_item_radius, def_cfg.hitcounter_item_radius, 0..=24, "px", unsafe_mode);
                                changed |= num_f32(ui, "label size", &mut self.config.hitcounter_label_size, def_cfg.hitcounter_label_size, 9.0..=20.0, (6.0, 48.0), unsafe_mode);
                                changed |= num_f32(ui, "value size", &mut self.config.hitcounter_value_size, def_cfg.hitcounter_value_size, 9.0..=24.0, (6.0, 48.0), unsafe_mode);
                                changed |= num_u32(ui, "dot size", &mut self.config.hitcounter_dot_size, def_cfg.hitcounter_dot_size, 0..=14, "px", unsafe_mode);
                                changed |= num_f32(ui, "bg opacity", &mut self.config.hitcounter_opacity, def_cfg.hitcounter_opacity, 0.1..=1.0, (0.0, 1.0), false);
                                changed |= num_f32(ui, "overlay scale", &mut self.config.hitcounter_scale, def_cfg.hitcounter_scale, 0.5..=2.0, (0.1, 10.0), unsafe_mode);

                                ui.separator();
                                ui.label(
                                    egui::RichText::new("update cadence (gameplay hits & ratio)")
                                        .size(13.0)
                                        .strong()
                                        .color(COLOR_TEXT_PRIMARY),
                                );
                                ui.add_space(4.0);
                                let prev_poll_ms = self.config.hitcounter_poll_ms;
                                ui.horizontal_wrapped(|ui| {
                                    ui.label("mode");
                                    for (ms, lbl) in [
                                        (16u32, "real-time (16ms)"),
                                        (50u32, "balanced (50ms)"),
                                        (120u32, "low cpu (120ms)"),
                                    ] {
                                        if ui.selectable_label(self.config.hitcounter_poll_ms == ms, lbl).clicked() {
                                            self.config.hitcounter_poll_ms = ms;
                                            changed = true;
                                        }
                                    }
                                });
                                changed |= num_u32(
                                    ui,
                                    "poll interval",
                                    &mut self.config.hitcounter_poll_ms,
                                    def_cfg.hitcounter_poll_ms,
                                    8..=250,
                                    "ms",
                                    unsafe_mode,
                                );
                                ui.label(
                                    egui::RichText::new("lower ms keeps hits and ratio responsive; raise for older hardware")
                                        .size(10.5)
                                        .color(COLOR_TEXT_MUTED),
                                );
                                if self.config.hitcounter_poll_ms != prev_poll_ms {
                                    crate::config::update_config_cache(&self.config);
                                    if !self.config.tosu_root_path.is_empty() {
                                        ensure_tosu_overlay_env(Path::new(&self.config.tosu_root_path));
                                    }
                                    if TOSU_CONNECTED.load(Ordering::Relaxed) {
                                        push_tosu_runtime_settings(self.config.tosu_port.max(1024), &self.config);
                                    }
                                }

                                ui.add_space(4.0);
                                egui::CollapsingHeader::new("advanced colors, borders & labels")
                                    .id_salt("hits_advanced_hdr")
                                    .default_open(false)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("frame bg");
                                            if ui.add(egui::TextEdit::singleline(&mut self.config.hitcounter_bg_color).desired_width(68.0)).changed() {
                                                changed = true;
                                            }
                                            ui.label("item bg");
                                            if ui.add(egui::TextEdit::singleline(&mut self.config.hitcounter_item_bg_color).desired_width(68.0)).changed() {
                                                changed = true;
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("border hex");
                                            if ui.add(egui::TextEdit::singleline(&mut self.config.hitcounter_border_color).desired_width(68.0)).changed() {
                                                changed = true;
                                            }
                                            ui.label("label hex");
                                            if ui.add(egui::TextEdit::singleline(&mut self.config.hitcounter_text_color).desired_width(68.0)).changed() {
                                                changed = true;
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            let has_border = self.config.hitcounter_border_style != "none";
                                            let mut b_flag = has_border;
                                            if ui.checkbox(&mut b_flag, "frame border").changed() {
                                                self.config.hitcounter_border_style = if b_flag { "solid" } else { "none" }.to_string();
                                                changed = true;
                                            }
                                            if ui.checkbox(&mut self.config.hitcounter_show_accent_bar, "accent ticks").changed() {
                                                changed = true;
                                            }
                                            if ui.checkbox(&mut self.config.hitcounter_show_item_border, "item borders").changed() {
                                                changed = true;
                                            }
                                        });
                                        changed |= num_f32(ui, "border width", &mut self.config.hitcounter_border_width, def_cfg.hitcounter_border_width, 0.0..=3.0, (0.0, 8.0), unsafe_mode);
                                        changed |= num_f32(ui, "glow strength", &mut self.config.hitcounter_glow_strength, def_cfg.hitcounter_glow_strength, 0.0..=1.0, (0.0, 1.0), false);
                                        changed |= num_i32(ui, "position x", &mut self.config.hitcounter_position_x, def_cfg.hitcounter_position_x, -400..=3840, "px", unsafe_mode);
                                        changed |= num_i32(ui, "position y", &mut self.config.hitcounter_position_y, def_cfg.hitcounter_position_y, -400..=2160, "px", unsafe_mode);

                                        ui.separator();
                                        for i in 0..6 {
                                            if i < self.config.hitcounter_labels.len() && i < self.config.hitcounter_colors.len() {
                                                ui.horizontal(|ui| {
                                                    if ui.add(egui::TextEdit::singleline(&mut self.config.hitcounter_labels[i]).desired_width(56.0)).changed() {
                                                        changed = true;
                                                    }
                                                    if ui.add(egui::TextEdit::singleline(&mut self.config.hitcounter_colors[i]).desired_width(74.0)).changed() {
                                                        changed = true;
                                                    }
                                                });
                                            }
                                        }
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
