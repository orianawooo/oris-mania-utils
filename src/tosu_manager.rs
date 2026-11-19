use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::Duration;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::bootstrap::{detect_tosu_root, is_valid_tosu_root, resolve_tosu_root};
use crate::config::{read_config, write_config};

const TOSU_RELEASES_LATEST_URL: &str = "https://api.github.com/repos/tosuapp/tosu/releases/latest";
const TOSU_VERSION_FILE: &str = ".tosu-version";

lazy_static::lazy_static! {
    static ref MANAGED_TOSU_CHILD: Mutex<Option<Child>> = Mutex::new(None);
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TosuClientStatus {
    pub installed: bool,
    pub running: bool,
    pub path: String,
    pub port: u16,
    pub installed_version: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct TosuInstallProgress {
    pub stage: String,
    pub message: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Deserialize)]
struct GithubReleaseResponse {
    tag_name: String,
    assets: Vec<GithubAssetResponse>,
}

#[derive(Deserialize)]
struct GithubAssetResponse {
    name: String,
    browser_download_url: String,
    size: u64,
}

pub fn managed_tosu_dir_standalone() -> PathBuf {
    if let Some(dir) = dirs_next::data_local_dir() {
        return dir.join("oris-mania-utils").join("tosu");
    }
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            return parent.join("tosu");
        }
    }
    PathBuf::from("tosu")
}

pub fn find_tosu_exe(root: &Path) -> Option<PathBuf> {
    let win_exe = root.join("tosu.exe");
    if win_exe.is_file() {
        return Some(win_exe);
    }
    let nix_exe = root.join("tosu");
    if nix_exe.is_file() {
        return Some(nix_exe);
    }
    None
}

pub fn is_tosu_running_locally(port: u16) -> bool {
    use std::net::ToSocketAddrs;
    let host = crate::config::runtime_tosu_host();
    if let Ok(addrs) = (host.as_str(), port).to_socket_addrs() {
        for addr in addrs {
            if std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
                return true;
            }
        }
    }
    false
}

pub fn read_tosu_env_network(tosu_root: &Path) -> (Option<String>, Option<u16>) {
    let env_path = tosu_root.join("tosu.env");
    let Ok(content) = fs::read_to_string(&env_path) else {
        return (None, None);
    };
    let mut host = None;
    let mut port = None;
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim();
            let val = v.trim().trim_matches('"').trim_matches('\'');
            if key.eq_ignore_ascii_case("SERVER_IP") && !val.is_empty() {
                host = Some(crate::config::normalize_host_str(val));
            } else if key.eq_ignore_ascii_case("SERVER_PORT") {
                if let Ok(parsed) = val.parse::<u16>() {
                    if parsed >= 1024 {
                        port = Some(parsed);
                    }
                }
            }
        }
    }
    (host, port)
}

#[cfg(windows)]
fn is_wine_env() -> bool {
    std::env::var_os("WINEPREFIX").is_some()
        || Path::new("C:\\windows\\system32\\wineboot.exe").is_file()
}

