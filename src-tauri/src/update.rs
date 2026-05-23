use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use futures_util::StreamExt;
use semver::Version;
use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::config::Config;

const GITHUB_RELEASES_LATEST_URL: &str = "https://api.github.com/repos/orianawooo/oris-mania-utils/releases/latest";
const VERSION_MARKER_FILE: &str = ".oris-mania-utils-version";

#[derive(serde::Serialize, Clone)]
pub struct ReleaseAssetInfo {
    pub name: String,
    pub download_url: String,
    pub size: u64,
    pub content_type: String,
}

#[derive(serde::Serialize, Clone)]
pub struct UpdateStatus {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub release_url: Option<String>,
    pub published_at: Option<String>,
    pub app_update_available: bool,
    pub overlay_runtime_version: Option<String>,
    pub overlay_update_available: bool,
    pub overlay_update_requires_app_update: bool,
    pub download_asset: Option<ReleaseAssetInfo>,
    pub notes: Vec<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct AppUpdateProgress {
    pub stage: String,
    pub message: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Deserialize)]
struct GithubReleaseResponse {
    tag_name: String,
    html_url: String,
    published_at: Option<String>,
    assets: Vec<GithubAssetResponse>,
}

#[derive(Deserialize)]
struct GithubAssetResponse {
    name: String,
    browser_download_url: String,
    size: u64,
    content_type: Option<String>,
}

fn normalize_version_tag(tag: &str) -> String {
    tag.trim().trim_start_matches(['v', 'V']).to_string()
}

fn parse_version(tag: &str) -> Option<Version> {
    Version::parse(&normalize_version_tag(tag)).ok()
}

fn version_is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(candidate), Some(current)) => candidate > current,
        _ => normalize_version_tag(candidate) != normalize_version_tag(current),
    }
}

fn overlay_static_root(config: &Config) -> Option<PathBuf> {
    if config.tosu_root_path.trim().is_empty() {
        return None;
    }

    let tosu_root = PathBuf::from(&config.tosu_root_path);
    let msd_path = crate::bootstrap::target_msdconverter_path(&tosu_root);
    msd_path.parent().map(|path| path.to_path_buf())
}

fn read_overlay_runtime_version(config: &Config) -> Option<String> {
    let static_root = overlay_static_root(config)?;
    fs::read_to_string(static_root.join(VERSION_MARKER_FILE))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn select_preferred_asset(assets: &[GithubAssetResponse]) -> Option<ReleaseAssetInfo> {
    let mut scored: Vec<(i32, &GithubAssetResponse)> = assets
        .iter()
        .filter_map(|asset| {
            let lowered = asset.name.to_lowercase();
            let score = if lowered.ends_with(".exe") {
                0
            } else if lowered.ends_with(".msi") {
                1
            } else if lowered.ends_with(".zip") {
                2
            } else {
                return None;
            };
            Some((score, asset))
        })
        .collect();

    scored.sort_by_key(|(score, asset)| (*score, asset.name.clone()));
    scored.first().map(|(_, asset)| ReleaseAssetInfo {
        name: asset.name.clone(),
        download_url: asset.browser_download_url.clone(),
        size: asset.size,
        content_type: asset.content_type.clone().unwrap_or_default(),
    })
}

async fn fetch_latest_release() -> Result<(String, String, Option<String>, Vec<GithubAssetResponse>), String> {
    let client = reqwest::Client::builder()
        .user_agent(format!("oris-mania-utils/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("Could not create update client: {error}"))?;

    let response = client
        .get(GITHUB_RELEASES_LATEST_URL)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| format!("Could not contact GitHub releases: {error}"))?;

    if !response.status().is_success() {
        return Err(format!("GitHub releases returned {}", response.status()));
    }

    let release = response
        .json::<GithubReleaseResponse>()
        .await
        .map_err(|error| format!("Could not parse GitHub release response: {error}"))?;

    Ok((
        normalize_version_tag(&release.tag_name),
        release.html_url,
        release.published_at,
        release.assets,
    ))
}

pub async fn check_for_updates(config: &Config) -> Result<UpdateStatus, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let overlay_runtime_version = read_overlay_runtime_version(config);
    let overlay_update_available = overlay_runtime_version
        .as_deref()
        .map(|installed| normalize_version_tag(installed) != current_version)
        .unwrap_or(false);

    let (latest_version, release_url, published_at, assets) = fetch_latest_release().await?;
    let app_update_available = version_is_newer(&latest_version, &current_version);
    let download_asset = select_preferred_asset(&assets);
    let overlay_update_requires_app_update = app_update_available && !overlay_update_available;

    let mut notes = Vec::new();
    if overlay_runtime_version.is_none() {
        notes.push("No installed overlay version marker was found in tosu/static yet.".to_string());
    } else if overlay_update_available {
        notes.push("The overlays installed in tosu/static are older than this app build.".to_string());
    } else {
        notes.push("Installed overlays already match this app build.".to_string());
    }

    if app_update_available {
        notes.push(format!("GitHub has a newer release available: v{latest_version}."));
    } else {
        notes.push("This app build is already on the latest GitHub release.".to_string());
    }

    Ok(UpdateStatus {
        current_version,
        latest_version: Some(latest_version),
        release_url: Some(release_url),
        published_at,
        app_update_available,
        overlay_runtime_version,
        overlay_update_available,
        overlay_update_requires_app_update,
        download_asset,
        notes,
    })
}

fn escape_ps_single(value: &str) -> String {
    value.replace('\'', "''")
}

fn update_workspace_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_local_data_dir()
        .unwrap_or_else(|_| dirs_next::data_local_dir().unwrap_or_else(|| PathBuf::from(".")))
        .join("updates")
}

