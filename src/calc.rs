use crate::dan::{calculate_combined_dan_with_od, DanResult, RawChartNote};
use minacalc_rs::{Calc, CalcMode, Note};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock};

thread_local! {
    static TLS_CALC: RefCell<Option<Calc>> = const { RefCell::new(None) };
}

fn with_calc<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce(&Calc) -> Result<R, String>,
{
    TLS_CALC.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = Some(Calc::new().map_err(|e| e.to_string())?);
        }
        let calc = slot
            .as_ref()
            .ok_or_else(|| "calc unavailable".to_string())?;
        f(calc)
    })
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SkillRatings {
    pub overall: f32,
    pub stream: f32,
    pub jumpstream: f32,
    pub handstream: f32,
    pub stamina: f32,
    pub jackspeed: f32,
    pub chordjack: f32,
    pub technical: f32,
    pub map_type: String,
    #[serde(default)]
    pub dan: DanResult,
    #[serde(default)]
    pub bg_path: String,
    #[serde(default)]
    pub adaptive_accent: String,
    #[serde(default)]
    pub adaptive_accent_secondary: String,
    #[serde(default)]
    pub adaptive_bg: String,
    #[serde(default)]
    pub adaptive_item_bg: String,
    #[serde(default)]
    pub adaptive_border: String,
}

fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = 0.5 * (max + min);
    let d = max - min;
    if d < 1e-5 {
        return (0.0, 0.0, l);
    }
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let mut h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    h /= 6.0;
    (h, s, l)
}

fn hsl_to_hex(h: f32, s: f32, l: f32) -> String {
    let hue2rgb = |p: f32, q: f32, mut t: f32| -> f32 {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            return p + (q - p) * 6.0 * t;
        }
        if t < 0.5 {
            return q;
        }
        if t < 2.0 / 3.0 {
            return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
        }
        p
    };
    let (r, g, b) = if s < 1e-5 {
        (l, l, l)
    } else {
        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;
        (
            hue2rgb(p, q, h + 1.0 / 3.0),
            hue2rgb(p, q, h),
            hue2rgb(p, q, h - 1.0 / 3.0),
        )
    };
    format!(
        "#{:02x}{:02x}{:02x}",
        (r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (b.clamp(0.0, 1.0) * 255.0).round() as u8
    )
}

fn extract_palette_from_image(
    img: image::DynamicImage,
) -> (String, String, String, String, String) {
    let thumb = img
        .resize_exact(24, 24, image::imageops::FilterType::Nearest)
        .to_rgb8();
    let mut hue_bins = [0.0_f32; 24];
    let mut bin_r = [0.0_f32; 24];
    let mut bin_g = [0.0_f32; 24];
    let mut bin_b = [0.0_f32; 24];

    for px in thumb.pixels() {
        let r = px[0] as f32 / 255.0;
        let g = px[1] as f32 / 255.0;
        let b = px[2] as f32 / 255.0;
        let (h, s, l) = rgb_to_hsl(r, g, b);
        if !(0.08..=0.92).contains(&l) {
            continue;
        }
        let weight = s * s * (1.0 - (2.0 * l - 1.0).abs() * 0.6) + 0.05;
        let idx = ((h * 24.0) as usize).min(23);
        hue_bins[idx] += weight;
        bin_r[idx] += r * weight;
        bin_g[idx] += g * weight;
        bin_b[idx] += b * weight;
    }

    let best_idx = hue_bins
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0);

    let w = hue_bins[best_idx];
    if w <= 1e-4 {
        return (
            "#e588a6".into(),
            "#d47695".into(),
            "#141415".into(),
            "#19191b".into(),
            "#28282b".into(),
        );
    }
    let (h, s, l) = rgb_to_hsl(
        bin_r[best_idx] / w,
        bin_g[best_idx] / w,
        bin_b[best_idx] / w,
    );
    let accent = hsl_to_hex(h, s.clamp(0.42, 0.85), l.clamp(0.60, 0.74));
    let accent_sec = hsl_to_hex(
        (h + 0.045) % 1.0,
        (s * 0.9).clamp(0.38, 0.80),
        (l * 0.92).clamp(0.54, 0.68),
    );
    let bg = hsl_to_hex(h, (s * 0.45).clamp(0.08, 0.34), 0.085);
    let item_bg = hsl_to_hex(h, (s * 0.38).clamp(0.08, 0.28), 0.148);
    let border = hsl_to_hex(h, (s * 0.40).clamp(0.10, 0.30), 0.225);
    (accent, accent_sec, bg, item_bg, border)
}