pub fn ensure_tosu_overlay_env(tosu_root: &Path) -> bool {
    let env_path = tosu_root.join("tosu.env");
    let existing = fs::read_to_string(&env_path).unwrap_or_default();
    let cfg = read_config();

    let mut filtered: Vec<String> = existing
        .lines()
        .filter(|line| {
            let t = line.trim();
            !t.starts_with("ENABLE_GOSU_OVERLAY=")
                && !t.starts_with("ENABLE_INGAME_OVERLAY=")
                && !t.starts_with("ENABLE_KEY_OVERLAY=")
                && !t.starts_with("POLL_RATE=")
                && !t.starts_with("PRECISE_DATA_POLL_RATE=")
                && !t.starts_with("KEYOVERLAY_POLL_RATE=")
                && !t.starts_with("OPEN_DASHBOARD_ON_STARTUP=")
                && !t.starts_with("SHOW_MP_MENU=")
                && !t.starts_with("SERVER_PORT=")
        })
        .map(|s| s.to_string())
        .collect();

    let precise_rate = cfg.hitcounter_poll_ms.clamp(5, 100);
    let server_port = cfg.tosu_port.max(1024);
    filtered.push(format!("SERVER_PORT={}", server_port));
    if !filtered.iter().any(|l| l.trim().starts_with("SERVER_IP=")) {
        filtered.push(format!(
            "SERVER_IP={}",
            crate::config::normalize_host_str(&cfg.tosu_host)
        ));
    }
    filtered.push("POLL_RATE=100".to_string());
    filtered.push(format!("PRECISE_DATA_POLL_RATE={}", precise_rate));
    filtered.push("ENABLE_KEY_OVERLAY=true".to_string());
    filtered.push("ENABLE_GOSU_OVERLAY=false".to_string());
    filtered.push("OPEN_DASHBOARD_ON_STARTUP=false".to_string());

    #[cfg(windows)]
    {
        let enable_ingame = !is_wine_env() && cfg.enable_tosu_ingame_overlay;
        filtered.push(format!("ENABLE_INGAME_OVERLAY={}", enable_ingame));
        if enable_ingame {
            seed_default_ingame_overlays(tosu_root);
        }
    }
    #[cfg(not(windows))]
    {
        let _ = cfg.enable_tosu_ingame_overlay;
        filtered.push("ENABLE_INGAME_OVERLAY=false".to_string());
    }

    if !filtered
        .iter()
        .any(|l| l.trim().starts_with("STATIC_FOLDER_PATH="))
    {
        filtered.push("STATIC_FOLDER_PATH=./static".to_string());
    }

    let mut new_content = filtered.join("\n");
    if !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    if existing == new_content {
        return false;
    }
    let _ = fs::write(&env_path, new_content);
    true
}

#[cfg(windows)]
fn seed_default_ingame_overlays(tosu_root: &Path) {
    let settings_dir = tosu_root.join("settings");
    let ingame_path = settings_dir.join("__ingame__.values.json");
    if let Ok(existing) = fs::read_to_string(&ingame_path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&existing) {
            let has_overlays = val
                .get("profiles")
                .and_then(|p| p.as_array())
                .and_then(|arr| arr.first())
                .and_then(|p| p.get("overlays"))
                .and_then(|o| o.as_array())
                .map(|o| !o.is_empty())
                .unwrap_or(false);
            if has_overlays {
                return;
            }
        }
    }
    let _ = fs::create_dir_all(&settings_dir);
    let default_json = r#"{"obs_profile":"default","ingame_profile":"default","profiles":[{"id":"default","name":"default","overlays":[{"_enabled":true,"_settings":false,"id":1,"folderName":"radar","url":"tosu://server/radar","width":340,"height":360,"top":20,"left":20,"scale":1,"z_index":1},{"_enabled":true,"_settings":false,"id":2,"folderName":"keys","url":"tosu://server/keys","width":260,"height":340,"top":40,"left":380,"scale":1,"z_index":1},{"_enabled":true,"_settings":false,"id":3,"folderName":"hits","url":"tosu://server/hits","width":180,"height":220,"top":400,"left":20,"scale":1,"z_index":1}]}]}"#;
    let _ = fs::write(ingame_path, default_json);
}

#[cfg(windows)]
fn terminate_tosu_in_dir(tosu_root: &Path) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let exe_target = tosu_root
        .join("tosu.exe")
        .to_string_lossy()
        .replace('\'', "''");
    let script = format!(
        "Get-CimInstance Win32_Process -Filter \"Name='tosu.exe'\" | Where-Object {{ $_.ExecutablePath -eq '{}' }} | ForEach-Object {{ Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }}",
        exe_target
    );
    let _ = Command::new("powershell")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
}

pub fn launch_tosu_process(tosu_root: &Path) -> Result<String, String> {
    let env_changed = ensure_tosu_overlay_env(tosu_root);
    let cfg = read_config();
    let port = cfg.tosu_port.max(1024);
    if is_tosu_running_locally(port) {
        if env_changed {
            push_tosu_runtime_settings(port, &cfg);
        }
        return Ok("tosu is already running".to_string());
    }

    let exe = find_tosu_exe(tosu_root).ok_or_else(|| {
        format!(
            "could not find tosu executable in {}",
            tosu_root.to_string_lossy()
        )
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(&exe) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            let _ = fs::set_permissions(&exe, permissions);
        }
    }

    let mut cmd = Command::new(&exe);
    cmd.current_dir(tosu_root);
    if !cfg.show_tosu_console {
        cmd.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
    }

    let child = cmd
        .spawn()
        .map_err(|err| format!("failed to launch tosu: {}", err))?;

    let mut lock = MANAGED_TOSU_CHILD.lock().unwrap();
    if let Some(mut old) = lock.take() {
        let _ = old.kill();
    }
    *lock = Some(child);

    Ok(exe.to_string_lossy().to_string())
}

