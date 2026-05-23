use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tauri::{path::BaseDirectory, AppHandle, Manager};

use crate::config::{read_config, Config};

const VERSION_MARKER_FILE: &str = ".oris-mania-utils-version";
const OVERLAY_NAMES: [&str; 3] = ["msdconverter", "ManiaKeystrokes", "HitCounter"];

struct EmbeddedOverlayFile {
    path: &'static str,
    bytes: &'static [u8],
}

const EMBEDDED_OVERLAY_FILES: &[EmbeddedOverlayFile] = &[
    EmbeddedOverlayFile { path: "msdconverter/api.js", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/msdconverter/api.js")) },
    EmbeddedOverlayFile { path: "msdconverter/assets/javascript.svg", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/msdconverter/assets/javascript.svg")) },
    EmbeddedOverlayFile { path: "msdconverter/assets/tauri.svg", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/msdconverter/assets/tauri.svg")) },
    EmbeddedOverlayFile { path: "msdconverter/chart.js", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/msdconverter/chart.js")) },
    EmbeddedOverlayFile { path: "msdconverter/config.js", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/msdconverter/config.js")) },
    EmbeddedOverlayFile { path: "msdconverter/events.js", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/msdconverter/events.js")) },
    EmbeddedOverlayFile { path: "msdconverter/index.html", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/msdconverter/index.html")) },
    EmbeddedOverlayFile { path: "msdconverter/keystrokes.css", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/msdconverter/keystrokes.css")) },
    EmbeddedOverlayFile { path: "msdconverter/keystrokes.html", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/msdconverter/keystrokes.html")) },
    EmbeddedOverlayFile { path: "msdconverter/keystrokes.js", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/msdconverter/keystrokes.js")) },
    EmbeddedOverlayFile { path: "msdconverter/main.js", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/msdconverter/main.js")) },
    EmbeddedOverlayFile { path: "msdconverter/state.js", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/msdconverter/state.js")) },
    EmbeddedOverlayFile { path: "msdconverter/styles.css", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/msdconverter/styles.css")) },
    EmbeddedOverlayFile { path: "ManiaKeystrokes/index.html", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/ManiaKeystrokes/index.html")) },
    EmbeddedOverlayFile { path: "ManiaKeystrokes/keystrokes.css", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/ManiaKeystrokes/keystrokes.css")) },
    EmbeddedOverlayFile { path: "ManiaKeystrokes/keystrokes-geometry.js", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/ManiaKeystrokes/keystrokes-geometry.js")) },
    EmbeddedOverlayFile { path: "ManiaKeystrokes/keystrokes.js", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/ManiaKeystrokes/keystrokes.js")) },
    EmbeddedOverlayFile { path: "HitCounter/hitcounter-layout.js", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/HitCounter/hitcounter-layout.js")) },
    EmbeddedOverlayFile { path: "HitCounter/index.html", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/HitCounter/index.html")) },
    EmbeddedOverlayFile { path: "HitCounter/main.js", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/HitCounter/main.js")) },
    EmbeddedOverlayFile { path: "HitCounter/styles.css", bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../overlays/HitCounter/styles.css")) },
];

#[derive(serde::Serialize, Clone)]
pub struct BootstrapReport {
    pub config: Config,
    pub tosu_detected: bool,
    pub songs_detected: bool,
    pub overlays_installed: bool,
    pub overlay_statuses: Vec<OverlayStatus>,
    pub needs_tosu_folder: bool,
    pub needs_songs_folder: bool,
    pub notes: Vec<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct OverlayStatus {
    pub name: String,
    pub installed: bool,
    pub installed_version: Option<String>,
    pub update_available: bool,
}

#[derive(serde::Serialize, Clone)]
pub struct InstallReport {
    pub source_root: String,
    pub target_root: String,
    pub installed: bool,
    pub already_current: bool,
    pub copied_files: usize,
    pub installed_overlays: Vec<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct OverlayInstallProgress {
    pub overlay: String,
    pub file: String,
    pub overlay_done: usize,
    pub overlay_total: usize,
    pub overall_done: usize,
    pub overall_total: usize,
    pub message: String,
}

fn path_exists(path: &Path) -> bool {
    path.exists() && path.is_dir()
}

fn overlay_required_files(overlay_name: &str) -> &'static [&'static str] {
    match overlay_name {
        "msdconverter" => &["index.html", "main.js", "styles.css"],
        "ManiaKeystrokes" => &["index.html", "keystrokes.js", "keystrokes.css", "keystrokes-geometry.js"],
        "HitCounter" => &["index.html", "main.js", "styles.css", "hitcounter-layout.js"],
        _ => &[],
    }
}

fn overlay_source_files(overlay_name: &str) -> &'static [&'static str] {
    match overlay_name {
        "msdconverter" => &[
            "api.js",
            "assets/javascript.svg",
            "assets/tauri.svg",
            "chart.js",
            "config.js",
            "events.js",
            "index.html",
            "keystrokes.css",
            "keystrokes.html",
            "keystrokes.js",
            "main.js",
            "state.js",
            "styles.css",
        ],
        "ManiaKeystrokes" => &[
            "index.html",
            "keystrokes.css",
            "keystrokes-geometry.js",
            "keystrokes.js",
        ],
        "HitCounter" => &[
            "hitcounter-layout.js",
            "index.html",
            "main.js",
            "styles.css",
        ],
        _ => &[],
    }
}

fn is_valid_overlay_source_root(path: &Path) -> bool {
    OVERLAY_NAMES.iter().all(|overlay_name| {
        overlay_source_files(overlay_name)
            .iter()
            .all(|file_name| path.join(overlay_name).join(file_name).exists())
    })
}

fn is_valid_overlay_install_root(path: &Path) -> bool {
    path.join("msdconverter").join("index.html").exists()
        && path.join("msdconverter").join("main.js").exists()
        && path.join("msdconverter").join("styles.css").exists()
        && path.join("ManiaKeystrokes").join("index.html").exists()
        && path.join("ManiaKeystrokes").join("keystrokes.js").exists()
        && path.join("ManiaKeystrokes").join("keystrokes.css").exists()
        && path.join("ManiaKeystrokes").join("keystrokes-geometry.js").exists()
        && path.join("HitCounter").join("index.html").exists()
        && path.join("HitCounter").join("main.js").exists()
        && path.join("HitCounter").join("styles.css").exists()
        && path.join("HitCounter").join("hitcounter-layout.js").exists()
}

pub fn is_valid_osu_songs_path(path: &Path) -> bool {
    if !path.exists() || !path.is_dir() {
        return false;
    }

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                if let Ok(sub_entries) = fs::read_dir(entry_path) {
                    for sub_entry in sub_entries.flatten() {
                        if sub_entry
                            .path()
                            .extension()
                            .map_or(false, |ext| ext == "osu")
                        {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

pub fn is_valid_tosu_root(path: &Path) -> bool {
    path_exists(path)
        && path.join("tosu.exe").is_file()
        && path.join("static").is_dir()
}

fn candidate_roots() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            candidates.push(dir.to_path_buf());
            candidates.push(dir.join(".."));
            candidates.push(dir.join("..").join(".."));
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.clone());
        candidates.push(cwd.join(".."));
    }

    if let Some(home) = dirs_next::home_dir() {
        candidates.push(home.clone());
        candidates.push(home.join("Downloads"));
        candidates.push(home.join("Desktop"));
        candidates.push(home.join("Documents"));
        candidates.push(home.join("OneDrive"));
        candidates.push(home.join("OneDrive").join("Desktop"));
        candidates.push(home.join("OneDrive").join("Documents"));
        candidates.push(home.join("AppData").join("Local"));
    }

    if let Some(downloads) = dirs_next::download_dir() {
        candidates.push(downloads);
    }
    if let Some(desktop) = dirs_next::desktop_dir() {
        candidates.push(desktop);
    }
    if let Some(documents) = dirs_next::document_dir() {
        candidates.push(documents);
    }
    if let Some(local) = dirs_next::data_local_dir() {
        candidates.push(local);
    }

    candidates
}

fn scan_for_matching_child_dirs(base: &Path) -> Vec<PathBuf> {
    let mut matches = Vec::new();
    let base = base.to_path_buf();

    if is_valid_tosu_root(&base) {
        matches.push(base.clone());
    }

    if let Ok(entries) = fs::read_dir(&base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.contains("tosu") || name.contains("osu") {
                matches.push(path);
            }
        }
    }

    matches
}

pub fn detect_tosu_root(configured: Option<&Path>) -> Option<PathBuf> {
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();

    if let Some(path) = configured {
        if !path.as_os_str().is_empty() {
            candidates.push(path.to_path_buf());
        }
    }

    if let Some(process_root) = running_tosu_root() {
        candidates.push(process_root);
    }

    for base in candidate_roots() {
        for candidate in scan_for_matching_child_dirs(&base) {
            candidates.push(candidate);
        }
    }

    candidates
        .into_iter()
        .filter(|path| seen.insert(path.to_string_lossy().to_lowercase()))
        .find(|path| is_valid_tosu_root(path))
}

pub fn detect_osu_songs_path() -> Option<PathBuf> {
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();

    if let Some(local) = dirs_next::data_local_dir() {
        candidates.push(local.join("osu!").join("Songs"));
    }
    if let Some(home) = dirs_next::home_dir() {
        candidates.push(home.join("osu!").join("Songs"));
        candidates.push(home.join("Downloads").join("osu!").join("Songs"));
        candidates.push(home.join("Desktop").join("osu!").join("Songs"));
        candidates.push(home.join("Documents").join("osu!").join("Songs"));
        candidates.push(home.join("OneDrive").join("Desktop").join("osu!").join("Songs"));
        candidates.push(home.join("OneDrive").join("Documents").join("osu!").join("Songs"));
    }

    for drive in b'C'..=b'Z' {
        let root = format!("{}:\\", drive as char);
        candidates.push(PathBuf::from(root).join("osu!").join("Songs"));
    }

    candidates
        .into_iter()
        .filter(|path| seen.insert(path.to_string_lossy().to_lowercase()))
        .find(|path| is_valid_osu_songs_path(path))
}

fn running_tosu_root() -> Option<PathBuf> {
    let output = Command::new("powershell")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-CimInstance Win32_Process -Filter \"Name='tosu.exe'\" | Select-Object -First 1 -ExpandProperty ExecutablePath",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return None;
    }

    let exe_path = PathBuf::from(text);
    exe_path.parent().map(|p| p.to_path_buf())
}

fn target_overlay_root(tosu_root: &Path) -> PathBuf {
    tosu_root.join("static")
}

pub fn target_msdconverter_path(tosu_root: &Path) -> PathBuf {
    target_overlay_root(tosu_root).join("msdconverter")
}

fn read_marker(target_root: &Path) -> Option<String> {
    fs::read_to_string(target_root.join(VERSION_MARKER_FILE))
        .ok()
        .map(|value| value.trim().to_string())
}

fn write_marker(target_root: &Path) {
    let _ = fs::write(target_root.join(VERSION_MARKER_FILE), env!("CARGO_PKG_VERSION"));
}

fn is_overlay_installed(target_root: &Path, overlay_name: &str) -> bool {
    let overlay_root = target_root.join(overlay_name);
    overlay_required_files(overlay_name)
        .iter()
        .all(|file_name| overlay_root.join(file_name).exists())
}

fn collect_overlay_statuses(target_root: &Path) -> Vec<OverlayStatus> {
    let installed_version = read_marker(target_root);
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    OVERLAY_NAMES
        .iter()
        .map(|name| {
            let installed = is_overlay_installed(target_root, name);
            OverlayStatus {
                name: (*name).to_string(),
                installed,
                installed_version: installed_version.clone(),
                update_available: installed
                    && installed_version
                        .as_deref()
                        .map(|version| version.trim() != current_version)
                        .unwrap_or(false),
            }
        })
        .collect()
}

fn embedded_overlay_cache_root() -> Option<PathBuf> {
    dirs_next::data_local_dir().map(|dir| {
        dir.join("oris-mania-utils")
            .join("embedded-overlays")
            .join(env!("CARGO_PKG_VERSION"))
    })
}

fn extract_embedded_overlay_files(target_root: &Path) -> Result<usize, String> {
    let mut written_files = 0usize;

    for file in EMBEDDED_OVERLAY_FILES {
        let target_path = target_root.join(file.path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("Could not create embedded overlay parent {}: {}", parent.to_string_lossy(), err)
            })?;
        }
        fs::write(&target_path, file.bytes).map_err(|err| {
            format!(
                "Could not write embedded overlay file {}: {}",
                target_path.to_string_lossy(),
                err
            )
        })?;
        written_files += 1;
    }

    Ok(written_files)
}

fn ensure_embedded_overlay_source_root() -> Option<PathBuf> {
    let cache_root = embedded_overlay_cache_root()?;
    if is_valid_overlay_source_root(&cache_root) {
        return Some(cache_root);
    }

    if fs::create_dir_all(&cache_root).is_err() {
        return None;
    }

    if extract_embedded_overlay_files(&cache_root).is_err() {
        return None;
    }

    if is_valid_overlay_source_root(&cache_root) {
        Some(cache_root)
    } else {
        None
    }
}

fn copy_directory_tree(source: &Path, target: &Path, overlay_name: &str) -> Result<usize, String> {
    if !source.exists() {
        return Err(format!("Missing source overlay folder: {}", source.to_string_lossy()));
    }

    fs::create_dir_all(target)
        .map_err(|err| format!("Could not create {}: {}", target.to_string_lossy(), err))?;

    let mut copied_files = 0usize;
    for relative_path in overlay_source_files(overlay_name) {
        let source_path = source.join(relative_path);
        if !source_path.exists() {
            return Err(format!("Missing source overlay file: {}", source_path.to_string_lossy()));
        }
        let target_path = target.join(relative_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("Could not create {}: {}", parent.to_string_lossy(), err)
            })?;
        }
        fs::copy(&source_path, &target_path).map_err(|err| {
            format!(
                "Could not copy {} to {}: {}",
                source_path.to_string_lossy(),
                target_path.to_string_lossy(),
                err
            )
        })?;
        copied_files += 1;
    }

    Ok(copied_files)
}

fn collect_copyable_files(source: &Path, overlay_name: &str, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for relative_path in overlay_source_files(overlay_name) {
        let source_path = source.join(relative_path);
        if !source_path.exists() {
            return Err(format!("Missing source overlay file: {}", source_path.to_string_lossy()));
        }
        files.push(source_path);
    }
    Ok(())
}

fn copy_overlay_with_progress<F: FnMut(OverlayInstallProgress)>(
    source_root: &Path,
    target_root: &Path,
    overlay_name: &str,
    overall_done: &mut usize,
    overall_total: usize,
    on_progress: &mut F,
) -> Result<usize, String> {
    let source_dir = source_root.join(overlay_name);
    let target_dir = target_root.join(overlay_name);
    fs::create_dir_all(&target_dir).map_err(|err| {
        format!("Could not create {}: {}", target_dir.to_string_lossy(), err)
    })?;

    let mut source_files = Vec::new();
    collect_copyable_files(&source_dir, overlay_name, &mut source_files)?;
    let overlay_total = source_files.len();
    let mut overlay_done = 0usize;

    for source_path in source_files {
        let relative = source_path
            .strip_prefix(&source_dir)
            .map_err(|err| format!("Could not resolve relative path for {}: {}", source_path.to_string_lossy(), err))?;
        let target_path = target_dir.join(relative);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("Could not create {}: {}", parent.to_string_lossy(), err)
            })?;
        }

        fs::copy(&source_path, &target_path).map_err(|err| {
            format!(
                "Could not copy {} to {}: {}",
                source_path.to_string_lossy(),
                target_path.to_string_lossy(),
                err
            )
        })?;

        overlay_done += 1;
        *overall_done += 1;
        on_progress(OverlayInstallProgress {
            overlay: overlay_name.to_string(),
            file: relative.to_string_lossy().replace('\\', "/"),
            overlay_done,
            overlay_total,
            overall_done: *overall_done,
            overall_total,
            message: format!("Installing {} ({}/{})", overlay_name, overlay_done, overlay_total),
        });
    }

    Ok(overlay_done)
}

pub fn locate_overlay_source_root(app: &AppHandle) -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(path) = app.path().resolve("overlays", BaseDirectory::Resource) {
        candidates.push(path);
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("overlays"));
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            candidates.push(dir.join("overlays"));
        }
    }

    #[cfg(debug_assertions)]
    {
        candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("overlays"));
    }

    candidates
        .into_iter()
        .find(|path| is_valid_overlay_source_root(path))
        .or_else(ensure_embedded_overlay_source_root)
}

pub fn ensure_overlays_installed(app: &AppHandle, tosu_root: &Path) -> Result<InstallReport, String> {
    let target_root = target_overlay_root(tosu_root);
    fs::create_dir_all(&target_root).map_err(|err| {
        format!(
            "Could not create TOSU static folder {}: {}",
            target_root.to_string_lossy(),
            err
        )
    })?;

    let current_version = env!("CARGO_PKG_VERSION");
    let target_ready = is_valid_overlay_install_root(&target_root);
    let marker = read_marker(&target_root);

    if target_ready && marker.as_deref() == Some(current_version) {
        return Ok(InstallReport {
            source_root: String::new(),
            target_root: target_root.to_string_lossy().to_string(),
            installed: false,
            already_current: true,
            copied_files: 0,
            installed_overlays: OVERLAY_NAMES.iter().map(|name| (*name).to_string()).collect(),
        });
    }

    let source_root = locate_overlay_source_root(app)
        .ok_or_else(|| "Could not find bundled overlay files to install.".to_string())?;

    let mut copied_files = 0usize;
    for overlay_name in OVERLAY_NAMES {
        let source_dir = source_root.join(overlay_name);
        let target_dir = target_root.join(overlay_name);
        copied_files += copy_directory_tree(&source_dir, &target_dir, overlay_name)?;
    }

    write_marker(&target_root);

    Ok(InstallReport {
        source_root: source_root.to_string_lossy().to_string(),
        target_root: target_root.to_string_lossy().to_string(),
        installed: true,
        already_current: false,
        copied_files,
        installed_overlays: OVERLAY_NAMES.iter().map(|name| (*name).to_string()).collect(),
    })
}

pub fn install_selected_overlays_with_progress<F: FnMut(OverlayInstallProgress)>(
    app: &AppHandle,
    tosu_root: &Path,
    selected_overlays: &[String],
    mut on_progress: F,
) -> Result<InstallReport, String> {
    let target_root = target_overlay_root(tosu_root);
    fs::create_dir_all(&target_root).map_err(|err| {
        format!(
            "Could not create TOSU static folder {}: {}",
            target_root.to_string_lossy(),
            err
        )
    })?;

    let source_root = locate_overlay_source_root(app)
        .ok_or_else(|| "Could not find bundled overlay files to install.".to_string())?;

    let selected: Vec<String> = selected_overlays
        .iter()
        .filter(|name| OVERLAY_NAMES.contains(&name.as_str()))
        .cloned()
        .collect();

    if selected.is_empty() {
        return Err("No valid overlays were selected for installation.".to_string());
    }

    let mut overall_total = 0usize;
    for overlay_name in &selected {
        let mut files = Vec::new();
        collect_copyable_files(&source_root.join(overlay_name), overlay_name, &mut files)?;
        overall_total += files.len();
    }

    let mut overall_done = 0usize;
    let mut copied_files = 0usize;

    for overlay_name in &selected {
        copied_files += copy_overlay_with_progress(
            &source_root,
            &target_root,
            overlay_name,
            &mut overall_done,
            overall_total,
            &mut on_progress,
        )?;
    }

    if collect_overlay_statuses(&target_root).iter().all(|status| status.installed) {
        write_marker(&target_root);
    }

    Ok(InstallReport {
        source_root: source_root.to_string_lossy().to_string(),
        target_root: target_root.to_string_lossy().to_string(),
        installed: true,
        already_current: false,
        copied_files,
        installed_overlays: selected,
    })
}

pub fn bootstrap_config(_app: &AppHandle) -> BootstrapReport {
    let mut config = read_config();
    let mut notes = Vec::new();
    let mut tosu_detected = false;
    let mut songs_detected = false;
    let mut overlays_installed = false;
    let mut overlay_statuses = Vec::new();
    let mut needs_tosu_folder = false;
    let mut needs_songs_folder = false;

    let current_tosu_path = if config.tosu_root_path.is_empty() {
        None
    } else {
        Some(PathBuf::from(&config.tosu_root_path))
    };

    if let Some(path) = current_tosu_path.as_ref() {
        if is_valid_tosu_root(path) {
            tosu_detected = true;
        }
    }

    if !tosu_detected {
        if let Some(detected) = detect_tosu_root(current_tosu_path.as_deref()) {
            config.tosu_root_path = detected.to_string_lossy().to_string();
            tosu_detected = true;
            notes.push(format!("Detected TOSU at {}", config.tosu_root_path));
        } else {
            needs_tosu_folder = true;
        }
    }

    if is_valid_osu_songs_path(Path::new(&config.osu_songs_path)) {
        songs_detected = true;
    } else {
        if let Some(detected) = detect_osu_songs_path() {
            config.osu_songs_path = detected.to_string_lossy().to_string();
            songs_detected = true;
            notes.push(format!(
                "Detected osu! Songs at {}",
                config.osu_songs_path
            ));
        } else {
            needs_songs_folder = true;
        }
    }

    let tosu_root = if is_valid_tosu_root(Path::new(&config.tosu_root_path)) {
        Some(PathBuf::from(&config.tosu_root_path))
    } else {
        None
    };

    if let Some(tosu_root) = tosu_root {
        let target_root = target_overlay_root(&tosu_root);
        overlay_statuses = collect_overlay_statuses(&target_root);
        overlays_installed = overlay_statuses.iter().all(|status| status.installed);
        if overlays_installed {
            notes.push("Overlay files are already installed.".to_string());
        } else {
            notes.push("Overlay installation is still pending.".to_string());
        }
    } else {
        needs_tosu_folder = true;
    }

    BootstrapReport {
        config,
        tosu_detected,
        songs_detected,
        overlays_installed,
        overlay_statuses,
        needs_tosu_folder,
        needs_songs_folder,
        notes,
    }
}