pub fn extract_adaptive_palette(path: &Path) -> (String, String, String, String, String) {
    let Ok(img) = image::open(path) else {
        return (
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        );
    };
    extract_palette_from_image(img)
}

pub fn extract_adaptive_palette_bytes(bytes: &[u8]) -> (String, String, String, String, String) {
    let Ok(img) = image::load_from_memory(bytes) else {
        return (
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        );
    };
    extract_palette_from_image(img)
}

fn disk_cache_path() -> Option<PathBuf> {
    dirs_next::data_local_dir().map(|d| d.join("oris-mania-utils").join("msd_dan_cache_v2.json"))
}

fn load_disk_cache() -> HashMap<String, SkillRatings> {
    if let Some(p) = disk_cache_path() {
        if let Ok(txt) = fs::read_to_string(p) {
            if let Ok(map) = serde_json::from_str::<HashMap<String, SkillRatings>>(&txt) {
                return map;
            }
        }
    }
    HashMap::new()
}

static SAVING_CACHE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn save_disk_cache_now(cache: &HashMap<String, SkillRatings>) {
    if let Some(p) = disk_cache_path() {
        if let Some(parent) = p.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(txt) = serde_json::to_string(cache) {
            let tmp = p.with_extension("json.tmp");
            if fs::write(&tmp, &txt).is_ok() && fs::rename(&tmp, &p).is_err() {
                let _ = fs::write(&p, txt);
                let _ = fs::remove_file(&tmp);
            }
        }
    }
}

fn schedule_disk_cache_save() {
    if SAVING_CACHE.swap(true, std::sync::atomic::Ordering::AcqRel) {
        return;
    }
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(600));
        SAVING_CACHE.store(false, std::sync::atomic::Ordering::Release);
        let snapshot = CALC_CACHE.lock().ok().map(|g| g.clone());
        if let Some(map) = snapshot {
            if map.len() <= 2500 {
                save_disk_cache_now(&map);
            }
        }
    });
}

lazy_static::lazy_static! {
    static ref CALC_CACHE: Mutex<HashMap<String, SkillRatings>> = Mutex::new(load_disk_cache());
    static ref SONGS_INDEX: RwLock<HashMap<String, PathBuf>> = RwLock::new(HashMap::new());
    static ref INDEXED_SONGS_PATH: RwLock<String> = RwLock::new(String::new());
}

pub fn rebuild_songs_index(songs_path: &str) {
    let songs_dir = Path::new(songs_path);
    if !songs_dir.is_dir() {
        return;
    }

    if let Ok(indexed_path) = INDEXED_SONGS_PATH.read() {
        if indexed_path.as_str() == songs_path {
            return;
        }
    }

    let mut index = match SONGS_INDEX.write() {
        Ok(g) => g,
        Err(_) => return,
    };
    index.clear();
    if let Ok(entries) = fs::read_dir(songs_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().trim().to_string();
            index.insert(name.to_lowercase(), entry.path());
        }
    }
    if let Ok(mut indexed_path) = INDEXED_SONGS_PATH.write() {
        *indexed_path = songs_path.to_string();
    }
}