pub fn push_tosu_runtime_settings(port: u16, cfg: &crate::config::Config) {
    #[cfg(windows)]
    let enable_ingame = !is_wine_env() && cfg.enable_tosu_ingame_overlay;
    #[cfg(not(windows))]
    let enable_ingame = false;

    let host = crate::config::normalize_host_str(&cfg.tosu_host);
    let precise_rate = cfg.hitcounter_poll_ms.clamp(5, 100);
    let payload = serde_json::json!({
        "pollRate": 100,
        "preciseDataPollRate": precise_rate,
        "enableKeyOverlay": true,
        "enableGosuOverlay": false,
        "openDashboardOnStartup": false,
        "enableIngameOverlay": enable_ingame,
    });

    std::thread::spawn(move || {
        if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            rt.block_on(async move {
                if let Ok(client) = reqwest::Client::builder()
                    .timeout(Duration::from_secs(2))
                    .build()
                {
                    let _ = client
                        .post(format!("http://{}:{}/api/settings", host, port))
                        .json(&payload)
                        .send()
                        .await;
                }
            });
        }
    });
}

pub fn restart_tosu_process(tosu_root: &Path) -> Result<String, String> {
    stop_tosu_process();
    #[cfg(windows)]
    terminate_tosu_in_dir(tosu_root);
    std::thread::sleep(Duration::from_millis(250));
    launch_tosu_process(tosu_root)
}

