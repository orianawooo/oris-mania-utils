use std::f32::consts::PI;

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use tiny_skia::{
    Color, FillRule, GradientStop, LineCap, LineJoin, LinearGradient, Paint, Path, PathBuilder,
    Pixmap, Point, SpreadMode, Stroke, Transform,
};

use crate::calc::SkillRatings;
use crate::config::Config;

#[derive(Clone)]
pub struct TrailBlock {
    pub key: usize,
    pub y: f32,
    pub h: f32,
    pub holding: bool,
    pub color: [u8; 3],
}

struct FontCache {
    sans: FontRef<'static>,
    mono: FontRef<'static>,
}

type CachedBgEntry = (String, u32, u32, std::sync::Arc<Vec<u8>>);

lazy_static::lazy_static! {
    static ref FONTS: FontCache = FontCache {
        sans: FontRef::try_from_slice(epaint_default_fonts::UBUNTU_LIGHT).expect("sans font"),
        mono: FontRef::try_from_slice(epaint_default_fonts::HACK_REGULAR).expect("mono font"),
    };
    static ref BG_IMG_CACHE: std::sync::Mutex<Option<CachedBgEntry>> = std::sync::Mutex::new(None);
}

pub fn get_cached_bg_rgba(
    path: &str,
    target_w: u32,
    target_h: u32,
) -> Option<(u32, u32, std::sync::Arc<Vec<u8>>)> {
    if path.trim().is_empty() {
        return None;
    }
    let tw = target_w.clamp(64, 1600);
    let th = target_h.clamp(64, 1600);
    if let Ok(guard) = BG_IMG_CACHE.lock() {
        if let Some((ref cached_path, w, h, ref bytes)) = *guard {
            if cached_path == path && w == tw && h == th {
                return Some((w, h, std::sync::Arc::clone(bytes)));
            }
        }
    }
    let img = image::open(path).ok()?;
    let resized = img.resize_to_fill(tw, th, image::imageops::FilterType::Triangle);
    let rgba = resized.to_rgba8();
    let (w, h) = rgba.dimensions();
    let arc = std::sync::Arc::new(rgba.into_raw());
    if let Ok(mut guard) = BG_IMG_CACHE.lock() {
        *guard = Some((path.to_string(), w, h, std::sync::Arc::clone(&arc)));
    }
    Some((w, h, arc))
}

pub fn hex_rgb(hex: &str, fallback: [u8; 3]) -> [u8; 3] {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&clean[0..2], 16),
            u8::from_str_radix(&clean[2..4], 16),
            u8::from_str_radix(&clean[4..6], 16),
        ) {
            return [r, g, b];
        }
    }
    fallback
}

fn rounded_rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    let r = r.min(w * 0.5).min(h * 0.5).max(0.0);
    let mut pb = PathBuilder::new();
    if r <= 0.1 {
        pb.move_to(x, y);
        pb.line_to(x + w, y);
        pb.line_to(x + w, y + h);
        pb.line_to(x, y + h);
        pb.close();
    } else {
        let k = r * 0.5522848;
        pb.move_to(x + r, y);
        pb.line_to(x + w - r, y);
        pb.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
        pb.line_to(x + w, y + h - r);
        pb.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
        pb.line_to(x + r, y + h);
        pb.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
        pb.line_to(x, y + r);
        pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
        pb.close();
    }
    pb.finish()
}

fn fill_rrect(pm: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, rgba: [u8; 4]) {
    if let Some(path) = rounded_rect_path(x, y, w, h, r) {
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]));
        paint.anti_alias = true;
        pm.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn fill_rrect_vgrad(
    pm: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    top_rgba: [u8; 4],
    bot_rgba: [u8; 4],
) {
    if let Some(path) = rounded_rect_path(x, y, w, h, r) {
        let stops = vec![
            GradientStop::new(
                0.0,
                Color::from_rgba8(top_rgba[0], top_rgba[1], top_rgba[2], top_rgba[3]),
            ),
            GradientStop::new(
                1.0,
                Color::from_rgba8(bot_rgba[0], bot_rgba[1], bot_rgba[2], bot_rgba[3]),
            ),
        ];
        if let Some(shader) = LinearGradient::new(
            Point::from_xy(x, y),
            Point::from_xy(x, y + h.max(1.0)),
            stops,
            SpreadMode::Pad,
            Transform::identity(),
        ) {
            let mut paint = Paint {
                shader,
                anti_alias: true,
                ..Default::default()
            };
            paint.anti_alias = true;
            pm.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn stroke_rrect(
    pm: &mut Pixmap,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    rgba: [u8; 4],
    width: f32,
) {
    if let Some(path) =
        rounded_rect_path(x + 0.5, y + 0.5, (w - 1.0).max(1.0), (h - 1.0).max(1.0), r)
    {
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]));
        paint.anti_alias = true;
        let stroke = Stroke {
            width,
            ..Default::default()
        };
        pm.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

fn draw_line_seg(pm: &mut Pixmap, x0: f32, y0: f32, x1: f32, y1: f32, rgba: [u8; 4], width: f32) {
    let mut pb = PathBuilder::new();
    pb.move_to(x0, y0);
    pb.line_to(x1, y1);
    if let Some(path) = pb.finish() {
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]));
        paint.anti_alias = true;
        let stroke = Stroke {
            width,
            ..Default::default()
        };
        pm.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

fn fill_circle(pm: &mut Pixmap, cx: f32, cy: f32, r: f32, rgba: [u8; 4]) {
    if let Some(path) = PathBuilder::from_circle(cx, cy, r) {
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]));
        paint.anti_alias = true;
        pm.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

#[derive(Clone, Copy)]
enum TextAlign {
    Left,
    Center,
    Right,
}

fn measure_text(text: &str, font: &FontRef<'static>, px: f32) -> f32 {
    let scaled = font.as_scaled(PxScale::from(px));
    let fallback_scaled = FONTS.mono.as_scaled(PxScale::from(px));
    let mut w = 0.0f32;
    let mut prev = None;
    for ch in text.chars() {
        let mut gid = scaled.glyph_id(ch);
        if gid.0 == 0 {
            gid = fallback_scaled.glyph_id(ch);
            w += fallback_scaled.h_advance(gid);
            prev = None;
            continue;
        }
        if let Some(p) = prev {
            w += scaled.kern(p, gid);
        }
        w += scaled.h_advance(gid);
        prev = Some(gid);
    }
    w
}