pub fn find_osu_file(songs_path: &str, folder_or_id: &str, file: &str) -> Option<PathBuf> {
    let folder_clean_str = folder_or_id.trim().replace('\\', "/");
    let folder_trimmed = folder_clean_str.trim();
    let file_clean_str = file.trim().replace('\\', "/");
    let file_trimmed = Path::new(&file_clean_str)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or(file_clean_str);
    let file_trimmed = file_trimmed.trim();

    if songs_path.trim().is_empty() || folder_trimmed.is_empty() || file_trimmed.is_empty() {
        return None;
    }

    let songs_dir = Path::new(songs_path);
    if !songs_dir.is_dir() {
        return None;
    }

    let find_in_dir = |dir: &Path| -> Option<PathBuf> {
        let direct = dir.join(file_trimmed);
        if direct.is_file() {
            return Some(direct);
        }
        let file_lower = file_trimmed.to_lowercase();
        let file_clean = file_lower.trim_start_matches([' ', '-']);
        let prefix_len = file_clean.len().min(20);
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                let name_lower = name_str.trim().to_lowercase();
                if !name_lower.ends_with(".osu") {
                    continue;
                }
                let name_clean = name_lower.trim_start_matches([' ', '-']);
                if name_lower == file_lower
                    || name_clean == file_clean
                    || (prefix_len >= 8 && name_clean.starts_with(&file_clean[..prefix_len]))
                {
                    let candidate = dir.join(name_str.as_ref());
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }
        None
    };

    let direct_folder = songs_dir.join(folder_trimmed);
    if direct_folder.is_dir() {
        if let Some(p) = find_in_dir(&direct_folder) {
            return Some(p);
        }
    }

    let folder_lower = folder_trimmed.to_lowercase();
    let numeric_prefix = folder_lower
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .unwrap_or("");

    let mut index_empty = true;
    if let Ok(index) = SONGS_INDEX.read() {
        index_empty = index.is_empty();
        if let Some(dir_path) = index.get(&folder_lower) {
            if let Some(p) = find_in_dir(dir_path) {
                return Some(p);
            }
        }
        if !numeric_prefix.is_empty() {
            if let Some(dir_path) = index.get(numeric_prefix) {
                if let Some(p) = find_in_dir(dir_path) {
                    return Some(p);
                }
            }
        }
        if !numeric_prefix.is_empty() || folder_lower.len() >= 4 {
            for (key, dir_path) in index.iter() {
                if key.len() < 3 {
                    continue;
                }
                let key_num = key
                    .split(|c: char| !c.is_ascii_digit())
                    .next()
                    .unwrap_or("");
                if (!numeric_prefix.is_empty() && key_num == numeric_prefix)
                    || (folder_lower.len() >= 4
                        && (key.starts_with(&folder_lower) || folder_lower.starts_with(key)))
                {
                    if let Some(p) = find_in_dir(dir_path) {
                        return Some(p);
                    }
                }
            }
        }
    }

    if index_empty || !numeric_prefix.is_empty() {
        if let Ok(entries) = fs::read_dir(songs_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                let trimmed = name_str.trim();
                let hit = trimmed.eq_ignore_ascii_case(folder_trimmed)
                    || (!numeric_prefix.is_empty()
                        && (trimmed == numeric_prefix
                            || trimmed.starts_with(&format!("{} ", numeric_prefix))));
                if hit {
                    let entry_path = entry.path();
                    if let Some(p) = find_in_dir(&entry_path) {
                        if let Ok(mut index) = SONGS_INDEX.write() {
                            index.insert(trimmed.to_lowercase(), entry_path);
                        }
                        return Some(p);
                    }
                }
            }
        }
    }

    None
}

pub type ParsedChart = (u32, Vec<(u32, f32)>, Vec<RawChartNote>, String);
pub type ParsedChartWithOd = (u32, f32, Vec<(u32, f32)>, Vec<RawChartNote>, String);

pub fn parse_osu_to_bitmask_rows(path: &str) -> Result<ParsedChart, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    parse_osu_to_bitmask_rows_str(&content)
}