pub fn stop_tosu_process() {
    if let Ok(mut lock) = MANAGED_TOSU_CHILD.lock() {
        if let Some(mut child) = lock.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub fn read_installed_tosu_version(tosu_root: &Path) -> String {
    if let Ok(v) = fs::read_to_string(tosu_root.join(TOSU_VERSION_FILE)) {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    String::new()
}

pub fn get_tosu_client_status_standalone() -> TosuClientStatus {
    let config = read_config();
    let configured_path = if config.tosu_root_path.trim().is_empty() {
        None
    } else {
        Some(PathBuf::from(&config.tosu_root_path))
    };

    let resolved = configured_path
        .as_deref()
        .and_then(resolve_tosu_root)
        .or_else(|| detect_tosu_root(None))
        .or_else(|| {
            let m = managed_tosu_dir_standalone();
            if is_valid_tosu_root(&m) {
                Some(m)
            } else {
                None
            }
        });

    let installed = resolved.is_some();
    let path_str = resolved
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let installed_version = resolved
        .as_ref()
        .map(|p| read_installed_tosu_version(p))
        .unwrap_or_default();

    let port = config.tosu_port.max(1024);
    let running = is_tosu_running_locally(port);

    TosuClientStatus {
        installed,
        running,
        path: path_str,
        port,
        installed_version,
    }
}

pub async fn fetch_latest_tosu_version() -> Option<String> {
    let client = reqwest::Client::builder()
        .user_agent("oris-mania-utils")
        .timeout(Duration::from_secs(6))
        .build()
        .ok()?;
    let resp = client.get(TOSU_RELEASES_LATEST_URL).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let release = resp.json::<GithubReleaseResponse>().await.ok()?;
    Some(release.tag_name)
}

pub async fn install_tosu_core<F>(target_dir: PathBuf, mut progress_cb: F) -> Result<String, String>
where
    F: FnMut(TosuInstallProgress) + Send + 'static,
{
    let res = install_tosu_core_inner(&target_dir, &mut progress_cb).await;
    if let Err(ref err) = res {
        progress_cb(TosuInstallProgress {
            stage: "error".to_string(),
            message: err.clone(),
            downloaded_bytes: 0,
            total_bytes: 0,
        });
    }
    res
}

async fn install_tosu_core_inner<F>(
    target_dir: &Path,
    progress_cb: &mut F,
) -> Result<String, String>
where
    F: FnMut(TosuInstallProgress) + Send + 'static,
{
    progress_cb(TosuInstallProgress {
        stage: "fetching".to_string(),
        message: "fetching latest tosu release...".to_string(),
        downloaded_bytes: 0,
        total_bytes: 0,
    });

    let client = reqwest::Client::builder()
        .user_agent("oris-mania-utils")
        .connect_timeout(Duration::from_secs(15))
        .build()
        .map_err(|err| format!("failed to build http client: {}", err))?;

    let release_resp = client
        .get(TOSU_RELEASES_LATEST_URL)
        .send()
        .await
        .map_err(|err| format!("failed to query github releases: {}", err))?;

    if !release_resp.status().is_success() {
        return Err(format!(
            "github api returned status {}",
            release_resp.status()
        ));
    }

    let release = release_resp
        .json::<GithubReleaseResponse>()
        .await
        .map_err(|err| format!("failed to parse tosu release info: {}", err))?;

    let os_keyword = if cfg!(windows) { "windows" } else { "linux" };
    let asset = release
        .assets
        .into_iter()
        .find(|a| {
            let lower = a.name.to_lowercase();
            lower.contains(os_keyword) && lower.ends_with(".zip")
        })
        .ok_or_else(|| {
            format!(
                "no supported {} zip asset found for tosu {}",
                os_keyword, release.tag_name
            )
        })?;

    fs::create_dir_all(target_dir).map_err(|err| {
        format!(
            "could not create tosu directory {}: {}",
            target_dir.to_string_lossy(),
            err
        )
    })?;

    let zip_part = target_dir.join(format!("tosu-{}.zip.part", release.tag_name));
    let zip_dest = target_dir.join(format!("tosu-{}.zip", release.tag_name));
    let staging_dir = target_dir.join(".tmp-tosu-extract");
    let _ = fs::remove_file(&zip_part);
    let _ = fs::remove_dir_all(&staging_dir);

    progress_cb(TosuInstallProgress {
        stage: "downloading".to_string(),
        message: format!("downloading tosu {}...", release.tag_name),
        downloaded_bytes: 0,
        total_bytes: asset.size,
    });

    let download_resp = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .map_err(|err| format!("failed to download tosu zip: {}", err))?;

    if !download_resp.status().is_success() {
        return Err(format!(
            "download returned status {}",
            download_resp.status()
        ));
    }

    let total_bytes = download_resp.content_length().unwrap_or(asset.size);
    let mut downloaded_bytes = 0u64;
    let mut stream = download_resp.bytes_stream();
    let mut out_file = tokio::fs::File::create(&zip_part)
        .await
        .map_err(|err| format!("could not create destination zip file: {}", err))?;

    use tokio::io::AsyncWriteExt;
    let mut last_emit = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|err| format!("download stream interrupted: {}", err))?;
        out_file
            .write_all(&chunk)
            .await
            .map_err(|err| format!("failed to write zip chunk: {}", err))?;
        downloaded_bytes += chunk.len() as u64;

        if last_emit.elapsed() >= Duration::from_millis(150) || downloaded_bytes == total_bytes {
            last_emit = std::time::Instant::now();
            progress_cb(TosuInstallProgress {
                stage: "downloading".to_string(),
                message: format!(
                    "downloading tosu ({:.1} mb / {:.1} mb)...",
                    (downloaded_bytes as f64) / (1024.0 * 1024.0),
                    (total_bytes as f64) / (1024.0 * 1024.0)
                ),
                downloaded_bytes,
                total_bytes,
            });
        }
    }

    out_file
        .flush()
        .await
        .map_err(|err| format!("could not flush zip file: {}", err))?;
    drop(out_file);

    if downloaded_bytes < 1_000_000 || (total_bytes > 0 && downloaded_bytes < total_bytes) {
        let _ = fs::remove_file(&zip_part);
        return Err(format!(
            "incomplete tosu download ({} / {} bytes)",
            downloaded_bytes, total_bytes
        ));
    }
    fs::rename(&zip_part, &zip_dest)
        .map_err(|err| format!("could not finalize zip archive: {}", err))?;

    progress_cb(TosuInstallProgress {
        stage: "extracting".to_string(),
        message: "extracting tosu files...".to_string(),
        downloaded_bytes: total_bytes,
        total_bytes,
    });

    fs::create_dir_all(&staging_dir)
        .map_err(|err| format!("could not create staging dir: {}", err))?;

    let file =
        File::open(&zip_dest).map_err(|err| format!("could not open downloaded zip: {}", err))?;
    let mut archive = zip::ZipArchive::new(BufReader::new(file))
        .map_err(|err| format!("invalid zip archive: {}", err))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|err| format!("failed to read zip entry: {}", err))?;

        let outpath = match entry.enclosed_name() {
            Some(path) => staging_dir.join(path),
            None => continue,
        };

        if entry.name().ends_with('/') {
            fs::create_dir_all(&outpath).ok();
        } else {
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p).ok();
            }
            let mut outfile = File::create(&outpath).map_err(|err| {
                format!(
                    "could not extract file {}: {}",
                    outpath.to_string_lossy(),
                    err
                )
            })?;
            std::io::copy(&mut entry, &mut outfile)
                .map_err(|err| format!("failed to write extracted file: {}", err))?;
        }
    }
    drop(archive);

    let extracted_bin = find_tosu_exe(&staging_dir)
        .or_else(|| {
            fs::read_dir(&staging_dir).ok().and_then(|rd| {
                rd.flatten()
                    .filter(|e| e.path().is_dir())
                    .find_map(|e| find_tosu_exe(&e.path()))
            })
        })
        .ok_or_else(|| "downloaded zip did not contain a tosu binary".to_string())?;

    let bin_len = fs::metadata(&extracted_bin).map(|m| m.len()).unwrap_or(0);
    if bin_len < 1_000_000 {
        let _ = fs::remove_dir_all(&staging_dir);
        let _ = fs::remove_file(&zip_dest);
        return Err("extracted tosu binary looks corrupted or empty".to_string());
    }

    stop_tosu_process();
    #[cfg(windows)]
    terminate_tosu_in_dir(target_dir);

    let bin_name = extracted_bin
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            if cfg!(windows) {
                "tosu.exe".to_string()
            } else {
                "tosu".to_string()
            }
        });
    let dest_bin = target_dir.join(&bin_name);
    let backup_bin = target_dir.join(format!("{}.old", bin_name));
    let _ = fs::remove_file(&backup_bin);
    if dest_bin.exists() {
        let _ = fs::rename(&dest_bin, &backup_bin);
    }
    if let Err(err) = fs::rename(&extracted_bin, &dest_bin) {
        if let Err(copy_err) = fs::copy(&extracted_bin, &dest_bin) {
            if backup_bin.exists() {
                let _ = fs::rename(&backup_bin, &dest_bin);
            }
            let _ = fs::remove_dir_all(&staging_dir);
            let _ = fs::remove_file(&zip_dest);
            return Err(format!(
                "could not install {}: {} / {}",
                bin_name, err, copy_err
            ));
        }
    }
    let _ = fs::remove_file(&backup_bin);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(&dest_bin) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            let _ = fs::set_permissions(&dest_bin, permissions);
        }
    }

    let _ = fs::remove_dir_all(&staging_dir);
    let _ = fs::remove_file(&zip_dest);
    let _ = fs::write(target_dir.join(TOSU_VERSION_FILE), &release.tag_name);
    ensure_tosu_overlay_env(target_dir);

    if crate::config::runtime_browser_overlays() {
        let static_dir = crate::bootstrap::resolve_tosu_static_dir(target_dir);
        let _ = fs::create_dir_all(&static_dir);
        let _ = crate::bootstrap::extract_embedded_overlay_files(&static_dir);
    }

    let mut cfg = read_config();
    cfg.tosu_root_path = target_dir.to_string_lossy().to_string();
    write_config(&cfg);

    progress_cb(TosuInstallProgress {
        stage: "complete".to_string(),
        message: format!("tosu {} installed", release.tag_name),
        downloaded_bytes: total_bytes,
        total_bytes,
    });

    Ok(target_dir.to_string_lossy().to_string())
}