#[allow(clippy::too_many_arguments)]
fn draw_text(
    pm: &mut Pixmap,
    text: &str,
    x: f32,
    cy: f32,
    px: f32,
    mono: bool,
    rgba: [u8; 4],
    align: TextAlign,
) {
    if text.is_empty() {
        return;
    }
    let font = if mono { &FONTS.mono } else { &FONTS.sans };
    let alt_font = if mono { &FONTS.sans } else { &FONTS.mono };
    let scaled = font.as_scaled(PxScale::from(px));
    let alt_scaled = alt_font.as_scaled(PxScale::from(px));
    let total_w = measure_text(text, font, px);

    let start_x = match align {
        TextAlign::Left => x,
        TextAlign::Center => x - total_w * 0.5,
        TextAlign::Right => x - total_w,
    };
    let baseline = cy + (scaled.ascent() + scaled.descent()) * 0.5 - scaled.descent() * 0.15;

    let pw = pm.width() as i32;
    let ph = pm.height() as i32;
    let data = pm.data_mut();
    let alpha_scale = rgba[3] as f32 / 255.0;

    let mut pen_x = start_x;
    let mut prev = None;
    for ch in text.chars() {
        let mut use_font = font;
        let mut use_scaled = &scaled;
        let mut gid = scaled.glyph_id(ch);
        if gid.0 == 0 {
            gid = alt_scaled.glyph_id(ch);
            use_font = alt_font;
            use_scaled = &alt_scaled;
        } else if let Some(p) = prev {
            pen_x += scaled.kern(p, gid);
        }
        let glyph =
            gid.with_scale_and_position(PxScale::from(px), ab_glyph::point(pen_x, baseline));
        if let Some(outlined) = use_font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|gx, gy, cov| {
                let px_x = bounds.min.x as i32 + gx as i32;
                let px_y = bounds.min.y as i32 + gy as i32;
                if px_x < 0 || px_y < 0 || px_x >= pw || px_y >= ph {
                    return;
                }
                let src_a = (cov * alpha_scale).clamp(0.0, 1.0);
                if src_a <= 0.004 {
                    return;
                }
                let idx = ((px_y * pw + px_x) * 4) as usize;
                let inv_a = 1.0 - src_a;
                let r = (rgba[0] as f32 * src_a + data[idx] as f32 * inv_a).clamp(0.0, 255.0) as u8;
                let g =
                    (rgba[1] as f32 * src_a + data[idx + 1] as f32 * inv_a).clamp(0.0, 255.0) as u8;
                let b =
                    (rgba[2] as f32 * src_a + data[idx + 2] as f32 * inv_a).clamp(0.0, 255.0) as u8;
                let a = (255.0 * src_a + data[idx + 3] as f32 * inv_a).clamp(0.0, 255.0) as u8;
                data[idx] = r;
                data[idx + 1] = g;
                data[idx + 2] = b;
                data[idx + 3] = a;
            });
        }
        pen_x += use_scaled.h_advance(gid);
        prev = Some(gid);
    }
}

fn to_premultiplied_bgra(mut pm: Pixmap) -> Vec<u8> {
    let bytes = pm.data_mut();
    for chunk in bytes.as_chunks_mut::<4>().0 {
        chunk.swap(0, 2);
    }
    pm.take()
}

fn clamp_scale(raw: f32, unsafe_numbers: bool) -> f32 {
    if !raw.is_finite() {
        return 1.0;
    }
    if unsafe_numbers {
        raw.clamp(0.1, 10.0)
    } else {
        raw.clamp(0.5, 2.0)
    }
}

fn shape_radius(shape: &str, base: f32, w: f32, h: f32, sc: f32) -> f32 {
    match shape {
        "square" => 3.0 * sc,
        "pill" | "circle" => (w.min(h) * 0.5).max(base),
        _ => base,
    }
}

fn resolve_key_rgb(cfg: &Config, index: usize) -> [u8; 3] {
    let fallback = if index == 0 || index == 3 {
        hex_rgb(&cfg.key_color_outer, [0xe5, 0x88, 0xa6])
    } else {
        hex_rgb(&cfg.key_color_inner, [0xd4, 0x76, 0x95])
    };
    cfg.key_colors
        .get(index)
        .map(|s| hex_rgb(s, fallback))
        .unwrap_or(fallback)
}

fn hsv_to_rgb(hue_deg: f32) -> [u8; 3] {
    let h = (hue_deg.rem_euclid(360.0)) / 60.0;
    let x = 1.0 - ((h % 2.0) - 1.0).abs();
    let (r, g, b) = match h as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
}

pub fn keys_trail_area_h(cfg: &Config) -> f32 {
    let sc = clamp_scale(cfg.keys_bg_scale, cfg.unsafe_numbers);
    if cfg.show_trails {
        (cfg.trail_height as f32).clamp(20.0, 3400.0) * sc
    } else {
        8.0
    }
}

pub fn render_keys_bgra(
    cfg: &Config,
    pressed: &[bool; 4],
    hold_ms: &[u64; 4],
    trails: &[TrailBlock],
    phase_secs: f32,
) -> (u32, u32, Vec<u8>) {
    render_keys_bgra_live(cfg, pressed, hold_ms, trails, phase_secs, None, true)
}

fn draw_offline_pill(pm: &mut Pixmap, cx: f32, cy: f32, sc: f32, bg: [u8; 3], border: [u8; 3]) {
    let text = "tosu! offline";
    let font_px = 9.5 * sc;
    let tw = measure_text(text, &FONTS.mono, font_px);
    let pw = tw + 22.0 * sc;
    let ph = 16.0 * sc;
    let px = cx - pw * 0.5;
    let py = cy - ph * 0.5;
    fill_rrect(pm, px, py, pw, ph, 4.0 * sc, [bg[0], bg[1], bg[2], 235]);
    stroke_rrect(
        pm,
        px,
        py,
        pw,
        ph,
        4.0 * sc,
        [border[0], border[1], border[2], 255],
        1.0,
    );
    fill_circle(pm, px + 8.0 * sc, cy, 2.5 * sc, [0xef, 0x44, 0x44, 255]);
    draw_text(
        pm,
        text,
        px + 14.0 * sc,
        cy,
        font_px,
        true,
        [0xe4, 0xe4, 0xe7, 255],
        TextAlign::Left,
    );
}

