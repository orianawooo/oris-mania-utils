use futures_util::StreamExt;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Condvar, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use crate::bootstrap::{detect_osu_songs_path, is_valid_tosu_root, target_radar_path};
use crate::calc::{
    calculate_from_osu_content, calculate_map_with_direct, extract_adaptive_palette,
    peek_cached_ratings, rebuild_songs_index, store_cached_ratings, SkillRatings,
};
use crate::config::{
    read_config, runtime_browser_overlays, runtime_hitcounter_poll_ms, runtime_tosu_host,
    runtime_tosu_port, write_config, CONFIG_REV,
};

#[derive(Clone, Default)]
pub struct LiveOsuState {
    pub song_title: String,
    pub song_artist: String,
    pub song_difficulty: String,
    pub bg_path: String,
    pub hits: [u32; 6],
    pub ratings: Option<SkillRatings>,
}

#[derive(Clone)]
struct PendingCalcJob {
    full_key: String,
    beatmap_key: String,
    map_key_event: String,
    osu_songs_path: String,
    tosu_songs: String,
    direct_file: String,
    folder: String,
    file: String,
    rate: f32,
    md5: String,
    title: String,
    artist: String,
    diff: String,
    bg_path: String,
    in_gameplay: bool,
}

lazy_static::lazy_static! {
    pub static ref LIVE_OSU_STATE: RwLock<LiveOsuState> = RwLock::new(LiveOsuState::default());
    pub static ref LAST_MSD_EVENT: RwLock<Option<String>> = RwLock::new(None);
    static ref COMPLETED_JOB_KEY: Mutex<String> = Mutex::new(String::new());
    static ref LAST_DISPATCHED_MAP_ID: Mutex<String> = Mutex::new(String::new());
    static ref PENDING_CALC: (Mutex<Option<PendingCalcJob>>, Condvar) = (Mutex::new(None), Condvar::new());
    static ref CACHED_SONGS_PATH: RwLock<(u64, String)> = RwLock::new((0, String::new()));
}

static CALC_WORKER_INIT: OnceLock<()> = OnceLock::new();
static BOOT_INSTANT: OnceLock<Instant> = OnceLock::new();
static LAST_WS_FRAME_MS: AtomicU64 = AtomicU64::new(0);
static LAST_HTTP_POLL_MS: AtomicU64 = AtomicU64::new(0);
static LAST_HTTP_HIT_CHANGE_MS: AtomicU64 = AtomicU64::new(0);
pub static LIVE_RATINGS_SEQ: AtomicU64 = AtomicU64::new(0);
pub static LIVE_HITS_SEQ: AtomicU64 = AtomicU64::new(0);
pub static LIVE_OSU_STATE_NUM: AtomicI64 = AtomicI64::new(-1);
pub static LIVE_SONG_TIME_MS: AtomicI64 = AtomicI64::new(0);
pub static TOSU_CONNECTED: AtomicBool = AtomicBool::new(false);

lazy_static::lazy_static! {
    pub static ref OVERLAY_WAKE: (Mutex<u64>, Condvar) = (Mutex::new(0), Condvar::new());
}

fn elapsed_ms() -> u64 {
    BOOT_INSTANT.get_or_init(Instant::now).elapsed().as_millis() as u64
}

pub fn wake_overlay_thread() {
    let (lock, cvar) = &*OVERLAY_WAKE;
    if let Ok(mut g) = lock.lock() {
        *g = g.wrapping_add(1);
        cvar.notify_all();
    }
}

fn fast_songs_path() -> String {
    let rev = CONFIG_REV.load(Ordering::Relaxed);
    if let Ok(guard) = CACHED_SONGS_PATH.read() {
        if guard.0 == rev && !guard.1.is_empty() {
            return guard.1.clone();
        }
    }
    let path = read_config().osu_songs_path;
    if let Ok(mut guard) = CACHED_SONGS_PATH.write() {
        *guard = (rev, path.clone());
    }
    path
}

