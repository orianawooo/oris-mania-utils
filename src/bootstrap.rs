use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::Command;

const VERSION_MARKER_FILE: &str = ".oris-mania-utils-version";

struct EmbeddedOverlayFile {
    path: &'static str,
    bytes: &'static [u8],
}

const EMBEDDED_OVERLAY_FILES: &[EmbeddedOverlayFile] = &[
    EmbeddedOverlayFile {
        path: "radar/index.html",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/overlays/radar/index.html"
        )),
    },
    EmbeddedOverlayFile {
        path: "radar/main.js",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/overlays/radar/main.js"
        )),
    },
    EmbeddedOverlayFile {
        path: "radar/styles.css",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/overlays/radar/styles.css"
        )),
    },
    EmbeddedOverlayFile {
        path: "keys/index.html",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/overlays/keys/index.html"
        )),
    },
    EmbeddedOverlayFile {
        path: "keys/keystrokes.css",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/overlays/keys/keystrokes.css"
        )),
    },
    EmbeddedOverlayFile {
        path: "keys/keystrokes-geometry.js",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/overlays/keys/keystrokes-geometry.js"
        )),
    },
    EmbeddedOverlayFile {
        path: "keys/keystrokes.js",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/overlays/keys/keystrokes.js"
        )),
    },
    EmbeddedOverlayFile {
        path: "hits/hitcounter-layout.js",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/overlays/hits/hitcounter-layout.js"
        )),
    },
    EmbeddedOverlayFile {
        path: "hits/index.html",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/overlays/hits/index.html"
        )),
    },
    EmbeddedOverlayFile {
        path: "hits/main.js",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/overlays/hits/main.js"
        )),
    },
    EmbeddedOverlayFile {
        path: "hits/styles.css",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/overlays/hits/styles.css"
        )),
    },
];

const LEGACY_OVERLAY_DIRS: &[&str] = &["msdconverter", "ManiaKeystrokes", "HitCounter"];

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
                        if sub_entry.path().extension().is_some_and(|ext| ext == "osu") {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

pub fn resolve_tosu_root(path: &Path) -> Option<PathBuf> {
    if !path.exists() || !path.is_dir() {
        return None;
    }
    #[cfg(target_os = "windows")]
    let bin_name = "tosu.exe";
    #[cfg(not(target_os = "windows"))]
    let bin_name = "tosu";

    let valid_bin = |dir: &Path| -> bool {
        let p = dir.join(bin_name);
        p.is_file()
            && fs::metadata(&p)
                .map(|m| m.len() > 1_000_000)
                .unwrap_or(false)
    };

    if valid_bin(path) {
        return Some(path.to_path_buf());
    }
    if let Some(parent) = path.parent() {
        if valid_bin(parent) {
            return Some(parent.to_path_buf());
        }
    }
    None
}

pub fn is_valid_tosu_root(path: &Path) -> bool {
    resolve_tosu_root(path).is_some()
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
        candidates.push(home.join("tosu"));
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
        candidates.push(local.join("oris-mania-utils").join("tosu"));
        candidates.push(local);
    }

    candidates
}