pub fn render_keys_bgra_live(
    cfg: &Config,
    pressed: &[bool; 4],
    hold_ms: &[u64; 4],
    trails: &[TrailBlock],
    phase_secs: f32,
    skills: Option<&SkillRatings>,
    tosu_connected: bool,
) -> (u32, u32, Vec<u8>) {
    let sc = clamp_scale(cfg.keys_bg_scale, cfg.unsafe_numbers);
    let style = cfg.keys_style.as_str();
    let is_dock_tint = style == "dock-tint";
    let is_dock = style == "dock" || is_dock_tint;
    let is_flat = style == "flat";
    let key_w = (cfg.key_size as f32).max(12.0) * sc;
    let key_h = (cfg.key_height as f32).max(12.0) * sc;
    let gap = if is_dock {
        0.0
    } else {
        cfg.key_gap as f32 * sc
    };
    let pad = if is_dock {
        0.0
    } else if cfg.keys_bg_enabled {
        cfg.keys_bg_padding as f32 * sc
    } else {
        1.0 * sc
    };
    let bg_rad = cfg.keys_bg_radius as f32 * sc;
    let key_rad = cfg.key_radius as f32 * sc;
    let trail_area_h = keys_trail_area_h(cfg).min(3600.0);
    let offline_h = if tosu_connected { 0.0 } else { 20.0 * sc };

    let frame_w = 4.0 * key_w + 3.0 * gap + 2.0 * pad;
    let frame_h = key_h + 2.0 * pad;
    let min_w = if tosu_connected { 16.0 } else { 116.0 * sc };
    let draw_w = frame_w.max(min_w);
    let w = ((draw_w + 8.0).ceil() as u32).clamp(16, 4096);
    let h = ((trail_area_h + frame_h + offline_h + 8.0).ceil() as u32).clamp(16, 4096);

    let mut pm = Pixmap::new(w, h).expect("keys pixmap");
    let ox = 4.0 + (draw_w - frame_w) * 0.5;
    let frame_y = trail_area_h;

    let auto_skills = skills.filter(|s| cfg.keys_auto_bg_theme && !s.adaptive_accent.is_empty());

    let mut frame_bg = hex_rgb(&cfg.keys_bg_color, [0x14, 0x14, 0x15]);
    let mut frame_border = hex_rgb(&cfg.keys_border_color, [0x28, 0x28, 0x2b]);
    if let Some(s) = auto_skills {
        frame_bg = hex_rgb(&s.adaptive_bg, frame_bg);
        frame_border = hex_rgb(&s.adaptive_border, frame_border);
    }

    let frad = if is_dock {
        8.0 * sc
    } else {
        shape_radius(&cfg.keys_bg_shape, bg_rad, frame_w, frame_h, sc)
    };
    if cfg.keys_bg_enabled || is_dock {
        let bg_a = (cfg.keys_bg_opacity.clamp(0.0, 1.0) * 255.0) as u8;
        fill_rrect(
            &mut pm,
            ox,
            frame_y,
            frame_w,
            frame_h,
            frad,
            [frame_bg[0], frame_bg[1], frame_bg[2], bg_a],
        );
    }

    let key_rgb_for = |idx: usize| -> [u8; 3] {
        if let Some(s) = auto_skills {
            if idx == 0 || idx == 3 {
                hex_rgb(&s.adaptive_accent, [0xe5, 0x88, 0xa6])
            } else {
                hex_rgb(
                    &s.adaptive_accent_secondary,
                    hex_rgb(&s.adaptive_accent, [0xd4, 0x76, 0x95]),
                )
            }
        } else {
            resolve_key_rgb(cfg, idx)
        }
    };

    if cfg.show_trails {
        let base_alpha = cfg.trail_opacity.clamp(0.05, 1.0);
        let top_alpha = cfg.trail_fade.clamp(0.0, 1.0);
        for b in trails {
            if b.key >= 4 || b.h <= 1.0 {
                continue;
            }
            let kx = ox + pad + (b.key as f32) * (key_w + gap);
            let default_tw = (cfg.key_size as f32 - 8.0).max(16.0);
            let tw =
                (*cfg.trail_widths.get(b.key).unwrap_or(&(default_tw as u32)) as f32 * sc).max(4.0);
            let t_ox = *cfg.trail_offsets_x.get(b.key).unwrap_or(&0) as f32 * sc;
            let tx = kx + (key_w - tw) * 0.5 + t_ox;
            let base_y = frame_y + pad;
            let ty = (base_y - b.y * sc - b.h * sc).max(2.0);
            let bottom = (base_y - b.y * sc).min(base_y);
            let th = (bottom - ty).max(0.0);
            if th > 0.5 {
                let rgb = if *cfg.rgb_enabled_keys.get(b.key).unwrap_or(&false) {
                    let hue = phase_secs * 100.0 * cfg.rgb_speed.max(0.1) + (b.key as f32) * 35.0;
                    hsv_to_rgb(hue)
                } else {
                    key_rgb_for(b.key)
                };
                let tshape = cfg
                    .trail_shapes
                    .get(b.key)
                    .map(|s| s.as_str())
                    .unwrap_or("rounded");
                let trad = shape_radius(tshape, 3.0 * sc, tw, th, sc);
                let top_a = (top_alpha * 255.0).round() as u8;
                let bot_a = (base_alpha * 255.0).round() as u8;
                fill_rrect_vgrad(
                    &mut pm,
                    tx,
                    ty,
                    tw,
                    th,
                    trad,
                    [rgb[0], rgb[1], rgb[2], top_a],
                    [rgb[0], rgb[1], rgb[2], bot_a],
                );
            }
        }
    }

    let mut idle_rgb = hex_rgb(&cfg.key_inactive_color, [0x14, 0x14, 0x15]);
    let mut idle_border = hex_rgb(&cfg.key_border_color, [0x28, 0x28, 0x2b]);
    if let Some(s) = auto_skills {
        idle_rgb = hex_rgb(&s.adaptive_item_bg, idle_rgb);
        idle_border = hex_rgb(&s.adaptive_border, idle_border);
    }
    let font_px = cfg.keys_font_size.clamp(8.0, 48.0) * sc;
    let idle_alpha = (cfg.keys_bg_opacity.clamp(0.25, 1.0) * 255.0) as u8;

    for (i, &is_down) in pressed.iter().enumerate() {
        let kx = ox + pad + (i as f32) * (key_w + gap);
        let ky = frame_y + pad + if is_down && !is_dock { 1.0 * sc } else { 0.0 };
        let active_rgb = if *cfg.rgb_enabled_keys.get(i).unwrap_or(&false) && is_down {
            hsv_to_rgb(phase_secs * 100.0 * cfg.rgb_speed.max(0.1) + (i as f32) * 35.0)
        } else {
            key_rgb_for(i)
        };

        let kshape = cfg
            .key_shapes
            .get(i)
            .map(|s| s.as_str())
            .unwrap_or("rounded");
        let krad = if is_dock {
            if i == 0 || i == 3 {
                7.0 * sc
            } else {
                0.0
            }
        } else {
            shape_radius(kshape, key_rad, key_w, key_h, sc)
        };

        let fill = if is_down {
            if is_dock_tint {
                [active_rgb[0], active_rgb[1], active_rgb[2], 78]
            } else {
                [active_rgb[0], active_rgb[1], active_rgb[2], 248]
            }
        } else if is_dock {
            [0, 0, 0, 0]
        } else {
            [idle_rgb[0], idle_rgb[1], idle_rgb[2], idle_alpha]
        };
        if fill[3] > 0 {
            fill_rrect(&mut pm, kx, ky, key_w, key_h, krad, fill);
        }
        if is_down && is_dock_tint {
            let rim_x = if i == 0 { kx + 4.0 * sc } else { kx + 1.0 * sc };
            let rim_r = if i == 3 {
                kx + key_w - 4.0 * sc
            } else {
                kx + key_w - 1.0 * sc
            };
            fill_rrect(
                &mut pm,
                rim_x,
                ky,
                (rim_r - rim_x).max(4.0),
                2.0 * sc,
                1.0 * sc,
                [active_rgb[0], active_rgb[1], active_rgb[2], 195],
            );
        }

        if is_dock {
            if i > 0 {
                draw_line_seg(
                    &mut pm,
                    kx,
                    ky + 2.0 * sc,
                    kx,
                    ky + key_h - 2.0 * sc,
                    [255, 255, 255, 16],
                    1.0,
                );
            }
        } else if !is_flat {
            let border = if is_down {
                [active_rgb[0], active_rgb[1], active_rgb[2], 255]
            } else {
                [idle_border[0], idle_border[1], idle_border[2], 225]
            };
            stroke_rrect(&mut pm, kx, ky, key_w, key_h, krad, border, 1.0);
        }

        if cfg.keys_show_accent_bar && !is_down && !is_dock {
            let bar_w = (key_w - 10.0 * sc).max(6.0);
            fill_rrect(
                &mut pm,
                kx + (key_w - bar_w) * 0.5,
                ky + 2.5 * sc,
                bar_w,
                2.0 * sc,
                1.0 * sc,
                [active_rgb[0], active_rgb[1], active_rgb[2], 205],
            );
        }

        let label = cfg
            .key_labels
            .get(i)
            .filter(|s| !s.is_empty())
            .map(|s| s.as_str())
            .unwrap_or("K");
        let text_col = if is_down {
            if is_dock_tint {
                [active_rgb[0], active_rgb[1], active_rgb[2], 255]
            } else {
                [0x0f, 0x0f, 0x10, 255]
            }
        } else {
            [0xed, 0xed, 0xed, 255]
        };

        if cfg.show_hold_ms {
            draw_text(
                &mut pm,
                label,
                kx + key_w * 0.5,
                ky + key_h * 0.36,
                font_px,
                false,
                text_col,
                TextAlign::Center,
            );
            let ms_col = if is_down {
                if is_dock_tint {
                    [0xed, 0xed, 0xed, 185]
                } else {
                    [0x0f, 0x0f, 0x10, 200]
                }
            } else {
                [0x85, 0x85, 0x8b, 255]
            };
            draw_text(
                &mut pm,
                &format!("{}ms", hold_ms[i]),
                kx + key_w * 0.5,
                ky + key_h * 0.68,
                (9.5 * sc).min(font_px * 0.7),
                true,
                ms_col,
                TextAlign::Center,
            );
        } else {
            draw_text(
                &mut pm,
                label,
                kx + key_w * 0.5,
                ky + key_h * 0.5,
                font_px,
                false,
                text_col,
                TextAlign::Center,
            );
        }
    }

    if cfg.keys_bg_enabled || is_dock {
        let bg_a = (cfg.keys_bg_opacity.clamp(0.0, 1.0) * 255.0) as u8;
        stroke_rrect(
            &mut pm,
            ox,
            frame_y,
            frame_w,
            frame_h,
            frad,
            [
                frame_border[0],
                frame_border[1],
                frame_border[2],
                bg_a.max(85),
            ],
            1.0,
        );
    }

    if !tosu_connected {
        let pm_cx = (pm.width() as f32) * 0.5;
        draw_offline_pill(
            &mut pm,
            pm_cx,
            frame_y + frame_h + 11.0 * sc,
            sc,
            frame_bg,
            frame_border,
        );
    }

    (pm.width(), pm.height(), to_premultiplied_bgra(pm))
}

