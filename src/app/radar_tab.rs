use std::f32::consts::PI;
use std::path::Path;
use std::sync::atomic::Ordering;

use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Vec2};

use crate::calc::{calculate_map_internal, SkillRatings};
use crate::config::Config;
use crate::theme::ThemeScope;

use super::widgets::{
    draw_tosu_offline_pill, num_f32, num_i32, num_u32, parse_hex_color, COLOR_BG_CANVAS,
    COLOR_BG_CARD, COLOR_BG_SURFACE, COLOR_BORDER, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY,
    COLOR_TEXT_SECONDARY,
};
use super::App;

fn calculate_from_osu_file(path: &Path, rate: f32) -> Result<SkillRatings, String> {
    let parent = path
        .parent()
        .ok_or_else(|| "invalid file parent".to_string())?;
    let songs_root = parent.parent().unwrap_or(parent);
    let folder = parent
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let file = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    calculate_map_internal(&songs_root.to_string_lossy(), &folder, &file, rate, "")
}

impl App {
    pub(super) fn draw_radar_overlay_widget(&mut self, ui: &mut egui::Ui, accent_prop: Color32) {
        let mut accent = accent_prop;
        let mut base_bg = parse_hex_color(&self.config.radar_bg_color, COLOR_BG_CANVAS);
        let mut border_col = parse_hex_color(&self.config.radar_border_color, COLOR_BORDER);
        if self.config.radar_auto_bg_theme && !self.radar_skills.adaptive_accent.is_empty() {
            accent = parse_hex_color(&self.radar_skills.adaptive_accent, accent);
            base_bg = parse_hex_color(&self.radar_skills.adaptive_bg, base_bg);
            border_col = parse_hex_color(&self.radar_skills.adaptive_border, border_col);
        }
        let text_col = parse_hex_color(&self.config.radar_text_color, COLOR_TEXT_SECONDARY);
        let alpha = (self.config.bg_opacity.clamp(0.05, 1.0) * 255.0) as u8;
        let card_fill =
            Color32::from_rgba_unmultiplied(base_bg.r(), base_bg.g(), base_bg.b(), alpha);
        let sc = self.config.scale.clamp(0.55, 1.35);
        let rad = self.config.radar_radius as f32 * sc;
        let max_w_lim = if self.config.unsafe_numbers {
            1600.0
        } else {
            720.0
        };
        let avail_w = ui.available_width().max(150.0);
        let radar_w = ((self.config.radar_width as f32).clamp(150.0, max_w_lim) * sc).min(avail_w);
        let pad_x = 14.0 * sc;
        let pad_y = 12.0 * sc;
        let card_inner_w = (radar_w - 2.0 * pad_x).max(110.0);
        let poly_box = (card_inner_w * 0.48).clamp(76.0, 220.0);

        if self.config.radar_show_map_bg && !self.radar_skills.bg_path.is_empty() {
            let need_load = self
                .radar_bg_tex
                .as_ref()
                .map(|(p, _)| p != &self.radar_skills.bg_path)
                .unwrap_or(true);
            if need_load {
                if let Ok(img) = image::open(&self.radar_skills.bg_path) {
                    let resized = img.thumbnail(720, 720);
                    let rgba = resized.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    let cimg = egui::ColorImage::from_rgba_unmultiplied(
                        [w as usize, h as usize],
                        &rgba.into_raw(),
                    );
                    let tex =
                        ui.ctx()
                            .load_texture("radar_map_bg", cimg, egui::TextureOptions::LINEAR);
                    self.radar_bg_tex = Some((self.radar_skills.bg_path.clone(), tex));
                } else {
                    self.radar_bg_tex = None;
                }
            }
        }

        let frame_resp = egui::Frame::none()
            .fill(card_fill)
            .stroke(Stroke::new(1.0_f32, border_col))
            .rounding(Rounding::same(rad))
            .inner_margin(egui::Margin::symmetric(pad_x, pad_y))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_width(card_inner_w);
                    if !self.config.radar_auto_height {
                        let max_h_lim = if self.config.unsafe_numbers {
                            1600.0
                        } else {
                            900.0
                        };
                        let target_h = (self.config.radar_height as f32 * sc - 2.0 * pad_y)
                            .clamp(110.0, max_h_lim);
                        ui.set_min_height(target_h);
                    }
                    let bg_slot = ui.painter().add(egui::Shape::Noop);
                    let tosu_ok = crate::proxy::TOSU_CONNECTED.load(Ordering::Relaxed);
                    if self.config.radar_show_header {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.set_max_width(
                                    (card_inner_w
                                        - if self.config.radar_show_badge {
                                            64.0
                                        } else {
                                            4.0
                                        })
                                    .max(72.0),
                                );
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&self.radar_map_title)
                                                .size(13.0)
                                                .color(COLOR_TEXT_PRIMARY),
                                        )
                                        .truncate(),
                                    );
                                    if !self.radar_map_diff.is_empty() {
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(format!(
                                                    "[{}]",
                                                    self.radar_map_diff
                                                ))
                                                .size(11.0)
                                                .color(COLOR_TEXT_SECONDARY),
                                            )
                                            .truncate(),
                                        );
                                    }
                                });
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&self.radar_map_artist)
                                            .size(11.0)
                                            .color(COLOR_TEXT_MUTED),
                                    )
                                    .truncate(),
                                );
                            });

                            if !tosu_ok {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::TOP),
                                    |ui| {
                                        draw_tosu_offline_pill(ui, 1.0);
                                    },
                                );
                            } else if self.config.radar_show_badge {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::TOP),
                                    |ui| {
                                        let tag = self
                                            .radar_skills
                                            .map_type
                                            .to_lowercase()
                                            .replace(" map", "");
                                        egui::Frame::none()
                                            .fill(COLOR_BG_SURFACE)
                                            .stroke(Stroke::new(1.0_f32, border_col))
                                            .rounding(Rounding::same(4.0))
                                            .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    egui::RichText::new(tag)
                                                        .monospace()
                                                        .size(10.0)
                                                        .color(accent),
                                                );
                                            });
                                    },
                                );
                            }
                        });

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(8.0);
                    } else if !tosu_ok {
                        ui.horizontal(|ui| {
                            draw_tosu_offline_pill(ui, 1.0);
                        });
                        ui.add_space(4.0);
                    }

                    ui.horizontal(|ui| {
                        if self.config.show_radar {
                            let (rect, _) = ui.allocate_exact_size(
                                Vec2::new(poly_box, poly_box + 8.0),
                                egui::Sense::hover(),
                            );
                            let painter = ui.painter_at(rect);
                            let center = rect.center();
                            let max_r = (poly_box * 0.40).max(26.0);
                            let max_val = self.config.radar_max_value.max(5.0);

                            let all_skills = [
                                ("stream", self.radar_skills.stream),
                                ("jumpstream", self.radar_skills.jumpstream),
                                ("handstream", self.radar_skills.handstream),
                                ("stamina", self.radar_skills.stamina),
                                ("jackspeed", self.radar_skills.jackspeed),
                                ("chordjack", self.radar_skills.chordjack),
                                ("technical", self.radar_skills.technical),
                            ];

                            let active: Vec<f32> = all_skills
                                .iter()
                                .filter(|(k, _)| {
                                    *self.config.visible_skills.get(*k).unwrap_or(&true)
                                })
                                .map(|(_, v)| *v)
                                .collect();
                            let values: Vec<f32> = if active.len() >= 3 {
                                active
                            } else {
                                all_skills.iter().map(|(_, v)| *v).collect()
                            };
                            let n = values.len();

                            for step in 1..=4 {
                                let ring_r = max_r * (step as f32 / 4.0);
                                let mut points = Vec::with_capacity(n + 1);
                                for i in 0..n {
                                    let angle = -PI / 2.0 + (i as f32) * (2.0 * PI / n as f32);
                                    points.push(Pos2::new(
                                        center.x + ring_r * angle.cos(),
                                        center.y + ring_r * angle.sin(),
                                    ));
                                }
                                points.push(points[0]);
                                painter.add(egui::epaint::PathShape::line(
                                    points,
                                    Stroke::new(1.0_f32, COLOR_BORDER),
                                ));
                            }

                            for i in 0..n {
                                let angle = -PI / 2.0 + (i as f32) * (2.0 * PI / n as f32);
                                let end = Pos2::new(
                                    center.x + max_r * angle.cos(),
                                    center.y + max_r * angle.sin(),
                                );
                                painter.line_segment(
                                    [center, end],
                                    Stroke::new(1.0_f32, COLOR_BORDER),
                                );
                            }

                            let mut poly_points = Vec::with_capacity(n);
                            for (i, val) in values.iter().enumerate() {
                                let norm = (val / max_val).clamp(0.08, 1.0);
                                let r = max_r * norm;
                                let angle = -PI / 2.0 + (i as f32) * (2.0 * PI / n as f32);
                                poly_points.push(Pos2::new(
                                    center.x + r * angle.cos(),
                                    center.y + r * angle.sin(),
                                ));
                            }

                            let fill_a =
                                (self.config.radar_fill_opacity.clamp(0.0, 1.0) * 255.0) as u8;
                            let poly_fill = Color32::from_rgba_unmultiplied(
                                accent.r(),
                                accent.g(),
                                accent.b(),
                                fill_a,
                            );
                            painter.add(egui::epaint::PathShape::convex_polygon(
                                poly_points.clone(),
                                poly_fill,
                                Stroke::new(1.75_f32, accent),
                            ));

                            if self.config.radar_show_dots {
                                for p in &poly_points {
                                    painter.circle_filled(*p, 2.2, accent);
                                }
                            }

                            ui.add_space(10.0);
                        }

                        ui.vertical(|ui| {
                            let right_w = if self.config.show_radar {
                                (card_inner_w - poly_box - 10.0).max(86.0)
                            } else {
                                card_inner_w
                            };
                            ui.set_min_width(right_w);
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("overall")
                                        .size(12.0)
                                        .color(COLOR_TEXT_PRIMARY),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:.2}",
                                                self.radar_skills.overall
                                            ))
                                            .monospace()
                                            .size(13.0)
                                            .color(accent),
                                        );
                                    },
                                );
                            });
                            ui.separator();

                            let mut rows = [
                                ("stream", self.radar_skills.stream),
                                ("jumpstream", self.radar_skills.jumpstream),
                                ("handstream", self.radar_skills.handstream),
                                ("stamina", self.radar_skills.stamina),
                                ("jackspeed", self.radar_skills.jackspeed),
                                ("chordjack", self.radar_skills.chordjack),
                                ("technical", self.radar_skills.technical),
                            ];
                            rows.sort_by(|a, b| b.1.total_cmp(&a.1));

                            for (name, val) in rows {
                                if *self.config.visible_skills.get(name).unwrap_or(&true) {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(name).size(11.5).color(text_col),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    egui::RichText::new(format!("{:.2}", val))
                                                        .monospace()
                                                        .size(11.5)
                                                        .color(COLOR_TEXT_PRIMARY),
                                                );
                                            },
                                        );
                                    });
                                }
                            }
                        });
                    });

                    let mut dan_col_resolved = accent;
                    if self.config.radar_show_dan {
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(5.0);

                        let (dan_level, dan_tier, dan_sym, dan_name, dan_var, dan_sub, _) =
                            if self.radar_skills.dan.tier > 0 {
                                self.radar_skills
                                    .dan
                                    .resolve_for_mode(&self.config.radar_dan_engine)
                            } else if self.radar_skills.overall > 0.1 {
                                let approx = (self.radar_skills.overall * 0.52).clamp(1.0, 20.99);
                                let (t, s, n, v, sub, d) = crate::dan::format_dan_tier(approx);
                                (approx, t, s, n, v, sub, d)
                            } else {
                                (
                                    0.0,
                                    0,
                                    "-".to_string(),
                                    "unrated".to_string(),
                                    "".to_string(),
                                    "".to_string(),
                                    "-".to_string(),
                                )
                            };

                        let dan_col = match self.config.radar_dan_color_mode.as_str() {
                            "tier" => parse_hex_color(
                                crate::dan::default_dan_tier_color(dan_tier),
                                accent,
                            ),
                            "custom" => parse_hex_color(&self.config.radar_dan_color, accent),
                            _ => accent,
                        };
                        dan_col_resolved = dan_col;
                        let badge_bg = if self.config.radar_auto_bg_theme
                            || self
                                .config
                                .radar_dan_badge_bg
                                .eq_ignore_ascii_case("#1d1d21")
                        {
                            Color32::from_rgb(
                                base_bg.r().saturating_add(9),
                                base_bg.g().saturating_add(9),
                                base_bg.b().saturating_add(11),
                            )
                        } else {
                            parse_hex_color(&self.config.radar_dan_badge_bg, base_bg)
                        };

                        let (bar_rect, _) = ui.allocate_exact_size(
                            Vec2::new(card_inner_w, 38.0),
                            egui::Sense::hover(),
                        );
                        let painter = ui.painter();
                        painter.rect_filled(bar_rect, Rounding::same(6.0), badge_bg);
                        painter.rect_stroke(
                            bar_rect,
                            Rounding::same(6.0),
                            Stroke::new(1.0_f32, border_col),
                        );

                        let strip_rect = Rect::from_min_size(
                            Pos2::new(bar_rect.min.x + 2.0, bar_rect.min.y + 4.0),
                            Vec2::new(3.5, bar_rect.height() - 8.0),
                        );
                        painter.rect_filled(strip_rect, Rounding::same(2.0), dan_col);

                        let sym_str = if dan_var.is_empty() {
                            dan_sym
                        } else {
                            format!("{}{}", dan_sym, dan_var)
                        };
                        let sym_px = (self.config.radar_dan_size as f32).clamp(11.0, 26.0);
                        painter.text(
                            Pos2::new(bar_rect.min.x + 26.0, bar_rect.center().y - 1.0),
                            egui::Align2::CENTER_CENTER,
                            sym_str,
                            egui::FontId::monospace(sym_px),
                            dan_col,
                        );

                        let info_x = bar_rect.min.x + 48.0;
                        let title_str = if dan_sub.is_empty() {
                            dan_name
                        } else {
                            format!("{} · {}", dan_name, dan_sub)
                        };
                        painter.text(
                            Pos2::new(
                                info_x,
                                if self.config.radar_dan_show_breakdown {
                                    bar_rect.min.y + 13.0
                                } else {
                                    bar_rect.center().y
                                },
                            ),
                            egui::Align2::LEFT_CENTER,
                            title_str,
                            egui::FontId::proportional(11.5),
                            COLOR_TEXT_PRIMARY,
                        );

                        if self.config.radar_dan_show_breakdown {
                            let sub_str = if self.radar_skills.dan.xdc1_dan > 0.1 {
                                format!(
                                    "xdc1 {:.2} · comp {:.2}",
                                    self.radar_skills.dan.xdc1_dan,
                                    self.radar_skills.dan.companella_dan
                                )
                            } else if self.radar_skills.dan.companella_dan > 0.1 {
                                format!("comp {:.2}", self.radar_skills.dan.companella_dan)
                            } else {
                                "xdc1 + companella".to_string()
                            };
                            painter.text(
                                Pos2::new(info_x, bar_rect.min.y + 26.5),
                                egui::Align2::LEFT_CENTER,
                                sub_str,
                                egui::FontId::monospace(9.5),
                                COLOR_TEXT_MUTED,
                            );
                        }

                        if self.config.radar_dan_show_numeric && dan_level > 0.05 {
                            painter.text(
                                Pos2::new(bar_rect.max.x - 10.0, bar_rect.center().y - 1.0),
                                egui::Align2::RIGHT_CENTER,
                                format!("{:.2}", dan_level),
                                egui::FontId::monospace(13.0),
                                dan_col,
                            );
                        }

                        let frac = (dan_level - dan_level.floor()).clamp(0.04, 1.0);
                        let prog_w = (bar_rect.width() - 16.0) * frac;
                        let prog_rect = Rect::from_min_size(
                            Pos2::new(bar_rect.min.x + 8.0, bar_rect.max.y - 3.5),
                            Vec2::new(prog_w, 2.0),
                        );
                        painter.rect_filled(
                            prog_rect,
                            Rounding::same(1.0),
                            Color32::from_rgba_unmultiplied(
                                dan_col.r(),
                                dan_col.g(),
                                dan_col.b(),
                                195,
                            ),
                        );
                    }

                    if self.config.radar_show_strain_graph {
                        ui.add_space(6.0);
                        let gh = (self.config.radar_graph_height as f32).clamp(16.0, 90.0);
                        let (grect, _) = ui
                            .allocate_exact_size(Vec2::new(card_inner_w, gh), egui::Sense::hover());
                        let painter = ui.painter();
                        let g_bg = Color32::from_rgba_unmultiplied(
                            base_bg.r().saturating_add(7),
                            base_bg.g().saturating_add(7),
                            base_bg.b().saturating_add(9),
                            215,
                        );
                        painter.rect_filled(grect, Rounding::same(5.0), g_bg);
                        painter.rect_stroke(
                            grect,
                            Rounding::same(5.0),
                            Stroke::new(1.0_f32, border_col),
                        );

                        let gcol = match self.config.radar_graph_color_mode.as_str() {
                            "dan" => dan_col_resolved,
                            "custom" => parse_hex_color(&self.config.radar_graph_color, accent),
                            _ => accent,
                        };
                        let curve = &self.radar_skills.dan.strain_curve;
                        let n = curve.len();
                        if n >= 2 {
                            let inner_x = grect.min.x + 3.0;
                            let inner_w = (grect.width() - 6.0).max(10.0);
                            let inner_top = grect.min.y + 4.0;
                            let inner_bot = grect.max.y - 3.0;
                            let inner_h = (inner_bot - inner_top).max(4.0);

                            let mut line_pts = Vec::with_capacity(n);
                            for (i, &val) in curve.iter().enumerate() {
                                let px = inner_x + (i as f32 / (n - 1) as f32) * inner_w;
                                let py = inner_bot - val.clamp(0.0, 1.0) * inner_h;
                                line_pts.push(Pos2::new(px, py));
                            }
                            let fa = (self.config.radar_graph_fill_opacity.clamp(0.0, 1.0) * 255.0)
                                as u8;
                            let fill_col =
                                Color32::from_rgba_unmultiplied(gcol.r(), gcol.g(), gcol.b(), fa);
                            for i in 0..n - 1 {
                                let p0 = line_pts[i];
                                let p1 = line_pts[i + 1];
                                let poly = vec![
                                    p0,
                                    p1,
                                    Pos2::new(p1.x, inner_bot),
                                    Pos2::new(p0.x, inner_bot),
                                ];
                                painter.add(egui::epaint::PathShape::convex_polygon(
                                    poly,
                                    fill_col,
                                    Stroke::NONE,
                                ));
                            }
                            painter.add(egui::epaint::PathShape::line(
                                line_pts,
                                Stroke::new(1.4_f32, gcol),
                            ));

                            let live_ms = crate::proxy::LIVE_SONG_TIME_MS.load(Ordering::Relaxed);
                            if self.config.radar_graph_show_cursor
                                && live_ms > 0
                                && self.radar_skills.dan.duration_ms > 0
                            {
                                let prog = (live_ms as f32
                                    / self.radar_skills.dan.duration_ms as f32)
                                    .clamp(0.0, 1.0);
                                let cx = inner_x + prog * inner_w;
                                painter.line_segment(
                                    [
                                        Pos2::new(cx, grect.min.y + 2.0),
                                        Pos2::new(cx, grect.max.y - 2.0),
                                    ],
                                    Stroke::new(
                                        1.2_f32,
                                        Color32::from_rgba_unmultiplied(237, 237, 237, 210),
                                    ),
                                );
                            }
                        } else {
                            let mid_y = grect.max.y - 6.0;
                            painter.line_segment(
                                [
                                    Pos2::new(grect.min.x + 6.0, mid_y),
                                    Pos2::new(grect.max.x - 6.0, mid_y),
                                ],
                                Stroke::new(
                                    1.0_f32,
                                    Color32::from_rgba_unmultiplied(
                                        gcol.r(),
                                        gcol.g(),
                                        gcol.b(),
                                        70,
                                    ),
                                ),
                            );
                        }
                    }

                    if self.config.radar_show_map_bg && self.config.radar_map_bg_opacity > 0.01 {
                        let bg_a = (self.config.radar_map_bg_opacity.clamp(0.0, 1.0) * 255.0) as u8;
                        let rect = ui.min_rect().expand2(Vec2::new(pad_x, pad_y));
                        let shape = if let Some((_, ref tex)) = self.radar_bg_tex {
                            let tex_sz = tex.size();
                            let ta = (tex_sz[0] as f32).max(1.0) / (tex_sz[1] as f32).max(1.0);
                            let ra = rect.width().max(1.0) / rect.height().max(1.0);
                            let (u0, v0, u1, v1) = if ta > ra {
                                let half = (ra / ta) * 0.5;
                                (0.5 - half, 0.0, 0.5 + half, 1.0)
                            } else {
                                let half = (ta / ra) * 0.5;
                                (0.0, 0.5 - half, 1.0, 0.5 + half)
                            };
                            let mut mesh = egui::Mesh::with_texture(tex.id());
                            mesh.add_rect_with_uv(
                                rect,
                                Rect::from_min_max(Pos2::new(u0, v0), Pos2::new(u1, v1)),
                                Color32::from_white_alpha(bg_a),
                            );
                            egui::Shape::mesh(mesh)
                        } else {
                            let tint = Color32::from_rgba_unmultiplied(
                                accent.r(),
                                accent.g(),
                                accent.b(),
                                (bg_a as f32 * 0.25) as u8,
                            );
                            egui::Shape::rect_filled(rect, Rounding::same(rad), tint)
                        };
                        ui.painter().set(bg_slot, shape);
                    }
                });
            });
        let _ = frame_resp;
    }

    pub(super) fn ui_radar_tab(&mut self, ui: &mut egui::Ui, accent: Color32) {
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
                                egui::RichText::new("msd radar overlay")
                                    .size(14.0)
                                    .strong()
                                    .color(COLOR_TEXT_PRIMARY),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .selectable_label(self.float_radar, "float window")
                                        .clicked()
                                    {
                                        self.float_radar = !self.float_radar;
                                        self.config.float_radar_open = self.float_radar;
                                        self.mark_dirty();
                                    }
                                    if ui
                                        .checkbox(&mut self.config.show_radar_overlay, "enabled")
                                        .changed()
                                    {
                                        self.mark_dirty();
                                    }
                                },
                            );
                        });
                        ui.label(
                            egui::RichText::new("live etterna minacalc 515 skill breakdown")
                                .size(11.5)
                                .color(COLOR_TEXT_MUTED),
                        );
                        ui.add_space(16.0);

                        let sc = self.config.scale.clamp(0.55, 1.35);
                        let max_w_lim = if self.config.unsafe_numbers {
                            1600.0
                        } else {
                            720.0
                        };
                        let est_w = ((self.config.radar_width as f32).clamp(150.0, max_w_lim) * sc)
                            .min(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.add_space(((ui.available_width() - est_w) * 0.5).max(0.0));
                            self.draw_radar_overlay_widget(ui, accent);
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
                            .id_salt("radar_cfg_scroll")
                            .max_height((ui.available_height() - 12.0).max(240.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("radar configuration")
                                            .size(13.0)
                                            .strong()
                                            .color(COLOR_TEXT_PRIMARY),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.button("reset radar").clicked() {
                                                self.config.reset_radar();
                                                self.mark_dirty();
                                                self.set_toast("radar settings reset to defaults");
                                            }
                                            if ui.button("open .osu file").clicked() {
                                                if let Some(file_path) = rfd::FileDialog::new()
                                                    .add_filter("osu! beatmap", &["osu"])
                                                    .pick_file()
                                                {
                                                    match calculate_from_osu_file(&file_path, 1.0) {
                                                        Ok(ratings) => {
                                                            self.radar_map_title = file_path
                                                                .file_stem()
                                                                .unwrap_or_default()
                                                                .to_string_lossy()
                                                                .to_string();
                                                            self.radar_map_diff.clear();
                                                            self.radar_skills = ratings;
                                                        }
                                                        Err(err) => {
                                                            self.set_toast(&format!(
                                                                "calc error: {}",
                                                                err
                                                            ));
                                                        }
                                                    }
                                                }
                                            }
                                        },
                                    );
                                });

                                ui.add_space(6.0);
                                self.draw_theme_dropdown(ui, ThemeScope::Radar, accent);

                                let mut changed = false;
                                let unsafe_mode = self.config.unsafe_numbers;
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    ui.label("accent hex");
                                    if ui
                                        .add(
                                            egui::TextEdit::singleline(
                                                &mut self.config.accent_color,
                                            )
                                            .desired_width(72.0),
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    ui.label("bg hex");
                                    if ui
                                        .add(
                                            egui::TextEdit::singleline(
                                                &mut self.config.radar_bg_color,
                                            )
                                            .desired_width(72.0),
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("border hex");
                                    if ui
                                        .add(
                                            egui::TextEdit::singleline(
                                                &mut self.config.radar_border_color,
                                            )
                                            .desired_width(72.0),
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    ui.label("text hex");
                                    if ui
                                        .add(
                                            egui::TextEdit::singleline(
                                                &mut self.config.radar_text_color,
                                            )
                                            .desired_width(72.0),
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                });

                                ui.separator();
                                ui.label(
                                    egui::RichText::new("layout & position")
                                        .size(13.0)
                                        .strong()
                                        .color(COLOR_TEXT_PRIMARY),
                                );
                                ui.add_space(4.0);

                                let def_cfg = Config::default();
                                changed |= num_u32(
                                    ui,
                                    "radar width",
                                    &mut self.config.radar_width,
                                    def_cfg.radar_width,
                                    180..=640,
                                    "px",
                                    unsafe_mode,
                                );
                                if ui
                                    .checkbox(&mut self.config.radar_auto_height, "auto height")
                                    .changed()
                                {
                                    changed = true;
                                }
                                if !self.config.radar_auto_height {
                                    changed |= num_u32(
                                        ui,
                                        "radar height",
                                        &mut self.config.radar_height,
                                        def_cfg.radar_height,
                                        180..=720,
                                        "px",
                                        unsafe_mode,
                                    );
                                }
                                changed |= num_f32(
                                    ui,
                                    "overlay scale",
                                    &mut self.config.scale,
                                    def_cfg.scale,
                                    0.5..=2.0,
                                    (0.1, 10.0),
                                    unsafe_mode,
                                );
                                changed |= num_f32(
                                    ui,
                                    "bg opacity",
                                    &mut self.config.bg_opacity,
                                    def_cfg.bg_opacity,
                                    0.1..=1.0,
                                    (0.0, 1.0),
                                    false,
                                );
                                changed |= num_f32(
                                    ui,
                                    "polygon fill",
                                    &mut self.config.radar_fill_opacity,
                                    def_cfg.radar_fill_opacity,
                                    0.0..=0.8,
                                    (0.0, 1.0),
                                    false,
                                );
                                changed |= num_u32(
                                    ui,
                                    "corner radius",
                                    &mut self.config.radar_radius,
                                    def_cfg.radar_radius,
                                    0..=24,
                                    "px",
                                    unsafe_mode,
                                );
                                changed |= num_f32(
                                    ui,
                                    "max msd scale",
                                    &mut self.config.radar_max_value,
                                    def_cfg.radar_max_value,
                                    20.0..=55.0,
                                    (5.0, 200.0),
                                    unsafe_mode,
                                );
                                changed |= num_i32(
                                    ui,
                                    "position x",
                                    &mut self.config.radar_pos_x,
                                    def_cfg.radar_pos_x,
                                    -400..=3840,
                                    "px",
                                    unsafe_mode,
                                );
                                changed |= num_i32(
                                    ui,
                                    "position y",
                                    &mut self.config.radar_pos_y,
                                    def_cfg.radar_pos_y,
                                    -400..=2160,
                                    "px",
                                    unsafe_mode,
                                );

                                ui.horizontal(|ui| {
                                    if ui
                                        .checkbox(&mut self.config.show_radar, "radar polygon")
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    if ui
                                        .checkbox(&mut self.config.radar_show_header, "header")
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    if ui
                                        .checkbox(&mut self.config.radar_show_badge, "type badge")
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    if ui
                                        .checkbox(&mut self.config.radar_show_dots, "vertex dots")
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                });

                                ui.separator();
                                ui.label(
                                    egui::RichText::new("beatmap background")
                                        .size(13.0)
                                        .strong()
                                        .color(COLOR_TEXT_PRIMARY),
                                );
                                ui.add_space(4.0);
                                if ui
                                    .checkbox(
                                        &mut self.config.radar_show_map_bg,
                                        "use selected beatmap background",
                                    )
                                    .changed()
                                {
                                    changed = true;
                                }
                                if self.config.radar_show_map_bg {
                                    changed |= num_f32(
                                        ui,
                                        "map bg opacity",
                                        &mut self.config.radar_map_bg_opacity,
                                        def_cfg.radar_map_bg_opacity,
                                        0.02..=0.95,
                                        (0.0, 1.0),
                                        false,
                                    );
                                }
                                if ui
                                    .checkbox(
                                        &mut self.config.radar_auto_bg_theme,
                                        "adapt radar colors from map background",
                                    )
                                    .changed()
                                {
                                    changed = true;
                                }

                                ui.separator();
                                ui.label(
                                    egui::RichText::new("dan calculator (xdc1 + companella)")
                                        .size(13.0)
                                        .strong()
                                        .color(COLOR_TEXT_PRIMARY),
                                );
                                ui.add_space(4.0);
                                if ui
                                    .checkbox(
                                        &mut self.config.radar_show_dan,
                                        "show dan rating below radar",
                                    )
                                    .changed()
                                {
                                    changed = true;
                                }
                                if self.config.radar_show_dan {
                                    ui.horizontal(|ui| {
                                        ui.label("engine");
                                        for (id, label) in [
                                            ("combined", "combined (1st..κ)"),
                                            ("xdc1", "xdc1 (daniel)"),
                                            ("companella", "companella (onnx)"),
                                        ] {
                                            if ui
                                                .selectable_label(
                                                    self.config.radar_dan_engine == id,
                                                    label,
                                                )
                                                .clicked()
                                            {
                                                self.config.radar_dan_engine = id.to_string();
                                                changed = true;
                                            }
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("color mode");
                                        for (id, label) in [
                                            ("accent", "match theme"),
                                            ("tier", "greek tier"),
                                            ("custom", "custom"),
                                        ] {
                                            if ui
                                                .selectable_label(
                                                    self.config.radar_dan_color_mode == id,
                                                    label,
                                                )
                                                .clicked()
                                            {
                                                self.config.radar_dan_color_mode = id.to_string();
                                                changed = true;
                                            }
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("custom hex");
                                        if ui
                                            .add(
                                                egui::TextEdit::singleline(
                                                    &mut self.config.radar_dan_color,
                                                )
                                                .desired_width(68.0),
                                            )
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                        ui.label("badge bg");
                                        if ui
                                            .add(
                                                egui::TextEdit::singleline(
                                                    &mut self.config.radar_dan_badge_bg,
                                                )
                                                .desired_width(68.0),
                                            )
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                    });
                                    changed |= num_u32(
                                        ui,
                                        "symbol size",
                                        &mut self.config.radar_dan_size,
                                        def_cfg.radar_dan_size,
                                        11..=24,
                                        "px",
                                        unsafe_mode,
                                    );
                                    ui.horizontal(|ui| {
                                        if ui
                                            .checkbox(
                                                &mut self.config.radar_dan_show_numeric,
                                                "numeric level",
                                            )
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                        if ui
                                            .checkbox(
                                                &mut self.config.radar_dan_show_breakdown,
                                                "xdc1/comp sub-row",
                                            )
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                    });
                                }

                                ui.separator();
                                ui.label(
                                    egui::RichText::new("difficulty curve (daniel xdc1)")
                                        .size(13.0)
                                        .strong()
                                        .color(COLOR_TEXT_PRIMARY),
                                );
                                ui.add_space(4.0);
                                if ui
                                    .checkbox(
                                        &mut self.config.radar_show_strain_graph,
                                        "show song difficulty curve",
                                    )
                                    .changed()
                                {
                                    changed = true;
                                }
                                if self.config.radar_show_strain_graph {
                                    changed |= num_u32(
                                        ui,
                                        "graph height",
                                        &mut self.config.radar_graph_height,
                                        def_cfg.radar_graph_height,
                                        18..=72,
                                        "px",
                                        unsafe_mode,
                                    );
                                    changed |= num_f32(
                                        ui,
                                        "graph fill",
                                        &mut self.config.radar_graph_fill_opacity,
                                        def_cfg.radar_graph_fill_opacity,
                                        0.05..=0.80,
                                        (0.0, 1.0),
                                        false,
                                    );
                                    ui.horizontal(|ui| {
                                        ui.label("curve color");
                                        for (id, label) in [
                                            ("accent", "match theme"),
                                            ("dan", "dan color"),
                                            ("custom", "custom"),
                                        ] {
                                            if ui
                                                .selectable_label(
                                                    self.config.radar_graph_color_mode == id,
                                                    label,
                                                )
                                                .clicked()
                                            {
                                                self.config.radar_graph_color_mode = id.to_string();
                                                changed = true;
                                            }
                                        }
                                    });
                                    if self.config.radar_graph_color_mode == "custom" {
                                        ui.horizontal(|ui| {
                                            ui.label("curve hex");
                                            if ui
                                                .add(
                                                    egui::TextEdit::singleline(
                                                        &mut self.config.radar_graph_color,
                                                    )
                                                    .desired_width(68.0),
                                                )
                                                .changed()
                                            {
                                                changed = true;
                                            }
                                        });
                                    }
                                    if ui
                                        .checkbox(
                                            &mut self.config.radar_graph_show_cursor,
                                            "live time cursor in gameplay",
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                }

                                ui.separator();
                                ui.label(
                                    egui::RichText::new("visible skills")
                                        .size(13.0)
                                        .strong()
                                        .color(COLOR_TEXT_PRIMARY),
                                );
                                ui.add_space(4.0);

                                for key in [
                                    "stream",
                                    "jumpstream",
                                    "handstream",
                                    "stamina",
                                    "jackspeed",
                                    "chordjack",
                                    "technical",
                                ] {
                                    let entry = self
                                        .config
                                        .visible_skills
                                        .entry(key.to_string())
                                        .or_insert(true);
                                    if ui.checkbox(entry, key).changed() {
                                        changed = true;
                                    }
                                }

                                if changed {
                                    self.mark_dirty();
                                }
                            });
                    });
            });
        });
    }
}
