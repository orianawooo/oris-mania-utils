use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock};
use minacalc_rs::{Calc, CalcMode, Note};

#[derive(Clone, Serialize)]
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
}

lazy_static::lazy_static! {
    static ref CALC_CACHE: Mutex<HashMap<String, SkillRatings>> = Mutex::new(HashMap::new());
    static ref SONGS_INDEX: RwLock<HashMap<String, PathBuf>> = RwLock::new(HashMap::new());
}

pub fn rebuild_songs_index(songs_path: &str) {
    let songs_dir = Path::new(songs_path);
    if !songs_dir.is_dir() {
        return;
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
}

pub fn find_osu_file(songs_path: &str, folder_or_id: &str, file: &str) -> Option<PathBuf> {
    let folder_trimmed = folder_or_id.trim();
    let file_trimmed = file.trim();
    let songs_dir = Path::new(songs_path);

    let find_in_dir = |dir: &Path| -> Option<PathBuf> {
        let direct = dir.join(file_trimmed);
        if direct.exists() {
            return Some(direct);
        }
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                let name_clean = name_str.trim();
                let file_clean = file_trimmed.trim_start_matches([' ', '-']).to_lowercase();
                let name_c = name_clean.trim_start_matches([' ', '-']).to_lowercase();
                if name_clean.to_lowercase() == file_trimmed.to_lowercase()
                    || name_c == file_clean
                    || name_c.starts_with(&file_clean[..file_clean.len().min(20)])
                {
                    let candidate = dir.join(name_str.as_ref());
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
        }
        None
    };

    let direct_folder = songs_dir.join(folder_trimmed);
    if direct_folder.exists() {
        if let Some(p) = find_in_dir(&direct_folder) {
            return Some(p);
        }
    }

    let folder_lower = folder_trimmed.to_lowercase();
    if let Ok(index) = SONGS_INDEX.read() {
        if let Some(dir_path) = index.get(&folder_lower) {
            if let Some(p) = find_in_dir(dir_path) {
                return Some(p);
            }
        }
        for (key, dir_path) in index.iter() {
            if key.starts_with(&folder_lower) || key == &folder_lower {
                if let Some(p) = find_in_dir(dir_path) {
                    return Some(p);
                }
            }
        }
    }

    if let Ok(entries) = fs::read_dir(songs_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.trim().starts_with(folder_trimmed)
                || name_str.trim() == folder_trimmed
            {
                if let Some(p) = find_in_dir(&entry.path()) {
                    return Some(p);
                }
            }
        }
    }

    None
}

pub fn parse_osu_to_bitmask_rows(path: &str) -> Result<(u32, Vec<(u32, f32)>), String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut mode: u32 = 0;
    let mut keys: u32 = 4;
    let mut in_hit_objects = false;

    let mut timestamp_map: BTreeMap<u32, u32> = BTreeMap::new();

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

        if line == "[HitObjects]" {
            if mode != 3 {
                return Err("Please select an osu!mania map.".to_string());
            }
            if keys != 4 {
                return Err(format!("Only 4K maps are supported (This map is {}K).", keys));
            }
            in_hit_objects = true;
            continue;
        }

        if !in_hit_objects {
            continue;
        }

        if line.is_empty() || line.starts_with('[') {
            break;
        }

        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 3 {
            continue;
        }

        let x: f32 = parts[0].parse().unwrap_or(0.0);
        let time_ms: u32 = parts[2].parse().unwrap_or(0);
        let col = (x * keys as f32 / 512.0).floor() as u32;
        let col = col.min(keys.saturating_sub(1));
        let bit = 1u32 << col;

        *timestamp_map.entry(time_ms).or_insert(0) |= bit;
    }

    let rows: Vec<(u32, f32)> = timestamp_map
        .into_iter()
        .map(|(ms, mask)| (mask, ms as f32 / 1000.0))
        .collect();

    Ok((keys, rows))
}

pub fn classify_map(stream: f32, jumpstream: f32, handstream: f32, stamina: f32, jackspeed: f32, chordjack: f32, technical: f32) -> String {
    let skills = [
        ("Speed", stream),
        ("Jack", jackspeed.max(chordjack)),
        ("Stamina", stamina),
        ("Tech", technical.max(jumpstream).max(handstream)),
    ];

    match skills.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)) {
        Some((name, _)) => format!("{} Map", name),
        None => "Unknown".to_string(),
    }
}

pub fn calculate_map_internal(songs_path: &str, osu_folder: &str, osu_file: &str, rate: f32, map_md5: &str) -> Result<SkillRatings, String> {
    let key = if !map_md5.is_empty() {
        format!("{}||{}", map_md5, rate)
    } else {
        format!("{}||{}||{}", osu_folder, osu_file, rate)
    };
    {
        if let Ok(cache) = CALC_CACHE.lock() {
            if let Some(ratings) = cache.get(&key) {
                return Ok(ratings.clone());
            }
        }
    }

    let full_path = match find_osu_file(songs_path, osu_folder, osu_file) {
        Some(p) => p,
        None => return Err(format!("File not found. folder='{}' file='{}'", osu_folder, osu_file)),
    };

    let path_str = full_path.to_str().ok_or("Invalid path encoding")?;
    let (_keys, bitmask_rows) = parse_osu_to_bitmask_rows(path_str)?;

    if bitmask_rows.is_empty() {
        return Err("No notes found in this map (not a mania map?)".to_string());
    }

    let calc = Calc::new().map_err(|e| e.to_string())?;
    let notes: Vec<Note> = bitmask_rows.into_iter()
        .map(|(notes, row_time)| Note { notes, row_time })
        .collect();
    
    let ssr_scores = calc.calc_at_rate(&notes, rate, 0.93, 4, CalcMode::Msd)
        .map_err(|e| e.to_string())?;
    
    let map_type = classify_map(
        ssr_scores.stream,
        ssr_scores.jumpstream,
        ssr_scores.handstream,
        ssr_scores.stamina,
        ssr_scores.jackspeed,
        ssr_scores.chordjack,
        ssr_scores.technical,
    );

    let ratings = SkillRatings {
        overall: ssr_scores.overall,
        stream: ssr_scores.stream,
        jumpstream: ssr_scores.jumpstream,
        handstream: ssr_scores.handstream,
        stamina: ssr_scores.stamina,
        jackspeed: ssr_scores.jackspeed,
        chordjack: ssr_scores.chordjack,
        technical: ssr_scores.technical,
        map_type,
    };

    if let Ok(mut cache) = CALC_CACHE.lock() {
        cache.insert(key, ratings.clone());
    }

    Ok(ratings)
}