pub fn managed_engines_dir() -> PathBuf {
    if let Some(dir) = dirs_next::data_local_dir() {
        return dir.join("oris-mania-utils").join("engines");
    }
    PathBuf::from("engines")
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EngineStatus {
    pub installed_version: String,
    pub path: String,
    pub synced: bool,
}

pub fn get_minacalc_status() -> EngineStatus {
    let dir = managed_engines_dir().join("minacalc");
    let ver_file = dir.join(".minacalc-version");
    let ver = fs::read_to_string(&ver_file)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "v515 (embedded)".to_string());
    let synced = ver_file.is_file();
    EngineStatus {
        installed_version: if ver.is_empty() {
            "v515 (embedded)".to_string()
        } else {
            ver
        },
        path: dir.to_string_lossy().to_string(),
        synced,
    }
}

pub fn get_daniel_status() -> EngineStatus {
    let dir = managed_engines_dir().join("daniel");
    let ver_file = dir.join(".daniel-version");
    let ver = fs::read_to_string(&ver_file)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "xdc1 + companella (embedded)".to_string());
    let synced = ver_file.is_file();
    EngineStatus {
        installed_version: if ver.is_empty() {
            "xdc1 + companella (embedded)".to_string()
        } else {
            ver
        },
        path: dir.to_string_lossy().to_string(),
        synced,
    }
}