pub fn get_overlay_path() -> PathBuf {
    let config = read_config();
    if !config.tosu_root_path.is_empty() {
        let root = PathBuf::from(&config.tosu_root_path);
        if is_valid_tosu_root(&root) {
            return target_radar_path(&root);
        }
    }

    dirs_next::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("oris-mania-utils")
        .join("runtime-preview")
        .join("radar")
}

async fn connect_ws_preferred(
    port: u16,
) -> Option<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
> {
    let host = runtime_tosu_host();
    for endpoint in ["/websocket/v2", "/ws"] {
        let url = format!("ws://{}:{}{}", host, port, endpoint);
        let mut request = match url.into_client_request() {
            Ok(req) => req,
            Err(_) => continue,
        };
        if let Ok(origin) = format!("http://{}:{}", host, port).parse() {
            request.headers_mut().insert("Origin", origin);
        }
        if let Ok((ws_stream, _)) = tokio_tungstenite::connect_async(request).await {
            return Some(ws_stream);
        }
    }
    None
}

fn zero_ratings(label: &str) -> SkillRatings {
    SkillRatings {
        overall: 0.0,
        stream: 0.0,
        jumpstream: 0.0,
        handstream: 0.0,
        stamina: 0.0,
        jackspeed: 0.0,
        chordjack: 0.0,
        technical: 0.0,
        map_type: label.to_string(),
        dan: Default::default(),
        bg_path: String::new(),
        adaptive_accent: String::new(),
        adaptive_accent_secondary: String::new(),
        adaptive_bg: String::new(),
        adaptive_item_bg: String::new(),
        adaptive_border: String::new(),
    }
}

struct RatingMeta<'a> {
    full_key: &'a str,
    beatmap_key: &'a str,
    map_key_event: &'a str,
    rate: f32,
    title: &'a str,
    artist: &'a str,
    diff: &'a str,
    bg_path: &'a str,
}

fn publish_ratings(meta: &RatingMeta<'_>, mut ratings: SkillRatings) {
    let full_key = meta.full_key;
    let beatmap_key = meta.beatmap_key;
    let map_key_event = meta.map_key_event;
    let rate = meta.rate;
    let title = meta.title;
    let artist = meta.artist;
    let diff = meta.diff;
    let bg_path = meta.bg_path;
    if !bg_path.is_empty() && (ratings.bg_path.is_empty() || ratings.bg_path != bg_path) {
        let p = Path::new(bg_path);
        if p.is_file() {
            ratings.bg_path = bg_path.to_string();
            if ratings.adaptive_accent.is_empty() {
                let (aa, asec, ab, aib, abd) = extract_adaptive_palette(p);
                ratings.adaptive_accent = aa;
                ratings.adaptive_accent_secondary = asec;
                ratings.adaptive_bg = ab;
                ratings.adaptive_item_bg = aib;
                ratings.adaptive_border = abd;
            }
        }
    } else if ratings.adaptive_accent.is_empty() && !ratings.bg_path.is_empty() {
        let p = Path::new(&ratings.bg_path);
        if p.is_file() {
            let (aa, asec, ab, aib, abd) = extract_adaptive_palette(p);
            ratings.adaptive_accent = aa;
            ratings.adaptive_accent_secondary = asec;
            ratings.adaptive_bg = ab;
            ratings.adaptive_item_bg = aib;
            ratings.adaptive_border = abd;
        }
    }

    if let Ok(mut done) = COMPLETED_JOB_KEY.lock() {
        *done = full_key.to_string();
    }
    if let Ok(mut live) = LIVE_OSU_STATE.write() {
        live.bg_path = ratings.bg_path.clone();
        live.ratings = Some(ratings.clone());
    }
    LIVE_RATINGS_SEQ.fetch_add(1, Ordering::Relaxed);
    wake_overlay_thread();

    if !runtime_browser_overlays() && crate::KEY_SENDER.receiver_count() == 0 {
        return;
    }

    let overlay_dir = get_overlay_path();
    let _ = std::fs::create_dir_all(&overlay_dir);
    let msd_path = overlay_dir.join("msd.json");
    let output = serde_json::json!({
        "map_key": map_key_event,
        "beatmap_key": beatmap_key,
        "rate": rate,
        "ratings": &ratings
    });
    if let Ok(json_str) = serde_json::to_string(&output) {
        let _ = std::fs::write(&msd_path, json_str);
    }
    let ws_event = serde_json::json!({
        "event": "msd-updated",
        "map_key": map_key_event,
        "beatmap_key": beatmap_key,
        "rate": rate,
        "song_title": title,
        "song_artist": artist,
        "song_difficulty": diff,
        "ratings": &ratings
    });
    if let Ok(ws_str) = serde_json::to_string(&ws_event) {
        if let Ok(mut last_ev) = LAST_MSD_EVENT.write() {
            *last_ev = Some(ws_str.clone());
        }
        let _ = crate::KEY_SENDER.send(ws_str);
    }
}