pub fn resolve_radar_palette(cfg: &Config, skills: &SkillRatings) -> ([u8; 3], [u8; 3], [u8; 3]) {
    let mut accent = hex_rgb(&cfg.accent_color, [0xe5, 0x88, 0xa6]);
    let mut bg = hex_rgb(&cfg.radar_bg_color, [0x14, 0x14, 0x15]);
    let mut border = hex_rgb(&cfg.radar_border_color, [0x28, 0x28, 0x2b]);
    if cfg.radar_auto_bg_theme && !skills.adaptive_accent.is_empty() {
        accent = hex_rgb(&skills.adaptive_accent, accent);
        bg = hex_rgb(&skills.adaptive_bg, bg);
        border = hex_rgb(&skills.adaptive_border, border);
    }
    (accent, bg, border)
}

pub fn render_radar_bgra(
    cfg: &Config,
    title: &str,
    artist: &str,
    diff: &str,
    skills: &SkillRatings,
) -> (u32, u32, Vec<u8>) {
    render_radar_bgra_live(cfg, title, artist, diff, skills, 0, true)
}

pub fn render_radar_bgra_live(
    cfg: &Config,
    title: &str,
    artist: &str,
    diff: &str,
    skills: &SkillRatings,
    song_time_ms: i64,
    tosu_connected: bool,
) -> (u32, u32, Vec<u8>) {
    let sc = clamp_scale(cfg.scale, cfg.unsafe_numbers);
    let all_skills = [
        ("stream", skills.stream),
        ("jumpstream", skills.jumpstream),
        ("handstream", skills.handstream),
        ("stamina", skills.stamina),
        ("jackspeed", skills.jackspeed),
        ("chordjack", skills.chordjack),
        ("technical", skills.technical),
    ];

    let vis_count = all_skills
        .iter()
        .filter(|(k, _)| *cfg.visible_skills.get(*k).unwrap_or(&true))
        .count();

    let has_curve = cfg.radar_show_strain_graph;
    let header_h = if cfg.radar_show_header {
        60.0 * sc
    } else {
        14.0 * sc
    };
    let base_w = if cfg.radar_width >= 160 {
        cfg.radar_width as f32
    } else if cfg.show_radar {
        312.0
    } else {
        186.0
    };
    let card_w = (if cfg.show_radar {
        base_w.max(210.0)
    } else {
        base_w.max(150.0)
    }) * sc;
    let right_h = (28.0 + (vis_count as f32) * 18.0) * sc;
    let auto_body_h = right_h.max(if cfg.show_radar { 144.0 * sc } else { 0.0 });
    let dan_h = if cfg.radar_show_dan { 50.0 * sc } else { 0.0 };
    let graph_h_px = (cfg.radar_graph_height as f32).clamp(16.0, 120.0);
    let graph_h = if has_curve {
        (graph_h_px + 14.0) * sc
    } else {
        0.0
    };
    let auto_card_h = header_h + auto_body_h + dan_h + graph_h + 14.0 * sc;
    let card_h = if cfg.radar_auto_height {
        auto_card_h
    } else {
        let target_h = (cfg.radar_height as f32).max(140.0) * sc;
        target_h.max(header_h + 70.0 * sc + dan_h + graph_h + 14.0 * sc)
    };
    let body_h = (card_h - header_h - dan_h - graph_h - 14.0 * sc).max(60.0 * sc);
    let w = ((card_w + 8.0).ceil() as u32).clamp(32, 4096);
    let h = ((card_h + 8.0).ceil() as u32).clamp(32, 4096);

    let mut pm = Pixmap::new(w, h).expect("radar pixmap");
    let ox = 4.0;
    let oy = 4.0;

    let (accent, bg_rgb, border_rgb) = resolve_radar_palette(cfg, skills);
    let text_rgb = hex_rgb(&cfg.radar_text_color, [0xa1, 0xa1, 0xa6]);
    let bg_a = (cfg.bg_opacity.clamp(0.05, 1.0) * 255.0) as u8;
    let rad = cfg.radar_radius as f32 * sc;

    fill_rrect(
        &mut pm,
        ox,
        oy,
        card_w,
        card_h,
        rad,
        [bg_rgb[0], bg_rgb[1], bg_rgb[2], bg_a],
    );

    if cfg.radar_show_map_bg && cfg.radar_map_bg_opacity > 0.01 {
        let img_alpha = cfg.radar_map_bg_opacity.clamp(0.0, 1.0);
        let pw = pm.width() as i32;
        let ph = pm.height() as i32;
        let x0 = (ox.floor() as i32).max(0);
        let y0 = (oy.floor() as i32).max(0);
        let x1 = ((ox + card_w).ceil() as i32).min(pw);
        let y1 = ((oy + card_h).ceil() as i32).min(ph);
        let cached = get_cached_bg_rgba(
            &skills.bg_path,
            card_w.round() as u32,
            card_h.round() as u32,
        );
        let data = pm.data_mut();
        for py in y0..y1 {
            let v = ((py as f32 - oy) / card_h.max(1.0)).clamp(0.0, 1.0);
            for px in x0..x1 {
                let idx = ((py * pw + px) * 4) as usize;
                if data[idx + 3] < 8 {
                    continue;
                }
                let u = ((px as f32 - ox) / card_w.max(1.0)).clamp(0.0, 1.0);
                let (sr, sg, sb) = if let Some((iw, ih, ref bytes)) = cached {
                    let ix = ((u * (iw as f32 - 1.0)) as u32).min(iw - 1);
                    let iy = ((v * (ih as f32 - 1.0)) as u32).min(ih - 1);
                    let sidx = ((iy * iw + ix) * 4) as usize;
                    (
                        bytes[sidx] as f32,
                        bytes[sidx + 1] as f32,
                        bytes[sidx + 2] as f32,
                    )
                } else {
                    let wave =
                        ((u * std::f32::consts::PI).sin() * (1.0 - v * 0.65)).clamp(0.0, 1.0);
                    (
                        bg_rgb[0] as f32
                            + (accent[0] as f32 - bg_rgb[0] as f32) * (0.45 * wave + 0.15),
                        bg_rgb[1] as f32
                            + (accent[1] as f32 - bg_rgb[1] as f32) * (0.45 * wave + 0.15),
                        bg_rgb[2] as f32
                            + (accent[2] as f32 - bg_rgb[2] as f32) * (0.45 * wave + 0.15),
                    )
                };
                let dst_a = data[idx + 3] as f32 / 255.0;
                let mix = img_alpha * dst_a;
                data[idx] = (data[idx] as f32 * (1.0 - mix) + sr * mix).clamp(0.0, 255.0) as u8;
                data[idx + 1] =
                    (data[idx + 1] as f32 * (1.0 - mix) + sg * mix).clamp(0.0, 255.0) as u8;
                data[idx + 2] =
                    (data[idx + 2] as f32 * (1.0 - mix) + sb * mix).clamp(0.0, 255.0) as u8;
            }
        }
    }

    stroke_rrect(
        &mut pm,
        ox,
        oy,
        card_w,
        card_h,
        rad,
        [border_rgb[0], border_rgb[1], border_rgb[2], 255],
        1.0,
    );

    let pad = 14.0 * sc;
    if cfg.radar_show_header {
        let max_title_chars = ((card_w / sc - 90.0) / 7.5).clamp(10.0, 64.0) as usize;
        let title_disp: String = if title.chars().count() > max_title_chars {
            format!(
                "{}..",
                title
                    .chars()
                    .take(max_title_chars.saturating_sub(2))
                    .collect::<String>()
            )
        } else {
            title.to_string()
        };
        draw_text(
            &mut pm,
            &title_disp,
            ox + pad,
            oy + 20.0 * sc,
            13.0 * sc,
            false,
            [0xed, 0xed, 0xed, 255],
            TextAlign::Left,
        );
        let sub = if diff.is_empty() {
            artist.to_string()
        } else {
            format!("{} [{}]", artist, diff)
        };
        let max_sub_chars = ((card_w / sc - 40.0) / 6.2).clamp(12.0, 72.0) as usize;
        let sub_disp: String = if sub.chars().count() > max_sub_chars {
            format!(
                "{}..",
                sub.chars()
                    .take(max_sub_chars.saturating_sub(2))
                    .collect::<String>()
            )
        } else {
            sub
        };
        draw_text(
            &mut pm,
            &sub_disp,
            ox + pad,
            oy + 35.0 * sc,
            10.5 * sc,
            false,
            [0x85, 0x85, 0x8b, 255],
            TextAlign::Left,
        );

        if !tosu_connected {
            let bw = measure_text("tosu! offline", &FONTS.mono, 9.5 * sc) + 20.0 * sc;
            let bx = ox + card_w - pad - bw;
            let by = oy + 12.0 * sc;
            let bh = 18.0 * sc;
            fill_rrect(&mut pm, bx, by, bw, bh, 4.0 * sc, [0x26, 0x16, 0x1a, 245]);
            stroke_rrect(
                &mut pm,
                bx,
                by,
                bw,
                bh,
                4.0 * sc,
                [0x7f, 0x29, 0x35, 255],
                1.0,
            );
            fill_circle(
                &mut pm,
                bx + 7.5 * sc,
                by + bh * 0.5,
                2.5 * sc,
                [0xef, 0x44, 0x44, 255],
            );
            draw_text(
                &mut pm,
                "tosu! offline",
                bx + 13.5 * sc,
                by + bh * 0.5,
                9.5 * sc,
                true,
                [0xfc, 0xa5, 0xa5, 255],
                TextAlign::Left,
            );
        } else if cfg.radar_show_badge {
            let badge = skills.map_type.to_lowercase().replace(" map", "");
            if !badge.is_empty() {
                let bw = (measure_text(&badge, &FONTS.mono, 10.0 * sc) + 12.0 * sc).max(40.0 * sc);
                let bx = ox + card_w - pad - bw;
                let by = oy + 12.0 * sc;
                let bh = 18.0 * sc;
                fill_rrect(&mut pm, bx, by, bw, bh, 4.0 * sc, [0x23, 0x23, 0x25, 255]);
                stroke_rrect(
                    &mut pm,
                    bx,
                    by,
                    bw,
                    bh,
                    4.0 * sc,
                    [border_rgb[0], border_rgb[1], border_rgb[2], 255],
                    1.0,
                );
                draw_text(
                    &mut pm,
                    &badge,
                    bx + bw * 0.5,
                    by + bh * 0.5,
                    10.0 * sc,
                    true,
                    [accent[0], accent[1], accent[2], 255],
                    TextAlign::Center,
                );
            }
        }

        draw_line_seg(
            &mut pm,
            ox + pad,
            oy + 48.0 * sc,
            ox + card_w - pad,
            oy + 48.0 * sc,
            [border_rgb[0], border_rgb[1], border_rgb[2], 200],
            1.0,
        );
    }

    let radar_col_w = if cfg.show_radar {
        ((card_w - 2.0 * pad) * 0.48).max(96.0 * sc)
    } else {
        0.0
    };
    if cfg.show_radar {
        let cx = ox + pad + radar_col_w * 0.5;
        let cy = oy + header_h + body_h * 0.5 - 2.0 * sc;
        let max_r = (radar_col_w.min(body_h) * 0.41).max(26.0 * sc);
        let max_val = cfg.radar_max_value.max(5.0);

        let active: Vec<f32> = all_skills
            .iter()
            .filter(|(k, _)| *cfg.visible_skills.get(*k).unwrap_or(&true))
            .map(|(_, v)| *v)
            .collect();
        let vals = if active.len() >= 3 {
            active
        } else {
            all_skills.iter().map(|(_, v)| *v).collect()
        };
        let n = vals.len();

        for step in 1..=4 {
            let rr = max_r * (step as f32 / 4.0);
            let mut pb = PathBuilder::new();
            for i in 0..n {
                let a = -PI / 2.0 + (i as f32) * (2.0 * PI / n as f32);
                let px = cx + rr * a.cos();
                let py = cy + rr * a.sin();
                if i == 0 {
                    pb.move_to(px, py);
                } else {
                    pb.line_to(px, py);
                }
            }
            pb.close();
            if let Some(path) = pb.finish() {
                let mut paint = Paint::default();
                paint.set_color(Color::from_rgba8(0xed, 0xed, 0xed, 20));
                paint.anti_alias = true;
                pm.stroke_path(
                    &path,
                    &paint,
                    &Stroke {
                        width: 1.0,
                        ..Default::default()
                    },
                    Transform::identity(),
                    None,
                );
            }
        }

        for i in 0..n {
            let a = -PI / 2.0 + (i as f32) * (2.0 * PI / n as f32);
            draw_line_seg(
                &mut pm,
                cx,
                cy,
                cx + max_r * a.cos(),
                cy + max_r * a.sin(),
                [0xed, 0xed, 0xed, 20],
                1.0,
            );
        }

        let mut pb = PathBuilder::new();
        let mut pts = Vec::with_capacity(n);
        for (i, v) in vals.iter().enumerate() {
            let norm = (v / max_val).clamp(0.08, 1.0);
            let r = max_r * norm;
            let a = -PI / 2.0 + (i as f32) * (2.0 * PI / n as f32);
            let px = cx + r * a.cos();
            let py = cy + r * a.sin();
            pts.push((px, py));
            if i == 0 {
                pb.move_to(px, py);
            } else {
                pb.line_to(px, py);
            }
        }
        pb.close();
        if let Some(path) = pb.finish() {
            let fill_a = (cfg.radar_fill_opacity.clamp(0.0, 1.0) * 255.0) as u8;
            let mut fill_paint = Paint::default();
            fill_paint.set_color(Color::from_rgba8(accent[0], accent[1], accent[2], fill_a));
            fill_paint.anti_alias = true;
            pm.fill_path(
                &path,
                &fill_paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );

            let mut stroke_paint = Paint::default();
            stroke_paint.set_color(Color::from_rgba8(accent[0], accent[1], accent[2], 255));
            stroke_paint.anti_alias = true;
            let st = Stroke {
                width: 1.6 * sc,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Default::default()
            };
            pm.stroke_path(&path, &stroke_paint, &st, Transform::identity(), None);
        }
        if cfg.radar_show_dots {
            for (px, py) in pts {
                fill_circle(
                    &mut pm,
                    px,
                    py,
                    2.2 * sc,
                    [accent[0], accent[1], accent[2], 255],
                );
            }
        }
    }

    let list_x = if cfg.show_radar {
        ox + pad + radar_col_w + 10.0 * sc
    } else {
        ox + pad
    };
    let list_right = ox + card_w - pad;
    let row_step = if vis_count > 0 {
        ((body_h - 32.0 * sc) / (vis_count as f32)).clamp(15.0 * sc, 28.0 * sc)
    } else {
        18.0 * sc
    };
    let mut row_y = oy + header_h + 7.0 * sc;

    draw_text(
        &mut pm,
        "overall",
        list_x,
        row_y,
        12.0 * sc,
        false,
        [0xed, 0xed, 0xed, 255],
        TextAlign::Left,
    );
    draw_text(
        &mut pm,
        &format!("{:.2}", skills.overall),
        list_right,
        row_y,
        12.5 * sc,
        true,
        [accent[0], accent[1], accent[2], 255],
        TextAlign::Right,
    );
    draw_line_seg(
        &mut pm,
        list_x,
        row_y + 10.0 * sc,
        list_right,
        row_y + 10.0 * sc,
        [border_rgb[0], border_rgb[1], border_rgb[2], 200],
        1.0,
    );
    row_y += 22.0 * sc;

    let mut list_skills = all_skills.to_vec();
    list_skills.sort_by(|a, b| b.1.total_cmp(&a.1));

    for (name, val) in list_skills {
        if *cfg.visible_skills.get(name).unwrap_or(&true) {
            draw_text(
                &mut pm,
                name,
                list_x,
                row_y,
                11.0 * sc,
                false,
                [text_rgb[0], text_rgb[1], text_rgb[2], 255],
                TextAlign::Left,
            );
            draw_text(
                &mut pm,
                &format!("{:.2}", val),
                list_right,
                row_y,
                11.5 * sc,
                true,
                [0xed, 0xed, 0xed, 255],
                TextAlign::Right,
            );
            row_y += row_step;
        }
    }

    let mut dan_rgb_resolved = accent;
    if cfg.radar_show_dan {
        let (dan_level, dan_tier, dan_sym, dan_name, dan_var, dan_sub, _) = if skills.dan.tier > 0 {
            skills.dan.resolve_for_mode(&cfg.radar_dan_engine)
        } else if skills.overall > 0.1 {
            let approx = (skills.overall * 0.52).clamp(1.0, 20.99);
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

        let dan_rgb = match cfg.radar_dan_color_mode.as_str() {
            "tier" => hex_rgb(crate::dan::default_dan_tier_color(dan_tier), accent),
            "custom" => hex_rgb(&cfg.radar_dan_color, accent),
            _ => accent,
        };
        dan_rgb_resolved = dan_rgb;
        let badge_bg =
            if cfg.radar_auto_bg_theme || cfg.radar_dan_badge_bg.eq_ignore_ascii_case("#1d1d21") {
                [
                    bg_rgb[0].saturating_add(9),
                    bg_rgb[1].saturating_add(9),
                    bg_rgb[2].saturating_add(11),
                ]
            } else {
                hex_rgb(&cfg.radar_dan_badge_bg, bg_rgb)
            };

        let sep_y = oy + header_h + body_h + 2.0 * sc;
        draw_line_seg(
            &mut pm,
            ox + pad,
            sep_y,
            ox + card_w - pad,
            sep_y,
            [border_rgb[0], border_rgb[1], border_rgb[2], 180],
            1.0,
        );

        let bx = ox + pad;
        let by = sep_y + 7.0 * sc;
        let bw = card_w - 2.0 * pad;
        let bh = 38.0 * sc;
        fill_rrect(
            &mut pm,
            bx,
            by,
            bw,
            bh,
            6.0 * sc,
            [badge_bg[0], badge_bg[1], badge_bg[2], 235],
        );
        stroke_rrect(
            &mut pm,
            bx,
            by,
            bw,
            bh,
            6.0 * sc,
            [border_rgb[0], border_rgb[1], border_rgb[2], 220],
            1.0,
        );

        fill_rrect(
            &mut pm,
            bx + 2.0 * sc,
            by + 4.0 * sc,
            3.5 * sc,
            bh - 8.0 * sc,
            2.0 * sc,
            [dan_rgb[0], dan_rgb[1], dan_rgb[2], 255],
        );

        let sym_str = if dan_var.is_empty() {
            dan_sym.clone()
        } else {
            format!("{}{}", dan_sym, dan_var)
        };
        let sym_px = (cfg.radar_dan_size as f32).clamp(11.0, 26.0) * sc;
        draw_text(
            &mut pm,
            &sym_str,
            bx + 26.0 * sc,
            by + bh * 0.48,
            sym_px,
            true,
            [dan_rgb[0], dan_rgb[1], dan_rgb[2], 255],
            TextAlign::Center,
        );

        let info_x = bx + 48.0 * sc;
        let title_str = if dan_sub.is_empty() {
            dan_name.clone()
        } else {
            format!("{} · {}", dan_name, dan_sub)
        };
        draw_text(
            &mut pm,
            &title_str,
            info_x,
            by + if cfg.radar_dan_show_breakdown {
                13.0 * sc
            } else {
                bh * 0.5
            },
            11.5 * sc,
            false,
            [0xed, 0xed, 0xed, 255],
            TextAlign::Left,
        );

        if cfg.radar_dan_show_breakdown {
            let sub_str = if cfg.show_radar && skills.dan.xdc1_dan > 0.1 {
                format!(
                    "xdc1 {:.2} · comp {:.2}",
                    skills.dan.xdc1_dan, skills.dan.companella_dan
                )
            } else if skills.dan.companella_dan > 0.1 {
                format!("comp {:.2}", skills.dan.companella_dan)
            } else {
                "xdc1 + companella".to_string()
            };
            draw_text(
                &mut pm,
                &sub_str,
                info_x,
                by + 26.5 * sc,
                9.5 * sc,
                true,
                [0x85, 0x85, 0x8b, 255],
                TextAlign::Left,
            );
        }

        if cfg.radar_dan_show_numeric && dan_level > 0.05 {
            draw_text(
                &mut pm,
                &format!("{:.2}", dan_level),
                bx + bw - 10.0 * sc,
                by + bh * 0.48,
                13.0 * sc,
                true,
                [dan_rgb[0], dan_rgb[1], dan_rgb[2], 255],
                TextAlign::Right,
            );
        }

        let frac = (dan_level - dan_level.floor()).clamp(0.04, 1.0);
        let prog_w = (bw - 16.0 * sc) * frac;
        fill_rrect(
            &mut pm,
            bx + 8.0 * sc,
            by + bh - 3.5 * sc,
            prog_w,
            2.0 * sc,
            1.0 * sc,
            [dan_rgb[0], dan_rgb[1], dan_rgb[2], 190],
        );
    }

    if has_curve {
        let gx = ox + pad;
        let gy = oy + header_h + body_h + dan_h + 6.0 * sc;
        let gw = card_w - 2.0 * pad;
        let gh = graph_h_px * sc;
        let g_bg = [
            bg_rgb[0].saturating_add(7),
            bg_rgb[1].saturating_add(7),
            bg_rgb[2].saturating_add(9),
        ];
        fill_rrect(
            &mut pm,
            gx,
            gy,
            gw,
            gh,
            5.0 * sc,
            [g_bg[0], g_bg[1], g_bg[2], 215],
        );
        stroke_rrect(
            &mut pm,
            gx,
            gy,
            gw,
            gh,
            5.0 * sc,
            [border_rgb[0], border_rgb[1], border_rgb[2], 190],
            1.0,
        );

        let gcol = match cfg.radar_graph_color_mode.as_str() {
            "dan" => dan_rgb_resolved,
            "custom" => hex_rgb(&cfg.radar_graph_color, accent),
            _ => accent,
        };
        let curve = &skills.dan.strain_curve;
        let n = curve.len();
        if n >= 2 {
            let inner_x = gx + 3.0 * sc;
            let inner_w = (gw - 6.0 * sc).max(10.0);
            let inner_top = gy + 4.0 * sc;
            let inner_bot = gy + gh - 3.0 * sc;
            let inner_h = (inner_bot - inner_top).max(4.0);

            let mut fill_pb = PathBuilder::new();
            let mut line_pb = PathBuilder::new();
            for (i, &val) in curve.iter().enumerate() {
                let px = inner_x + (i as f32 / (n - 1) as f32) * inner_w;
                let py = inner_bot - val.clamp(0.0, 1.0) * inner_h;
                if i == 0 {
                    fill_pb.move_to(px, inner_bot);
                    fill_pb.line_to(px, py);
                    line_pb.move_to(px, py);
                } else {
                    fill_pb.line_to(px, py);
                    line_pb.line_to(px, py);
                }
            }
            fill_pb.line_to(inner_x + inner_w, inner_bot);
            fill_pb.close();

            if let Some(fpath) = fill_pb.finish() {
                let fa = (cfg.radar_graph_fill_opacity.clamp(0.0, 1.0) * 255.0) as u8;
                let mut fpaint = Paint::default();
                fpaint.set_color(Color::from_rgba8(gcol[0], gcol[1], gcol[2], fa));
                fpaint.anti_alias = true;
                pm.fill_path(
                    &fpath,
                    &fpaint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
            if let Some(lpath) = line_pb.finish() {
                let mut lpaint = Paint::default();
                lpaint.set_color(Color::from_rgba8(gcol[0], gcol[1], gcol[2], 235));
                lpaint.anti_alias = true;
                let st = Stroke {
                    width: 1.4 * sc,
                    line_cap: LineCap::Round,
                    line_join: LineJoin::Round,
                    ..Default::default()
                };
                pm.stroke_path(&lpath, &lpaint, &st, Transform::identity(), None);
            }

            if cfg.radar_graph_show_cursor && song_time_ms > 0 && skills.dan.duration_ms > 0 {
                let prog = (song_time_ms as f32 / skills.dan.duration_ms as f32).clamp(0.0, 1.0);
                let cx = inner_x + prog * inner_w;
                draw_line_seg(
                    &mut pm,
                    cx,
                    gy + 2.0 * sc,
                    cx,
                    gy + gh - 2.0 * sc,
                    [0xed, 0xed, 0xed, 210],
                    1.2 * sc,
                );
            }
        } else {
            let mid_y = gy + gh - 6.0 * sc;
            draw_line_seg(
                &mut pm,
                gx + 6.0 * sc,
                mid_y,
                gx + gw - 6.0 * sc,
                mid_y,
                [gcol[0], gcol[1], gcol[2], 70],
                1.0 * sc,
            );
        }
    }

    if !cfg.radar_show_header && !tosu_connected {
        draw_offline_pill(
            &mut pm,
            ox + card_w * 0.5,
            oy + 12.0 * sc,
            sc,
            bg_rgb,
            border_rgb,
        );
    }

    (pm.width(), pm.height(), to_premultiplied_bgra(pm))
}

pub fn render_hits_bgra(cfg: &Config, hit_counts: &[u32; 6]) -> (u32, u32, Vec<u8>) {
    render_hits_bgra_live(cfg, hit_counts, None, true)
}

#[allow(clippy::too_many_arguments)]
fn draw_text_shadowed(
    pm: &mut Pixmap,
    text: &str,
    x: f32,
    cy: f32,
    px: f32,
    mono: bool,
    rgba: [u8; 4],
    align: TextAlign,
    shadow: bool,
) {
    if shadow {
        draw_text(
            pm,
            text,
            x + 1.0,
            cy + 1.0,
            px,
            mono,
            [0x09, 0x09, 0x0b, 210],
            align,
        );
    }
    draw_text(pm, text, x, cy, px, mono, rgba, align);
}

pub fn render_hits_bgra_live(
    cfg: &Config,
    hit_counts: &[u32; 6],
    skills: Option<&SkillRatings>,
    tosu_connected: bool,
) -> (u32, u32, Vec<u8>) {
    let sc = clamp_scale(cfg.hitcounter_scale, cfg.unsafe_numbers);
    let style = cfg.hitcounter_style.as_str();
    let is_hud = style == "hud";
    let is_dock = style == "dock" || style == "dock-ratio";
    let is_dock_ratio = style == "dock-ratio";
    let is_grid = style == "grid-2x3";
    let horiz = is_dock || (is_hud && cfg.hitcounter_orientation == "horizontal");

    let pad = if is_hud {
        4.0 * sc
    } else if is_dock {
        0.0
    } else {
        cfg.hitcounter_padding as f32 * sc
    };
    let gap = if is_dock {
        0.0
    } else {
        cfg.hitcounter_gap as f32 * sc
    };
    let iw = if is_dock {
        (cfg.hitcounter_item_width as f32).clamp(38.0, 96.0) * sc
    } else if is_grid {
        58.0 * sc
    } else {
        (cfg.hitcounter_item_width as f32).max(20.0) * sc
    };
    let ih = if is_dock {
        (cfg.hitcounter_item_height as f32).clamp(34.0, 72.0) * sc
    } else {
        (cfg.hitcounter_item_height as f32).max(16.0) * sc
    };
    let irad = cfg.hitcounter_item_radius as f32 * sc;
    let status_h = if tosu_connected { 0.0 } else { 18.0 * sc };
    let show_top_ratio = !is_dock && !horiz && cfg.hitcounter_show_ratio;
    let ratio_h = if show_top_ratio { 22.0 * sc } else { 0.0 };

    let content_w = if is_dock {
        6.0 * iw + if is_dock_ratio { 54.0 * sc } else { 0.0 }
    } else if horiz {
        6.0 * iw + 5.0 * gap
    } else if is_grid {
        2.0 * iw + 12.0 * sc
    } else {
        iw
    };
    let draw_w = if tosu_connected {
        content_w
    } else {
        content_w.max(108.0 * sc)
    };
    let box_w = draw_w + 2.0 * pad;
    let items_h = if horiz {
        ih
    } else if is_grid {
        ratio_h + 3.0 * ih + 2.0 * 4.0 * sc
    } else {
        ratio_h + 6.0 * ih + 5.0 * gap
    };
    let box_h = status_h + items_h + 2.0 * pad;
    let w = ((box_w + 8.0).ceil() as u32).clamp(32, 4096);
    let h = ((box_h + 8.0).ceil() as u32).clamp(32, 4096);

    let mut pm = Pixmap::new(w, h).expect("hits pixmap");
    let ox = 4.0;
    let oy = 4.0;

    let auto_skills = skills.filter(|s| cfg.hitcounter_auto_bg_theme && !s.adaptive_bg.is_empty());
    let mut bg_rgb = hex_rgb(&cfg.hitcounter_bg_color, [0x14, 0x14, 0x15]);
    let mut border_rgb = hex_rgb(&cfg.hitcounter_border_color, [0x28, 0x28, 0x2b]);
    let mut accent_rgb = hex_rgb(&cfg.accent_color, [0xe5, 0x88, 0xa6]);
    if let Some(s) = auto_skills {
        bg_rgb = hex_rgb(&s.adaptive_bg, bg_rgb);
        border_rgb = hex_rgb(&s.adaptive_border, border_rgb);
        accent_rgb = hex_rgb(&s.adaptive_accent, accent_rgb);
    }

    let bg_a = (cfg.hitcounter_opacity.clamp(0.05, 1.0) * 255.0) as u8;
    let label_rgb = hex_rgb(&cfg.hitcounter_text_color, [0x85, 0x85, 0x8d]);
    let b_width = cfg.hitcounter_border_width.clamp(0.0, 4.0) * sc;
    let frame_rad = irad.clamp(4.0 * sc, 12.0 * sc);

    if !is_hud {
        fill_rrect(
            &mut pm,
            ox,
            oy,
            box_w,
            box_h,
            frame_rad,
            [bg_rgb[0], bg_rgb[1], bg_rgb[2], bg_a],
        );
        if cfg.hitcounter_border_style != "none" && b_width > 0.1 {
            stroke_rrect(
                &mut pm,
                ox,
                oy,
                box_w,
                box_h,
                frame_rad,
                [border_rgb[0], border_rgb[1], border_rgb[2], 255],
                b_width,
            );
        }
    }

    if !tosu_connected {
        let sy = oy + pad + 6.5 * sc;
        if is_hud {
            draw_offline_pill(&mut pm, ox + box_w * 0.5, sy, sc, bg_rgb, border_rgb);
        } else {
            let sx = ox + pad.max(8.0 * sc);
            fill_circle(
                &mut pm,
                sx + 4.0 * sc,
                sy,
                2.5 * sc,
                [0xef, 0x44, 0x44, 255],
            );
            draw_text(
                &mut pm,
                "tosu! offline",
                sx + 10.0 * sc,
                sy,
                9.5 * sc,
                true,
                [0xfc, 0xa5, 0xa5, 255],
                TextAlign::Left,
            );
            draw_line_seg(
                &mut pm,
                ox + pad,
                oy + pad + 14.5 * sc,
                ox + box_w - pad,
                oy + pad + 14.5 * sc,
                [border_rgb[0], border_rgb[1], border_rgb[2], 180],
                1.0,
            );
        }
    }

    let ratio_str = if hit_counts[1] > 0 {
        format!("{:.2}", hit_counts[0] as f32 / hit_counts[1] as f32)
    } else if hit_counts[0] > 0 {
        format!("{}", hit_counts[0])
    } else {
        "0.00".to_string()
    };

    let base_x = ox + (box_w - content_w) * 0.5;
    let mut base_y = oy + pad + status_h;

    if show_top_ratio {
        let ry = base_y + 7.5 * sc;
        draw_text_shadowed(
            &mut pm,
            "ratio",
            base_x,
            ry,
            cfg.hitcounter_label_size * sc,
            false,
            [0xd4, 0xd4, 0xd8, 255],
            TextAlign::Left,
            is_hud,
        );
        draw_text_shadowed(
            &mut pm,
            &ratio_str,
            base_x + content_w,
            ry,
            cfg.hitcounter_value_size * sc,
            true,
            [accent_rgb[0], accent_rgb[1], accent_rgb[2], 255],
            TextAlign::Right,
            is_hud,
        );
        if !is_hud {
            let div_y = base_y + 17.0 * sc;
            draw_line_seg(
                &mut pm,
                base_x,
                div_y,
                base_x + content_w,
                div_y,
                [255, 255, 255, 18],
                1.0,
            );
        }
        base_y += ratio_h;
    }

    let default_colors = [
        [0xed, 0xed, 0xed],
        [0xe5, 0xb5, 0x67],
        [0x4c, 0xc3, 0x8a],
        [0x6c, 0xb6, 0xff],
        [0xd2, 0x91, 0xe4],
        [0xe5, 0x48, 0x4d],
    ];
    let default_labels = ["max", "300", "200", "100", "50", "miss"];
    let dot_d = cfg.hitcounter_dot_size as f32 * sc;

    for (i, &hit_val) in hit_counts.iter().enumerate() {
        let (rx, ry, cell_w) = if is_grid {
            let col_idx = i % 2;
            let row_idx = i / 2;
            (
                base_x + (col_idx as f32) * (iw + 12.0 * sc),
                base_y + (row_idx as f32) * (ih + 4.0 * sc),
                iw,
            )
        } else if horiz {
            (base_x + (i as f32) * (iw + gap), base_y, iw)
        } else {
            (base_x, base_y + (i as f32) * (ih + gap), iw)
        };

        let label = cfg
            .hitcounter_labels
            .get(i)
            .map(|s| s.as_str())
            .unwrap_or(default_labels[i]);
        let col = cfg
            .hitcounter_colors
            .get(i)
            .map(|s| hex_rgb(s, default_colors[i]))
            .unwrap_or(default_colors[i]);
        let val_a = if hit_val == 0 { 115 } else { 255 };

        if !is_hud && horiz && i > 0 {
            draw_line_seg(
                &mut pm,
                rx - gap * 0.5,
                ry + 2.0 * sc,
                rx - gap * 0.5,
                ry + ih - 2.0 * sc,
                [255, 255, 255, 16],
                1.0,
            );
        }

        if horiz {
            draw_text_shadowed(
                &mut pm,
                label,
                rx + cell_w * 0.5,
                ry + ih * 0.30,
                (cfg.hitcounter_label_size - 1.5).max(8.5) * sc,
                false,
                [label_rgb[0], label_rgb[1], label_rgb[2], 255],
                TextAlign::Center,
                is_hud,
            );
            draw_text_shadowed(
                &mut pm,
                &format!("{}", hit_val),
                rx + cell_w * 0.5,
                ry + ih * 0.68,
                cfg.hitcounter_value_size * sc,
                true,
                [col[0], col[1], col[2], val_a],
                TextAlign::Center,
                is_hud,
            );
        } else {
            let mut label_x = rx;
            if dot_d >= 2.5 && !is_grid {
                let r = dot_d * 0.5;
                fill_circle(
                    &mut pm,
                    label_x + r,
                    ry + ih * 0.5,
                    r,
                    [col[0], col[1], col[2], 235],
                );
                label_x += dot_d + 6.0 * sc;
            }
            let val_x = rx + cell_w;
            draw_text_shadowed(
                &mut pm,
                label,
                label_x,
                ry + ih * 0.5,
                cfg.hitcounter_label_size * sc,
                false,
                [label_rgb[0], label_rgb[1], label_rgb[2], 255],
                TextAlign::Left,
                is_hud,
            );
            draw_text_shadowed(
                &mut pm,
                &format!("{}", hit_val),
                val_x,
                ry + ih * 0.5,
                cfg.hitcounter_value_size * sc,
                true,
                [col[0], col[1], col[2], val_a],
                TextAlign::Right,
                is_hud,
            );
        }
    }

    if is_dock_ratio {
        let rx = base_x + 6.0 * iw;
        let rw = 54.0 * sc;
        fill_rrect(&mut pm, rx, base_y, rw, ih, 0.0, [255, 255, 255, 5]);
        draw_line_seg(
            &mut pm,
            rx,
            base_y + 2.0 * sc,
            rx,
            base_y + ih - 2.0 * sc,
            [255, 255, 255, 32],
            1.0,
        );
        draw_text_shadowed(
            &mut pm,
            "ratio",
            rx + rw * 0.5,
            base_y + ih * 0.30,
            (cfg.hitcounter_label_size - 1.5).max(8.5) * sc,
            false,
            [label_rgb[0], label_rgb[1], label_rgb[2], 255],
            TextAlign::Center,
            is_hud,
        );
        draw_text_shadowed(
            &mut pm,
            &ratio_str,
            rx + rw * 0.5,
            base_y + ih * 0.68,
            cfg.hitcounter_value_size * sc,
            true,
            [accent_rgb[0], accent_rgb[1], accent_rgb[2], 255],
            TextAlign::Center,
            is_hud,
        );
    }

    (pm.width(), pm.height(), to_premultiplied_bgra(pm))
}