pub fn parse_osu_to_bitmask_rows_with_od(path: &str) -> Result<ParsedChartWithOd, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    parse_osu_to_bitmask_rows_str_with_od(&content)
}

pub fn parse_osu_to_bitmask_rows_str(content: &str) -> Result<ParsedChart, String> {
    let (keys, _od, rows, raw_notes, bg_file) = parse_osu_to_bitmask_rows_str_with_od(content)?;
    Ok((keys, rows, raw_notes, bg_file))
}

pub fn parse_osu_to_bitmask_rows_str_with_od(content: &str) -> Result<ParsedChartWithOd, String> {
    let mut mode: u32 = 0;
    let mut keys: u32 = 4;
    let mut od: f32 = 5.0;
    let mut in_events = false;
    let mut in_hit_objects = false;
    let mut bg_file = String::new();

    let mut timestamp_map: BTreeMap<u32, u32> = BTreeMap::new();
    let mut raw_notes: Vec<RawChartNote> = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with("Mode:") {
            if let Some(val) = line.split(':').nth(1) {
                if let Ok(m) = val.trim().parse::<u32>() {
                    mode = m;
                }
            }
        }

        if line.starts_with("CircleSize:") {
            if let Some(val) = line.split(':').nth(1) {
                if let Ok(k) = val.trim().parse::<f32>() {
                    keys = k.floor() as u32;
                }
            }
        }

        if line.starts_with("OverallDifficulty:") {
            if let Some(val) = line.split(':').nth(1) {
                if let Ok(parsed_od) = val.trim().parse::<f32>() {
                    od = parsed_od.clamp(0.0, 10.0);
                }
            }
        }

        if line == "[Events]" {
            in_events = true;
            in_hit_objects = false;
            continue;
        }

        if line == "[HitObjects]" {
            in_events = false;
            if mode != 3 {
                return Err("Please select an osu!mania map.".to_string());
            }
            if keys != 4 {
                return Err(format!(
                    "Only 4K maps are supported (This map is {}K).",
                    keys
                ));
            }
            in_hit_objects = true;
            continue;
        }

        if line.starts_with('[') {
            in_events = false;
        }

        if in_events && bg_file.is_empty() && (line.starts_with("0,0,") || line.starts_with("0,")) {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 3 {
                let candidate = parts[2].trim().trim_matches('"');
                let lower = candidate.to_lowercase();
                if lower.ends_with(".jpg")
                    || lower.ends_with(".jpeg")
                    || lower.ends_with(".png")
                    || lower.ends_with(".webp")
                {
                    bg_file = candidate.to_string();
                }
            }
        }

        if !in_hit_objects {
            continue;
        }

        if line.is_empty() || line.starts_with('[') {
            break;
        }

        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 4 {
            continue;
        }

        let x: f32 = parts[0].parse().unwrap_or(0.0);
        let time_ms: i64 = parts[2].parse().unwrap_or(0);
        let obj_type: u32 = parts[3].parse().unwrap_or(0);
        let tail_ms: i64 = if (obj_type & 128) != 0 && parts.len() >= 6 {
            parts[5]
                .split(':')
                .next()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(-1)
        } else {
            -1
        };

        let col = (x * keys as f32 / 512.0).floor() as u32;
        let col = col.min(keys.saturating_sub(1));
        let bit = 1u32 << col;

        let ms_u32 = time_ms.max(0) as u32;
        *timestamp_map.entry(ms_u32).or_insert(0) |= bit;
        raw_notes.push(RawChartNote {
            col: col as usize,
            head_ms: time_ms,
            tail_ms,
        });
    }

    let rows: Vec<(u32, f32)> = timestamp_map
        .into_iter()
        .map(|(ms, mask)| (mask, ms as f32 / 1000.0))
        .collect();

    Ok((keys, od, rows, raw_notes, bg_file))
}