fn scan_for_matching_child_dirs(base: &Path) -> Vec<PathBuf> {
    let mut matches = Vec::new();

    if is_valid_tosu_root(base) {
        matches.push(base.to_path_buf());
    }

    if let Ok(entries) = fs::read_dir(base) {
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

fn running_tosu_root() -> Option<PathBuf> {
    #[cfg(windows)]
    {
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

        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !text.is_empty() {
                return PathBuf::from(text).parent().map(|p| p.to_path_buf());
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(entries) = fs::read_dir("/proc") {
            for entry in entries.flatten() {
                let exe_link = entry.path().join("exe");
                if let Ok(target) = fs::read_link(exe_link) {
                    if target
                        .file_name()
                        .is_some_and(|n| n == "tosu" || n == "tosu.exe")
                    {
                        if let Some(parent) = target.parent() {
                            return Some(parent.to_path_buf());
                        }
                    }
                }
            }
        }
    }

    None
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
        .find_map(|path| resolve_tosu_root(&path))
}

fn custom_beatmap_dirs_from_osu_cfg(osu_dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(osu_dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if !name.starts_with("osu!.") || !name.ends_with(".cfg") {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                if k.trim().eq_ignore_ascii_case("BeatmapDirectory") {
                    let cleaned = v.trim().trim_matches('"').trim_matches('\'');
                    if !cleaned.is_empty() {
                        let candidate = PathBuf::from(cleaned);
                        if candidate.is_absolute() {
                            out.push(candidate);
                        } else {
                            out.push(osu_dir.join(candidate));
                        }
                    }
                }
            }
        }
    }
    out
}

pub fn detect_osu_songs_path() -> Option<PathBuf> {
    let mut seen = HashSet::new();
    let mut osu_roots = Vec::new();
    let mut candidates = Vec::new();

    if let Some(home) = dirs_next::home_dir() {
        osu_roots.push(home.join("Games").join("osu!"));
        osu_roots.push(home.join("osu!"));
    }
    if let Some(local) = dirs_next::data_local_dir() {
        osu_roots.push(local.join("osu!"));
        #[cfg(target_os = "linux")]
        {
            osu_roots.push(local.join("osu-wine").join("osu!"));
            osu_roots.push(local.join("osu-wine").join("drive_c").join("osu!"));
            osu_roots.push(
                local
                    .join("lutris")
                    .join("runners")
                    .join("osu")
                    .join("drive_c")
                    .join("osu!"),
            );
        }
    }
    if let Some(home) = dirs_next::home_dir() {
        #[cfg(target_os = "linux")]
        {
            osu_roots.push(home.join(".wine").join("drive_c").join("osu!"));
            osu_roots.push(home.join("Games").join("osu").join("drive_c").join("osu!"));
        }
        osu_roots.push(home.join("Downloads").join("osu!"));
        osu_roots.push(home.join("Desktop").join("osu!"));
        osu_roots.push(home.join("Documents").join("osu!"));
        #[cfg(windows)]
        {
            osu_roots.push(home.join("OneDrive").join("Desktop").join("osu!"));
            osu_roots.push(home.join("OneDrive").join("Documents").join("osu!"));
        }
    }

    #[cfg(windows)]
    for drive in b'C'..=b'Z' {
        let root = format!("{}:\\", drive as char);
        osu_roots.push(PathBuf::from(root).join("osu!"));
    }

    for osu_root in &osu_roots {
        candidates.extend(custom_beatmap_dirs_from_osu_cfg(osu_root));
        candidates.push(osu_root.join("Songs"));
    }

    candidates
        .into_iter()
        .filter(|path| seen.insert(path.to_string_lossy().to_lowercase()))
        .find(|path| is_valid_osu_songs_path(path))
        .map(|p| fs::canonicalize(&p).unwrap_or(p))
}

pub fn resolve_tosu_static_dir(tosu_root: &Path) -> PathBuf {
    let env_path = tosu_root.join("tosu.env");
    if let Ok(content) = fs::read_to_string(&env_path) {
        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                if key.trim().eq_ignore_ascii_case("STATIC_FOLDER_PATH") {
                    let cleaned = value.trim().trim_matches('"').trim_matches('\'');
                    if !cleaned.is_empty() {
                        let candidate = PathBuf::from(cleaned);
                        if candidate.is_absolute() {
                            return candidate;
                        }
                        return tosu_root.join(candidate);
                    }
                }
            }
        }
    }
    tosu_root.join("static")
}

pub fn target_radar_path(tosu_root: &Path) -> PathBuf {
    resolve_tosu_static_dir(tosu_root).join("radar")
}

#[cfg(target_os = "linux")]
pub fn ensure_linux_desktop_entry() {
    let Some(data_dir) = dirs_next::data_local_dir() else {
        return;
    };
    let icon_dir = data_dir
        .join("icons")
        .join("hicolor")
        .join("256x256")
        .join("apps");
    let icon_path = icon_dir.join("oris-mania-utils.png");
    let logo_bytes = include_bytes!("../assets/logo.png");
    let icon_same = fs::read(&icon_path).ok().as_deref() == Some(logo_bytes);
    if !icon_same && fs::create_dir_all(&icon_dir).is_ok() {
        let _ = fs::write(&icon_path, logo_bytes);
    }

    let apps_dir = data_dir.join("applications");
    let desktop_path = apps_dir.join("oris-mania-utils.desktop");
    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "oris-mania-utils".to_string());
    let desktop_body = format!(
        "[Desktop Entry]\nType=Application\nName=ori's mania utils\nExec=\"{}\"\nIcon={}\nStartupWMClass=oris-mania-utils\nCategories=Game;Utility;\nNoDisplay=true\n",
        exe,
        icon_path.to_string_lossy()
    );
    let desktop_same =
        fs::read_to_string(&desktop_path).ok().as_deref() == Some(desktop_body.as_str());
    if !desktop_same && fs::create_dir_all(&apps_dir).is_ok() {
        let _ = fs::write(&desktop_path, desktop_body);
    }
}

pub fn extract_embedded_overlay_files(target_root: &Path) -> Result<usize, String> {
    for legacy_dir in LEGACY_OVERLAY_DIRS {
        let p = target_root.join(legacy_dir);
        if p.exists() {
            let _ = fs::remove_dir_all(p);
        }
    }

    let mut written_files = 0usize;
    for file in EMBEDDED_OVERLAY_FILES {
        let target_path = target_root.join(file.path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "could not create overlay folder {}: {}",
                    parent.to_string_lossy(),
                    err
                )
            })?;
        }
        let unchanged = fs::metadata(&target_path)
            .map(|m| m.len() as usize == file.bytes.len())
            .unwrap_or(false)
            && fs::read(&target_path).ok().as_deref() == Some(file.bytes);
        if !unchanged {
            fs::write(&target_path, file.bytes).map_err(|err| {
                format!(
                    "could not write overlay file {}: {}",
                    target_path.to_string_lossy(),
                    err
                )
            })?;
        }
        written_files += 1;
    }

    let _ = fs::write(
        target_root.join(VERSION_MARKER_FILE),
        env!("CARGO_PKG_VERSION"),
    );
    Ok(written_files)
}