async fn download_release_asset(app: &AppHandle, asset: &ReleaseAssetInfo, destination: &Path) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .user_agent(format!("oris-mania-utils/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("Could not create download client: {error}"))?;

    let response = client
        .get(&asset.download_url)
        .send()
        .await
        .map_err(|error| format!("Could not download release asset: {error}"))?;

    if !response.status().is_success() {
        return Err(format!("Release download returned {}", response.status()));
    }

    let total_bytes = response.content_length().unwrap_or(asset.size);
    let mut downloaded_bytes = 0u64;
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(destination)
        .await
        .map_err(|error| format!("Could not create update file: {error}"))?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("Could not read release bytes: {error}"))?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|error| format!("Could not write update file: {error}"))?;
        downloaded_bytes += chunk.len() as u64;
        let _ = app.emit(
            "app-update-progress",
            AppUpdateProgress {
                stage: "download".to_string(),
                message: format!("Downloading {}...", asset.name),
                downloaded_bytes,
                total_bytes,
            },
        );
    }

    let _ = app.emit(
        "app-update-progress",
        AppUpdateProgress {
            stage: "downloaded".to_string(),
            message: format!("Downloaded {}.", asset.name),
            downloaded_bytes,
            total_bytes,
        },
    );

    Ok(())
}

fn spawn_portable_update_script(app: &AppHandle, downloaded_path: &Path, treat_as_zip: bool) -> Result<(), String> {
    let current_exe = std::env::current_exe().map_err(|error| format!("Could not resolve current exe: {error}"))?;
    let backup_exe = current_exe.with_extension("old.exe");
    let updates_dir = update_workspace_dir(app);
    fs::create_dir_all(&updates_dir).map_err(|error| format!("Could not create update workspace: {error}"))?;
    let script_path = updates_dir.join("apply-update.ps1");
    let extract_dir = updates_dir.join("portable-extract");

    let source_preparation = if treat_as_zip {
        format!(
            "$extractDir = '{extract_dir}'\n\
if (Test-Path -LiteralPath $extractDir) {{ Remove-Item -LiteralPath $extractDir -Recurse -Force }}\n\
Expand-Archive -LiteralPath '{downloaded}' -DestinationPath $extractDir -Force\n\
$sourceExe = Get-ChildItem -Path $extractDir -Filter 'oris-mania-utils.exe' -Recurse | Select-Object -First 1 -ExpandProperty FullName\n\
if (-not $sourceExe) {{ throw 'Could not find oris-mania-utils.exe inside the downloaded zip.' }}\n",
            extract_dir = escape_ps_single(&extract_dir.to_string_lossy()),
            downloaded = escape_ps_single(&downloaded_path.to_string_lossy()),
        )
    } else {
        format!(
            "$sourceExe = '{downloaded}'\n",
            downloaded = escape_ps_single(&downloaded_path.to_string_lossy()),
        )
    };

    let script = format!(
        "$ErrorActionPreference = 'Stop'\n\
{source_preparation}\
for ($i = 0; $i -lt 240; $i++) {{\n\
  try {{\n\
    if (Test-Path -LiteralPath '{backup}') {{ Remove-Item -LiteralPath '{backup}' -Force }}\n\
    if (Test-Path -LiteralPath '{target}') {{ Move-Item -LiteralPath '{target}' -Destination '{backup}' -Force }}\n\
    Copy-Item -LiteralPath $sourceExe -Destination '{target}' -Force\n\
    Start-Process -FilePath '{target}'\n\
    exit 0\n\
  }} catch {{\n\
    Start-Sleep -Milliseconds 500\n\
  }}\n\
}}\n\
exit 1\n",
        source_preparation = source_preparation,
        backup = escape_ps_single(&backup_exe.to_string_lossy()),
        target = escape_ps_single(&current_exe.to_string_lossy()),
    );

    fs::write(&script_path, script).map_err(|error| format!("Could not write updater script: {error}"))?;

    Command::new("powershell")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-File",
            script_path.to_string_lossy().as_ref(),
        ])
        .spawn()
        .map_err(|error| format!("Could not start updater helper: {error}"))?;

    let app_handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(350));
        app_handle.exit(0);
    });

    Ok(())
}