pub fn classify_map(
    stream: f32,
    jumpstream: f32,
    handstream: f32,
    stamina: f32,
    jackspeed: f32,
    chordjack: f32,
    technical: f32,
) -> String {
    let skills = [
        ("Speed", stream),
        ("Jack", jackspeed.max(chordjack)),
        ("Stamina", stamina),
        ("Tech", technical.max(jumpstream).max(handstream)),
    ];

    match skills
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    {
        Some((name, _)) => format!("{} Map", name),
        None => "Unknown".to_string(),
    }
}

pub fn peek_cached_ratings(
    map_md5: &str,
    osu_folder: &str,
    osu_file: &str,
    rate: f32,
) -> Option<SkillRatings> {
    let key = if !map_md5.is_empty() {
        format!("{}||{:.2}", map_md5, rate)
    } else {
        format!("{}||{}||{:.2}", osu_folder, osu_file, rate)
    };
    if let Ok(cache) = CALC_CACHE.lock() {
        if let Some(ratings) = cache.get(&key) {
            if ratings.dan.tier > 0 && !ratings.dan.strain_curve.is_empty() {
                return Some(ratings.clone());
            }
        }
    }
    None
}

pub fn calculate_map_internal(
    songs_path: &str,
    osu_folder: &str,
    osu_file: &str,
    rate: f32,
    map_md5: &str,
) -> Result<SkillRatings, String> {
    calculate_map_with_direct(songs_path, "", "", osu_folder, osu_file, rate, map_md5)
}

pub fn calculate_map_with_direct(
    songs_path: &str,
    tosu_songs_path: &str,
    direct_beatmap_file: &str,
    osu_folder: &str,
    osu_file: &str,
    rate: f32,
    map_md5: &str,
) -> Result<SkillRatings, String> {
    let key = if !map_md5.is_empty() {
        format!("{}||{:.2}", map_md5, rate)
    } else {
        format!("{}||{}||{:.2}", osu_folder, osu_file, rate)
    };
    if let Ok(cache) = CALC_CACHE.lock() {
        if let Some(ratings) = cache.get(&key) {
            if ratings.dan.tier > 0 && !ratings.dan.strain_curve.is_empty() {
                return Ok(ratings.clone());
            }
        }
    }

    let mut full_path: Option<PathBuf> = None;
    let direct_clean = direct_beatmap_file.trim().replace('\\', "/");
    if !direct_clean.is_empty() {
        for base in [tosu_songs_path, songs_path] {
            if !base.trim().is_empty() {
                let candidate = Path::new(base.trim()).join(&direct_clean);
                if candidate.is_file() {
                    full_path = Some(candidate);
                    break;
                }
            }
        }
    }

    if full_path.is_none() {
        for base in [tosu_songs_path, songs_path] {
            if !base.trim().is_empty() {
                if let Some(found) = find_osu_file(base.trim(), osu_folder, osu_file) {
                    full_path = Some(found);
                    break;
                }
            }
        }
    }

    let full_path = match full_path {
        Some(p) => p,
        None => {
            return Err(format!(
                "File not found. folder='{}' file='{}'",
                osu_folder, osu_file
            ))
        }
    };

    let ratings = calculate_from_osu_file(&full_path, rate)?;

    if let Ok(mut cache) = CALC_CACHE.lock() {
        cache.insert(key, ratings.clone());
    }
    schedule_disk_cache_save();

    Ok(ratings)
}

pub fn store_cached_ratings(
    map_md5: &str,
    osu_folder: &str,
    osu_file: &str,
    rate: f32,
    ratings: &SkillRatings,
) {
    let key = if !map_md5.is_empty() {
        format!("{}||{:.2}", map_md5, rate)
    } else {
        format!("{}||{}||{:.2}", osu_folder, osu_file, rate)
    };
    if let Ok(mut cache) = CALC_CACHE.lock() {
        cache.insert(key, ratings.clone());
    }
    schedule_disk_cache_save();
}