fn fetch_map_and_calc_via_tosu_http(
    rt: &tokio::runtime::Runtime,
    client: &reqwest::Client,
    job: &PendingCalcJob,
) -> Result<SkillRatings, String> {
    let host = runtime_tosu_host();
    let port = runtime_tosu_port();
    let osu_url = format!("http://{}:{}/files/beatmap/file", host, port);
    let bg_url = format!("http://{}:{}/files/beatmap/background", host, port);

    rt.block_on(async {
        let resp = client
            .get(&osu_url)
            .send()
            .await
            .map_err(|e| format!("tosu beatmap fetch failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("tosu beatmap file returned {}", resp.status()));
        }
        let content = resp.text().await.map_err(|e| e.to_string())?;
        let bg_bytes = if let Ok(bg_resp) = client.get(&bg_url).send().await {
            if bg_resp.status().is_success() {
                bg_resp.bytes().await.ok().map(|b| b.to_vec())
            } else {
                None
            }
        } else {
            None
        };
        let mut ratings =
            calculate_from_osu_content(&content, job.rate, None, bg_bytes.as_deref())?;
        if !job.bg_path.is_empty() && ratings.bg_path.is_empty() {
            ratings.bg_path = job.bg_path.clone();
        }
        store_cached_ratings(&job.md5, &job.folder, &job.file, job.rate, &ratings);
        Ok(ratings)
    })
}

fn ensure_calc_worker() {
    CALC_WORKER_INIT.get_or_init(|| {
        std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .ok();
            let client = reqwest::Client::builder()
                .timeout(Duration::from_millis(1200))
                .build()
                .unwrap_or_default();

            loop {
                let job = {
                    let (lock, cvar) = &*PENDING_CALC;
                    let mut guard = lock.lock().unwrap();
                    while guard.is_none() {
                        guard = cvar.wait(guard).unwrap();
                    }
                    guard.take().unwrap()
                };

                if !job.in_gameplay {
                    std::thread::sleep(Duration::from_millis(45));
                    let (lock, _) = &*PENDING_CALC;
                    if let Ok(guard) = lock.lock() {
                        if guard.is_some() {
                            continue;
                        }
                    }
                }

                let mut calc_res = calculate_map_with_direct(
                    &job.osu_songs_path,
                    &job.tosu_songs,
                    &job.direct_file,
                    &job.folder,
                    &job.file,
                    job.rate,
                    &job.md5,
                );

                if let Err(ref e) = calc_res {
                    let is_permanent = e.contains("mania map") || e.contains("Only 4K");
                    if !is_permanent {
                        if let Some(ref rt_handle) = rt {
                            if let Ok(http_ratings) =
                                fetch_map_and_calc_via_tosu_http(rt_handle, &client, &job)
                            {
                                calc_res = Ok(http_ratings);
                            }
                        }
                    }
                }

                {
                    let (lock, _) = &*PENDING_CALC;
                    if let Ok(guard) = lock.lock() {
                        if let Some(ref newer) = *guard {
                            if newer.full_key != job.full_key {
                                continue;
                            }
                        }
                    }
                }

                match calc_res {
                    Ok(ratings) => {
                        let meta = RatingMeta {
                            full_key: &job.full_key,
                            beatmap_key: &job.beatmap_key,
                            map_key_event: &job.map_key_event,
                            rate: job.rate,
                            title: &job.title,
                            artist: &job.artist,
                            diff: &job.diff,
                            bg_path: &job.bg_path,
                        };
                        publish_ratings(&meta, ratings);
                    }
                    Err(e) => {
                        let permanent = e.contains("mania map") || e.contains("Only 4K");
                        if permanent {
                            let tag = if e.contains("Only 4K") {
                                "non-4k"
                            } else {
                                "non-mania"
                            };
                            let meta = RatingMeta {
                                full_key: &job.full_key,
                                beatmap_key: &job.beatmap_key,
                                map_key_event: &job.map_key_event,
                                rate: job.rate,
                                title: &job.title,
                                artist: &job.artist,
                                diff: &job.diff,
                                bg_path: &job.bg_path,
                            };
                            publish_ratings(&meta, zero_ratings(tag));
                        } else if let Ok(mut last_id) = LAST_DISPATCHED_MAP_ID.lock() {
                            if *last_id == job.full_key {
                                last_id.clear();
                            }
                        }
                    }
                }
            }
        });
    });
}