pub async fn apply_latest_update(app: AppHandle, config: &Config) -> Result<String, String> {
    let status = check_for_updates(config).await?;
    if !status.app_update_available {
        return Ok("Already running the latest release.".to_string());
    }

    let asset = status
        .download_asset
        .ok_or_else(|| "Latest release has no supported asset to download.".to_string())?;

    let updates_dir = update_workspace_dir(&app);
    fs::create_dir_all(&updates_dir).map_err(|error| format!("Could not create update directory: {error}"))?;
    let staged_path = updates_dir.join(&asset.name);
    if staged_path.exists() {
        let _ = fs::remove_file(&staged_path);
    }

    let _ = app.emit(
        "app-update-progress",
        AppUpdateProgress {
            stage: "prepare".to_string(),
            message: format!("Preparing update to v{}...", status.latest_version.clone().unwrap_or_default()),
            downloaded_bytes: 0,
            total_bytes: asset.size,
        },
    );

    download_release_asset(&app, &asset, &staged_path).await?;

    let lowered = asset.name.to_lowercase();
    if lowered.ends_with(".exe") {
        spawn_portable_update_script(&app, &staged_path, false)?;
        return Ok(format!("Downloaded {} and scheduled restart/update.", asset.name));
    }

    if lowered.ends_with(".msi") {
        Command::new("msiexec")
            .args(["/i", staged_path.to_string_lossy().as_ref()])
            .spawn()
            .map_err(|error| format!("Could not launch MSI installer: {error}"))?;
        return Ok(format!("Downloaded {} and launched the installer.", asset.name));
    }

    if lowered.ends_with(".zip") {
        spawn_portable_update_script(&app, &staged_path, true)?;
        return Ok(format!("Downloaded {} and scheduled restart/update.", asset.name));
    }

    Command::new("explorer.exe")
        .args(["/select,", staged_path.to_string_lossy().as_ref()])
        .spawn()
        .map_err(|error| format!("Could not reveal downloaded asset: {error}"))?;
    Ok(format!("Downloaded {}. Opened its location so you can install it.", asset.name))
}