pub fn calculate_from_osu_content(
    content: &str,
    rate: f32,
    parent_dir: Option<&Path>,
    bg_bytes: Option<&[u8]>,
) -> Result<SkillRatings, String> {
    let parsed = parse_osu_to_bitmask_rows_str_with_od(content)?;
    compute_skill_ratings(parsed, rate, parent_dir, bg_bytes)
}

pub fn calculate_from_osu_file(path: &Path, rate: f32) -> Result<SkillRatings, String> {
    let path_str = path.to_str().ok_or("Invalid path encoding")?;
    let parsed = parse_osu_to_bitmask_rows_with_od(path_str)?;
    compute_skill_ratings(parsed, rate, path.parent(), None)
}

fn compute_skill_ratings(
    chart: ParsedChartWithOd,
    rate: f32,
    parent_dir: Option<&Path>,
    bg_bytes: Option<&[u8]>,
) -> Result<SkillRatings, String> {
    let (keys, od, bitmask_rows, raw_notes, bg_file) = chart;

    if bitmask_rows.is_empty() {
        return Err("No notes found in this map (not a mania map?)".to_string());
    }

    let notes: Vec<Note> = bitmask_rows
        .into_iter()
        .map(|(notes, row_time)| Note { notes, row_time })
        .collect();

    let ssr_scores = with_calc(|calc| {
        calc.calc_at_rate(&notes, rate, 0.93, 4, CalcMode::Msd)
            .map_err(|e| e.to_string())
    })?;

    let map_type = classify_map(
        ssr_scores.stream,
        ssr_scores.jumpstream,
        ssr_scores.handstream,
        ssr_scores.stamina,
        ssr_scores.jackspeed,
        ssr_scores.chordjack,
        ssr_scores.technical,
    );

    let msd_arr = [
        ssr_scores.overall,
        ssr_scores.stream,
        ssr_scores.jumpstream,
        ssr_scores.handstream,
        ssr_scores.stamina,
        ssr_scores.jackspeed,
        ssr_scores.chordjack,
        ssr_scores.technical,
    ];
    let dan = calculate_combined_dan_with_od(
        &raw_notes,
        keys as usize,
        rate,
        od as f64,
        &msd_arr,
        "combined",
    );

    let mut bg_path = String::new();
    let mut adaptive_accent = String::new();
    let mut adaptive_accent_secondary = String::new();
    let mut adaptive_bg = String::new();
    let mut adaptive_item_bg = String::new();
    let mut adaptive_border = String::new();
    if !bg_file.is_empty() {
        if let Some(parent) = parent_dir {
            let candidate = parent.join(bg_file.replace('\\', "/"));
            if candidate.is_file() {
                bg_path = candidate.to_string_lossy().to_string();
                let (aa, asec, ab, aib, abd) = extract_adaptive_palette(&candidate);
                adaptive_accent = aa;
                adaptive_accent_secondary = asec;
                adaptive_bg = ab;
                adaptive_item_bg = aib;
                adaptive_border = abd;
            }
        }
    }
    if adaptive_accent.is_empty() {
        if let Some(bytes) = bg_bytes {
            let (aa, asec, ab, aib, abd) = extract_adaptive_palette_bytes(bytes);
            adaptive_accent = aa;
            adaptive_accent_secondary = asec;
            adaptive_bg = ab;
            adaptive_item_bg = aib;
            adaptive_border = abd;
        }
    }

    Ok(SkillRatings {
        overall: ssr_scores.overall,
        stream: ssr_scores.stream,
        jumpstream: ssr_scores.jumpstream,
        handstream: ssr_scores.handstream,
        stamina: ssr_scores.stamina,
        jackspeed: ssr_scores.jackspeed,
        chordjack: ssr_scores.chordjack,
        technical: ssr_scores.technical,
        map_type,
        dan,
        bg_path,
        adaptive_accent,
        adaptive_accent_secondary,
        adaptive_bg,
        adaptive_item_bg,
        adaptive_border,
    })
}