fn resolve_beatmap_bg_path(
    tosu_songs: &str,
    cfg_songs: &str,
    direct_bg: &str,
    folder: &str,
    bg_file: &str,
) -> String {
    let clean_direct = direct_bg
        .trim()
        .trim_start_matches(['/', '\\'])
        .replace('\\', "/");
    let clean_folder = folder.trim().replace('\\', "/");
    let clean_bg = bg_file.trim().replace('\\', "/");

    for base in [tosu_songs, cfg_songs] {
        let b = base.trim();
        if b.is_empty() {
            continue;
        }
        let base_path = Path::new(b);
        if !clean_direct.is_empty() {
            let cand = base_path.join(&clean_direct);
            if cand.is_file() {
                return cand.to_string_lossy().to_string();
            }
        }
        if !clean_folder.is_empty() && !clean_bg.is_empty() {
            let cand = base_path.join(&clean_folder).join(&clean_bg);
            if cand.is_file() {
                return cand.to_string_lossy().to_string();
            }
        }
    }
    String::new()
}

fn opt_str_null<'de, D>(de: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(de)?.unwrap_or_default())
}

fn opt_i64_null<'de, D>(de: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumOrStr {
        I(i64),
        F(f64),
        S(String),
    }
    match Option::<NumOrStr>::deserialize(de)? {
        Some(NumOrStr::I(v)) => Ok(Some(v)),
        Some(NumOrStr::F(v)) => Ok(Some(v as i64)),
        Some(NumOrStr::S(s)) => Ok(s.trim().parse::<i64>().ok()),
        None => Ok(None),
    }
}

fn opt_f32_null<'de, D>(de: D) -> Result<Option<f32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<f64>::deserialize(de)?.map(|v| v as f32))
}