pub async fn download_latest_minacalc() -> Result<String, String> {
    let dir = managed_engines_dir().join("minacalc");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let client = reqwest::Client::builder()
        .user_agent("oris-mania-utils")
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let mut version_label = "v515-latest".to_string();
    if let Ok(resp) = client
        .get("https://api.github.com/repos/etternagame/minacalc/commits/master")
        .send()
        .await
    {
        if let Ok(val) = resp.json::<serde_json::Value>().await {
            if let Some(sha) = val.get("sha").and_then(|s| s.as_str()) {
                version_label = format!("v515 ({})", &sha[..7.min(sha.len())]);
            }
        }
    }

    for file in ["MinaCalc.h", "MinaCalc.cpp", "Ulbu.h"] {
        let url = format!(
            "https://raw.githubusercontent.com/etternagame/minacalc/master/MinaCalc/{}",
            file
        );
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(bytes) = resp.bytes().await {
                    let _ = fs::write(dir.join(file), &bytes);
                }
            }
        }
    }

    fs::write(dir.join(".minacalc-version"), &version_label).map_err(|e| e.to_string())?;
    Ok(version_label)
}

pub async fn download_latest_daniel() -> Result<String, String> {
    let dir = managed_engines_dir().join("daniel");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let client = reqwest::Client::builder()
        .user_agent("oris-mania-utils")
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let mut version_label = "xdc1-latest".to_string();
    if let Ok(resp) = client
        .get("https://api.github.com/repos/TheBagelOfMan/Daniel/commits/main")
        .send()
        .await
    {
        if let Ok(val) = resp.json::<serde_json::Value>().await {
            if let Some(sha) = val.get("sha").and_then(|s| s.as_str()) {
                version_label = format!("xdc1 ({}) + companella", &sha[..7.min(sha.len())]);
            }
        }
    }

    let xdc1_url = "https://raw.githubusercontent.com/TheBagelOfMan/Daniel/main/xdc1/xdc1.cpp";
    let resp = client
        .get(xdc1_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        let cpp_path = dir.join("xdc1.cpp");
        fs::write(&cpp_path, &bytes).map_err(|e| e.to_string())?;
    }

    let scale_url = "https://raw.githubusercontent.com/TheBagelOfMan/Daniel/main/src/dan_scale.py";
    if let Ok(sresp) = client.get(scale_url).send().await {
        if sresp.status().is_success() {
            if let Ok(b) = sresp.bytes().await {
                let _ = fs::write(dir.join("dan_scale.py"), &b);
            }
        }
    }

    let onnx_url = "https://raw.githubusercontent.com/orianawooo/danorganizer/main/dan_model.onnx";
    if let Ok(oresp) = client.get(onnx_url).send().await {
        if oresp.status().is_success() {
            if let Ok(b) = oresp.bytes().await {
                let _ = fs::write(managed_engines_dir().join("dan_model.onnx"), &b);
            }
        }
    }

    fs::write(dir.join(".daniel-version"), &version_label).map_err(|e| e.to_string())?;
    Ok(version_label)
}