fn opt_u32_null<'de, D>(de: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<f64>::deserialize(de)?
        .map(|v| v.max(0.0) as u32)
        .unwrap_or(0))
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TosuPayload {
    state: TosuStateNum,
    beatmap: TosuBeatmapV2,
    folders: TosuFoldersV2,
    files: TosuFilesV2,
    #[serde(rename = "directPath")]
    direct_path: TosuDirectPathV2,
    play: TosuPlayV2,
    menu: TosuMenuLegacy,
    gameplay: TosuGameplayLegacy,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TosuStateNum {
    #[serde(deserialize_with = "opt_i64_null")]
    number: Option<i64>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TosuBeatmapPathLegacy {
    #[serde(deserialize_with = "opt_str_null")]
    folder: String,
    #[serde(deserialize_with = "opt_str_null")]
    file: String,
    #[serde(deserialize_with = "opt_str_null")]
    full: String,
    #[serde(deserialize_with = "opt_str_null")]
    bg: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TosuTimeV2 {
    #[serde(deserialize_with = "opt_i64_null")]
    live: Option<i64>,
    #[serde(deserialize_with = "opt_i64_null")]
    current: Option<i64>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TosuBeatmapV2 {
    #[serde(deserialize_with = "opt_i64_null")]
    id: Option<i64>,
    #[serde(deserialize_with = "opt_str_null")]
    checksum: String,
    #[serde(deserialize_with = "opt_str_null")]
    md5: String,
    #[serde(deserialize_with = "opt_str_null")]
    title: String,
    #[serde(rename = "titleUnicode", deserialize_with = "opt_str_null")]
    title_unicode: String,
    #[serde(deserialize_with = "opt_str_null")]
    artist: String,
    #[serde(rename = "artistUnicode", deserialize_with = "opt_str_null")]
    artist_unicode: String,
    #[serde(deserialize_with = "opt_str_null")]
    version: String,
    #[serde(deserialize_with = "opt_str_null")]
    difficulty: String,
    #[serde(deserialize_with = "opt_str_null")]
    mods: String,
    path: TosuBeatmapPathLegacy,
    time: TosuTimeV2,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TosuFoldersV2 {
    #[serde(deserialize_with = "opt_str_null")]
    beatmap: String,
    #[serde(deserialize_with = "opt_str_null")]
    songs: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TosuFilesV2 {
    #[serde(deserialize_with = "opt_str_null")]
    beatmap: String,
    #[serde(deserialize_with = "opt_str_null")]
    background: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TosuDirectPathV2 {
    #[serde(rename = "beatmapFolder", deserialize_with = "opt_str_null")]
    beatmap_folder: String,
    #[serde(rename = "beatmapFile", deserialize_with = "opt_str_null")]
    beatmap_file: String,
    #[serde(rename = "beatmapBackground", deserialize_with = "opt_str_null")]
    beatmap_background: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TosuModsV2 {
    #[serde(deserialize_with = "opt_str_null")]
    name: String,
    #[serde(rename = "str", deserialize_with = "opt_str_null")]
    legacy_str: String,
    #[serde(deserialize_with = "opt_f32_null")]
    rate: Option<f32>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TosuHits {
    #[serde(deserialize_with = "opt_u32_null")]
    geki: u32,
    #[serde(rename = "300", deserialize_with = "opt_u32_null")]
    n300: u32,
    #[serde(deserialize_with = "opt_u32_null")]
    katu: u32,
    #[serde(rename = "100", deserialize_with = "opt_u32_null")]
    n100: u32,
    #[serde(rename = "50", deserialize_with = "opt_u32_null")]
    n50: u32,
    #[serde(rename = "0", deserialize_with = "opt_u32_null")]
    n0: u32,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TosuPlayV2 {
    mods: TosuModsV2,
    hits: Option<TosuHits>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TosuMetadataLegacy {
    #[serde(deserialize_with = "opt_str_null")]
    title: String,
    #[serde(rename = "titleUnicode", deserialize_with = "opt_str_null")]
    title_unicode: String,
    #[serde(deserialize_with = "opt_str_null")]
    artist: String,
    #[serde(rename = "artistUnicode", deserialize_with = "opt_str_null")]
    artist_unicode: String,
    #[serde(deserialize_with = "opt_str_null")]
    difficulty: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TosuBmLegacy {
    #[serde(deserialize_with = "opt_i64_null")]
    id: Option<i64>,
    #[serde(deserialize_with = "opt_str_null")]
    md5: String,
    path: TosuBeatmapPathLegacy,
    metadata: TosuMetadataLegacy,
    time: TosuTimeV2,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TosuMenuLegacy {
    #[serde(deserialize_with = "opt_i64_null")]
    state: Option<i64>,
    bm: TosuBmLegacy,
    mods: TosuModsV2,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TosuGameplayLegacy {
    hits: Option<TosuHits>,
}

fn first_non_empty(opts: &[&str]) -> String {
    for &s in opts {
        if !s.is_empty() {
            return s.to_string();
        }
    }
    String::new()
}

pub fn process_tosu_payload(payload: &str) {
    process_tosu_payload_with_source(payload, false);
}

fn process_tosu_payload_with_source(payload: &str, from_http: bool) {
    let data: TosuPayload = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return,
    };

    let folder = first_non_empty(&[
        &data.folders.beatmap,
        &data.direct_path.beatmap_folder,
        &data.menu.bm.path.folder,
        &data.beatmap.path.folder,
    ]);
    let file = first_non_empty(&[
        &data.files.beatmap,
        &data.menu.bm.path.file,
        &data.beatmap.path.file,
    ]);
    let direct_file = data.direct_path.beatmap_file;
    let direct_bg = first_non_empty(&[
        &data.direct_path.beatmap_background,
        &data.menu.bm.path.full,
    ]);
    let bg_file = first_non_empty(&[&data.files.background, &data.menu.bm.path.bg]);
    let tosu_songs = data.folders.songs;

    let md5 = first_non_empty(&[&data.beatmap.checksum, &data.beatmap.md5, &data.menu.bm.md5]);
    let id = data
        .beatmap
        .id
        .or(data.menu.bm.id)
        .filter(|&n| n > 0)
        .map(|n| n.to_string())
        .unwrap_or_default();

    let title = first_non_empty(&[
        &data.beatmap.title,
        &data.beatmap.title_unicode,
        &data.menu.bm.metadata.title,
        &data.menu.bm.metadata.title_unicode,
    ]);
    let artist = first_non_empty(&[
        &data.beatmap.artist,
        &data.beatmap.artist_unicode,
        &data.menu.bm.metadata.artist,
        &data.menu.bm.metadata.artist_unicode,
    ]);
    let diff = first_non_empty(&[
        &data.beatmap.version,
        &data.beatmap.difficulty,
        &data.menu.bm.metadata.difficulty,
    ]);

    let mods_str = first_non_empty(&[
        &data.play.mods.name,
        &data.menu.mods.legacy_str,
        &data.beatmap.mods,
    ]);
    let v2_rate = data.play.mods.rate.unwrap_or(0.0);

    let state_num = data.state.number.or(data.menu.state).unwrap_or(-1);
    LIVE_OSU_STATE_NUM.store(state_num, Ordering::Relaxed);

    let live_time = data
        .beatmap
        .time
        .live
        .or(data.menu.bm.time.current)
        .unwrap_or(0);
    LIVE_SONG_TIME_MS.store(live_time.max(0), Ordering::Relaxed);

    let in_play_or_results = state_num == 2 || state_num == 7;
    let hits = if state_num >= 0 && !in_play_or_results {
        [0; 6]
    } else if let Some(h) = data.play.hits.or(data.gameplay.hits) {
        [h.geki, h.n300, h.katu, h.n100, h.n50, h.n0]
    } else {
        [0; 6]
    };

    let mut cfg_songs = fast_songs_path();
    if cfg_songs.is_empty() && !tosu_songs.is_empty() {
        cfg_songs = tosu_songs.clone();
    }
    let resolved_bg =
        resolve_beatmap_bg_path(&tosu_songs, &cfg_songs, &direct_bg, &folder, &bg_file);

    let mut hits_changed = false;
    if let Ok(mut live) = LIVE_OSU_STATE.write() {
        if !title.is_empty() {
            if live.song_title != title {
                live.song_title = title.clone();
            }
            if live.song_artist != artist {
                live.song_artist = artist.clone();
            }
            if live.song_difficulty != diff {
                live.song_difficulty = diff.clone();
            }
        }
        if !resolved_bg.is_empty() && live.bg_path != resolved_bg {
            live.bg_path = resolved_bg.clone();
            LIVE_RATINGS_SEQ.fetch_add(1, Ordering::Relaxed);
            wake_overlay_thread();
        }
        let now_ms = elapsed_ms();
        let new_sum: u32 = hits.iter().sum();
        let cur_sum: u32 = live.hits.iter().sum();
        let last_fast = LAST_HTTP_HIT_CHANGE_MS.load(Ordering::Relaxed);
        let stale_ws = !from_http
            && state_num == 2
            && new_sum > 0
            && new_sum < cur_sum
            && now_ms.saturating_sub(last_fast) < 180;
        if !stale_ws && live.hits != hits {
            live.hits = hits;
            hits_changed = true;
            if from_http {
                LAST_HTTP_HIT_CHANGE_MS.store(now_ms, Ordering::Relaxed);
            }
            LIVE_HITS_SEQ.fetch_add(1, Ordering::Relaxed);
        }
    }

    if hits_changed {
        wake_overlay_thread();
        if runtime_browser_overlays() && crate::KEY_SENDER.receiver_count() > 0 {
            let ev = serde_json::json!({
                "event": "hits-updated",
                "hits": hits,
                "state": state_num
            });
            if let Ok(s) = serde_json::to_string(&ev) {
                let _ = crate::KEY_SENDER.send(s);
            }
        }
    }

    let use_key = if !md5.is_empty() {
        md5.clone()
    } else if !id.is_empty() {
        id
    } else {
        String::new()
    };

    let rate = if v2_rate > 0.1 && (v2_rate - 1.0).abs() > 0.01 {
        v2_rate
    } else {
        let mods_upper = mods_str.to_uppercase();
        if mods_upper.contains("DT") || mods_upper.contains("NC") {
            1.5
        } else if mods_upper.contains("HT") {
            0.75
        } else {
            1.0
        }
    };

    let full_key = format!("{}||{:.2}", use_key, rate);

    if use_key.is_empty() || use_key == "0" {
        return;
    }

    let is_done = COMPLETED_JOB_KEY
        .lock()
        .map(|g| *g == full_key)
        .unwrap_or(false);
    if is_done {
        return;
    }

    let map_key_event = format!("{}||{}", use_key, mods_str);

    if let Some(cached) = peek_cached_ratings(&md5, &folder, &file, rate) {
        if let Ok(mut last_id) = LAST_DISPATCHED_MAP_ID.lock() {
            *last_id = full_key.clone();
        }
        let meta = RatingMeta {
            full_key: &full_key,
            beatmap_key: &use_key,
            map_key_event: &map_key_event,
            rate,
            title: &title,
            artist: &artist,
            diff: &diff,
            bg_path: &resolved_bg,
        };
        publish_ratings(&meta, cached);
        return;
    }

    if let Ok(mut last_id) = LAST_DISPATCHED_MAP_ID.lock() {
        if *last_id != full_key {
            *last_id = full_key.clone();
            if let Ok(mut done) = COMPLETED_JOB_KEY.lock() {
                done.clear();
            }
            if let Ok(mut live) = LIVE_OSU_STATE.write() {
                live.bg_path = resolved_bg.clone();
                live.ratings = Some(zero_ratings("..."));
            }
            LIVE_RATINGS_SEQ.fetch_add(1, Ordering::Relaxed);
            wake_overlay_thread();
        }
    }

    ensure_calc_worker();
    let (lock, cvar) = &*PENDING_CALC;
    if let Ok(mut guard) = lock.lock() {
        *guard = Some(PendingCalcJob {
            full_key,
            beatmap_key: use_key,
            map_key_event,
            osu_songs_path: cfg_songs,
            tosu_songs,
            direct_file,
            folder,
            file,
            rate,
            md5,
            title,
            artist,
            diff,
            bg_path: resolved_bg,
            in_gameplay: state_num == 2,
        });
        cvar.notify_one();
    }
}

pub async fn start_tosu_proxy() {
    tokio::task::spawn_blocking(|| {
        let mut config = read_config();
        if config.osu_songs_path.is_empty() {
            if let Some(detected) = detect_osu_songs_path() {
                config.osu_songs_path = detected.to_string_lossy().to_string();
                write_config(&config);
            }
        }
        if !config.osu_songs_path.is_empty() {
            rebuild_songs_index(&config.osu_songs_path);
        }
        if !config.tosu_root_path.is_empty() {
            crate::tosu_manager::ensure_tosu_overlay_env(Path::new(&config.tosu_root_path));
        }
    });

    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(120))
            .pool_max_idle_per_host(2)
            .build()
            .unwrap_or_default();

        loop {
            let poll_ms = runtime_hitcounter_poll_ms() as u64;
            let state_num = LIVE_OSU_STATE_NUM.load(Ordering::Relaxed);
            let sleep_step = if state_num == 2 {
                (poll_ms / 2).clamp(4, 16)
            } else {
                16
            };
            tokio::time::sleep(Duration::from_millis(sleep_step)).await;

            let connected = TOSU_CONNECTED.load(Ordering::Relaxed);
            let triggered = crate::HIT_POLL_TRIGGER.swap(false, Ordering::AcqRel);
            let state_num = LIVE_OSU_STATE_NUM.load(Ordering::Relaxed);
            let now_ms = elapsed_ms();
            let last_ws = LAST_WS_FRAME_MS.load(Ordering::Relaxed);
            let since_ws = now_ms.saturating_sub(last_ws);
            let last_poll = LAST_HTTP_POLL_MS.load(Ordering::Relaxed);
            let since_poll = now_ms.saturating_sub(last_poll);

            let ws_healthy = connected && last_ws > 0 && since_ws < 180;
            let interval_ms = if !connected {
                400
            } else if state_num == 2 {
                poll_ms
            } else if ws_healthy {
                360
            } else {
                100
            };

            let trigger_now =
                connected && state_num == 2 && triggered && since_poll >= poll_ms.min(16);
            if !trigger_now && since_poll < interval_ms {
                continue;
            }
            LAST_HTTP_POLL_MS.store(now_ms, Ordering::Relaxed);

            let host = runtime_tosu_host();
            let port = runtime_tosu_port();
            let url = format!("http://{}:{}/json/v2", host, port);
            if let Ok(resp) = client.get(&url).send().await {
                if resp.status().is_success() {
                    TOSU_CONNECTED.store(true, Ordering::SeqCst);
                    if let Ok(body) = resp.text().await {
                        process_tosu_payload_with_source(&body, true);
                    }
                }
            }
        }
    });

    loop {
        let port = runtime_tosu_port();

        match connect_ws_preferred(port).await {
            Some(mut ws_stream) => {
                TOSU_CONNECTED.store(true, Ordering::SeqCst);
                LAST_WS_FRAME_MS.store(elapsed_ms(), Ordering::Relaxed);

                while let Some(message) = ws_stream.next().await {
                    match message {
                        Ok(msg) => {
                            if msg.is_text() || msg.is_binary() {
                                LAST_WS_FRAME_MS.store(elapsed_ms(), Ordering::Relaxed);
                                let payload = msg.into_text().unwrap_or_default();
                                process_tosu_payload(&payload);
                            }
                        }
                        Err(_) => {
                            TOSU_CONNECTED.store(false, Ordering::SeqCst);
                            break;
                        }
                    }
                }
                TOSU_CONNECTED.store(false, Ordering::SeqCst);
            }
            None => {
                TOSU_CONNECTED.store(false, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tosu_payload_handles_nulls_and_resets_hits_on_exit() {
        let play_json = r#"{
            "state": {"number": 2},
            "beatmap": {"id": 123, "checksum": null, "title": "Song A", "artist": "Art", "version": "Hard"},
            "play": {"hits": {"geki": 10, "300": 5, "katu": 2, "100": 1, "50": 0, "0": 1}, "mods": {"name": null, "rate": null}}
        }"#;
        process_tosu_payload(play_json);
        {
            let st = LIVE_OSU_STATE.read().unwrap();
            assert_eq!(st.song_title, "Song A");
            assert_eq!(st.song_difficulty, "Hard");
            assert_eq!(st.hits, [10, 5, 2, 1, 0, 1]);
        }

        let menu_json = r#"{
            "state": {"number": 5},
            "beatmap": {"id": 124, "title": "Song B", "artist": "Art2", "version": "Insane"},
            "play": {"hits": {"geki": 10, "300": 5, "katu": 2, "100": 1, "50": 0, "0": 1}}
        }"#;
        process_tosu_payload(menu_json);
        {
            let st = LIVE_OSU_STATE.read().unwrap();
            assert_eq!(st.song_title, "Song B");
            assert_eq!(st.song_difficulty, "Insane");
            assert_eq!(st.hits, [0, 0, 0, 0, 0, 0]);
        }
    }
}
