import { state } from './state.js';
import { setupTosuConnection } from './api.js';
import { applyVisualSettings, populateSettingsPanel } from './config.js';
import { initUIListeners, initWindowControls, readAndSaveSetupSettings, setEditorTabAvailability } from './events.js';
import { drawRadar, clearRadar, initChart } from './chart.js';
import { initHitCounterEditor, ensureHitCounterConfig, applyHitCounterToDocument } from './hitcounter-editor.js';
import { applyConfigSnapshot, loadDefaultConfig, setPersistedConfigBaseline } from './config-store.js';
import { recordPerf } from './perf.js';

const isTauri = typeof window.__TAURI__ !== 'undefined';
const invoke = isTauri ? window.__TAURI__.core.invoke : async () => {};

const setupScreen = document.getElementById('setup-screen');
const overlay = document.getElementById('overlay');
const songTitle = document.getElementById('song-title');
const songArtist = document.getElementById('song-artist');
const mapTypeBadge = document.getElementById('map-type-badge');
const songsPathInput = document.getElementById('songs-path-input');
const tosuPathInput = document.getElementById('tosu-path-input');
const overlayInstallBtn = document.getElementById('install-selected-overlays-btn');
const overlayRefreshBtn = document.getElementById('refresh-overlay-status-btn');
const overlayProgressFill = document.getElementById('overlay-progress-fill');
const overlayProgressLabel = document.getElementById('overlay-progress-label');
const overlayProgressCount = document.getElementById('overlay-progress-count');
const updateCurrentVersion = document.getElementById('update-current-version');
const updateLatestVersion = document.getElementById('update-latest-version');
const updateReleaseDate = document.getElementById('update-release-date');
const updateOverlayVersion = document.getElementById('update-overlay-version');
const updateAppStatusCopy = document.getElementById('update-app-status-copy');
const updateOverlayStatusCopy = document.getElementById('update-overlay-status-copy');
const checkUpdatesBtn = document.getElementById('check-updates-btn');
const updateAppBtn = document.getElementById('update-app-btn');
const updateOverlaysBtn = document.getElementById('update-overlays-btn');
const updateProgressFill = document.getElementById('update-progress-fill');
const updateProgressLabel = document.getElementById('update-progress-label');
const updateProgressCount = document.getElementById('update-progress-count');

const OVERLAY_NAMES = ['msdconverter', 'ManiaKeystrokes', 'HitCounter'];
let overlayInstallBusy = false;
let overlayListenerReady = false;
let updateCheckBusy = false;
let updateApplyBusy = false;
let latestUpdateStatus = null;

function getOverlayChoiceInputs() {
    return Array.from(document.querySelectorAll('.overlay-choice-input'));
}

function getSelectedOverlayNames() {
    return getOverlayChoiceInputs()
        .filter((input) => input.checked)
        .map((input) => input.getAttribute('data-overlay-name'))
        .filter(Boolean);
}

function loadOverlaySelections() {
    try {
        const saved = JSON.parse(localStorage.getItem('overlaySelections') || 'null');
        const selected = Array.isArray(saved) && saved.length ? new Set(saved) : new Set(OVERLAY_NAMES);
        getOverlayChoiceInputs().forEach((input) => {
            const overlayName = input.getAttribute('data-overlay-name');
            input.checked = selected.has(overlayName);
        });
    } catch {
        getOverlayChoiceInputs().forEach((input) => {
            input.checked = true;
        });
    }
}

function persistOverlaySelections() {
    localStorage.setItem('overlaySelections', JSON.stringify(getSelectedOverlayNames()));
}

function setOverlayProgress(message, overallDone = 0, overallTotal = 0) {
    if (overlayProgressLabel) overlayProgressLabel.textContent = message;
    if (overlayProgressCount) overlayProgressCount.textContent = `${overallDone} / ${overallTotal}`;
    if (overlayProgressFill) {
        const ratio = overallTotal > 0 ? Math.max(0, Math.min(1, overallDone / overallTotal)) : 0;
        overlayProgressFill.style.width = `${Math.round(ratio * 100)}%`;
    }
}

function setUpdateProgress(message, overallDone = 0, overallTotal = 0) {
    if (updateProgressLabel) updateProgressLabel.textContent = message;
    if (updateProgressCount) updateProgressCount.textContent = `${overallDone} / ${overallTotal}`;
    if (updateProgressFill) {
        const ratio = overallTotal > 0 ? Math.max(0, Math.min(1, overallDone / overallTotal)) : 0;
        updateProgressFill.style.width = `${Math.round(ratio * 100)}%`;
    }
}

function updateOverlayInstallButtonState() {
    const hasValidTosuPath = Boolean(state.config.tosu_root_path);
    const hasSelection = getSelectedOverlayNames().length > 0;
    if (overlayInstallBtn) {
        overlayInstallBtn.disabled = overlayInstallBusy || !hasValidTosuPath || !hasSelection;
    }
    if (overlayRefreshBtn) {
        overlayRefreshBtn.disabled = overlayInstallBusy;
    }
}

function updateAppButtonState() {
    if (checkUpdatesBtn) {
        checkUpdatesBtn.disabled = updateCheckBusy || updateApplyBusy;
        checkUpdatesBtn.textContent = updateCheckBusy ? 'Checking...' : 'Check Updates';
    }
    if (updateAppBtn) {
        updateAppBtn.disabled = updateCheckBusy || updateApplyBusy || !latestUpdateStatus?.app_update_available;
        updateAppBtn.textContent = updateApplyBusy ? 'Updating...' : 'Update App';
    }
    if (updateOverlaysBtn) {
        const canUpdateInstalledOverlays = Boolean(state.config.tosu_root_path)
            && !updateCheckBusy
            && !updateApplyBusy
            && getSelectedOverlayNames().length > 0
            && (latestUpdateStatus?.overlay_update_available || false)
            && !(latestUpdateStatus?.overlay_update_requires_app_update || false);
        updateOverlaysBtn.disabled = !canUpdateInstalledOverlays;
        updateOverlaysBtn.textContent = 'Update Installed Overlays';
    }
}

function setOverlayInstallBusy(busy) {
    overlayInstallBusy = busy;
    if (overlayInstallBtn) {
        overlayInstallBtn.textContent = busy ? 'Installing...' : 'Install Selected Overlays';
    }
    updateOverlayInstallButtonState();
    updateAppButtonState();
}

function updateOverlayStatusUI(overlayStatuses = []) {
    const statusMap = new Map((overlayStatuses || []).map((status) => [status.name, !!status.installed]));
    for (const overlayName of OVERLAY_NAMES) {
        const badge = document.getElementById(`overlay-status-${overlayName}`);
        if (!badge) continue;
        const status = (overlayStatuses || []).find((entry) => entry.name === overlayName);
        const installed = statusMap.get(overlayName) === true;
        const outdated = installed && status?.update_available === true;
        badge.textContent = outdated ? 'Update available' : installed ? 'Installed' : 'Pending';
        badge.className = `overlay-choice-status ${outdated ? 'outdated' : installed ? 'installed' : 'pending'}`;
    }
    setEditorTabAvailability(overlayStatuses);
}

function getSetupMessageFromBootstrap(bootstrap) {
    if (!bootstrap) {
        return 'Run the manager from anywhere, select your overlays, and install them into TOSU.';
    }

    const installedCount = (bootstrap.overlay_statuses || []).filter((status) => status.installed).length;
    const totalOverlays = (bootstrap.overlay_statuses || []).length;

    const missingParts = [];
    if (bootstrap.needs_tosu_folder || !state.config.tosu_root_path) missingParts.push('TOSU');
    if (bootstrap.needs_songs_folder || !state.config.osu_songs_path) missingParts.push('osu! Songs');

    if (missingParts.length > 0) {
        return `Auto-detect could not find ${missingParts.join(' and ')}. Pick it once, then install the overlays.`;
    }

    if (!bootstrap.overlays_installed) {
        if (installedCount > 0 && totalOverlays > 0) {
            return `${installedCount} of ${totalOverlays} overlays are installed. You can keep configuring the manager or install the remaining overlays into tosu/static/.`;
        }
        return 'TOSU is detected. Choose which overlays you want and install them directly into tosu/static/.';
    }

    return 'MSD calculations are being sent to TOSU. You can leave this window minimized.';
}

function applyBootstrapState(bootstrap) {
    if (!bootstrap) {
        updateOverlayInstallButtonState();
        updateAppButtonState();
        return;
    }

    updateOverlayStatusUI(bootstrap.overlay_statuses || []);
    updateOverlayInstallButtonState();

    const missingParts = [];
    if (bootstrap.needs_tosu_folder || !state.config.tosu_root_path) missingParts.push('TOSU');
    if (bootstrap.needs_songs_folder || !state.config.osu_songs_path) missingParts.push('osu! Songs');

    if (missingParts.length === 0 && bootstrap.overlays_installed) {
        showOverlay(getSetupMessageFromBootstrap(bootstrap));
    } else {
        showSetup(getSetupMessageFromBootstrap(bootstrap));
    }

    updateAppButtonState();
}

function formatPublishedDate(value) {
    if (!value) return 'Release date unavailable.';
    const parsed = new Date(value);
    if (Number.isNaN(parsed.getTime())) return value;
    return parsed.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
}

function applyUpdateStatus(status) {
    latestUpdateStatus = status || null;
    const currentVersion = status?.current_version || '0.3.0';
    const latestVersion = status?.latest_version ? `v${status.latest_version}` : 'Unknown';
    const overlayVersion = status?.overlay_runtime_version ? `v${status.overlay_runtime_version}` : 'Not installed';

    if (updateCurrentVersion) updateCurrentVersion.textContent = `v${currentVersion}`;
    if (updateLatestVersion) updateLatestVersion.textContent = latestVersion;
    if (updateReleaseDate) updateReleaseDate.textContent = status?.published_at ? `Published ${formatPublishedDate(status.published_at)}` : 'Release date unavailable.';
    if (updateOverlayVersion) updateOverlayVersion.textContent = overlayVersion;

    if (updateAppStatusCopy) {
        if (!status) {
            updateAppStatusCopy.textContent = 'Unable to fetch release information right now.';
        } else if (status.app_update_available) {
            updateAppStatusCopy.textContent = 'A newer release is available on GitHub. One click will download it and restart the app.';
        } else {
            updateAppStatusCopy.textContent = 'This app build already matches the latest GitHub release.';
        }
    }

    if (updateOverlayStatusCopy) {
        if (!status) {
            updateOverlayStatusCopy.textContent = 'Install overlays into tosu/static to track their version.';
        } else if (status.overlay_update_available) {
            updateOverlayStatusCopy.textContent = 'The overlays installed in tosu/static are older than this app build. You can update them now.';
        } else if (status.overlay_update_requires_app_update) {
            updateOverlayStatusCopy.textContent = 'Your installed overlays match this app, but GitHub has a newer app release with newer overlay files.';
        } else if (status.overlay_runtime_version) {
            updateOverlayStatusCopy.textContent = 'Installed overlays already match this app build.';
        } else {
            updateOverlayStatusCopy.textContent = 'Install overlays into tosu/static to track their version.';
        }
    }

    updateAppButtonState();
}

async function checkForUpdates({ silent = false } = {}) {
    if (!isTauri || updateCheckBusy) return null;
    updateCheckBusy = true;
    updateAppButtonState();
    if (!silent) {
        setUpdateProgress('Checking GitHub for updates...', 0, 0);
    }

    try {
        const startedAt = performance.now();
        const status = await invoke('check_for_updates');
        recordPerf('check_for_updates', performance.now() - startedAt, { mode: 'tauri' });
        applyUpdateStatus(status);
        if (!silent) {
            setUpdateProgress(
                status.app_update_available
                    ? `New release found: v${status.latest_version}.`
                    : 'You are already on the latest release.',
                status.app_update_available ? 1 : 0,
                1,
            );
        }
        return status;
    } catch (error) {
        console.error('Update check failed:', error);
        applyUpdateStatus(null);
        if (!silent) {
            setUpdateProgress(`Update check failed: ${String(error)}`, 0, 0);
        }
        return null;
    } finally {
        updateCheckBusy = false;
        updateAppButtonState();
    }
}

function initOverlayInstallerListeners() {
    loadOverlaySelections();
    getOverlayChoiceInputs().forEach((input) => {
        input.addEventListener('change', () => {
            persistOverlaySelections();
            updateOverlayInstallButtonState();
            updateAppButtonState();
        });
    });

    if (overlayInstallBtn) {
        overlayInstallBtn.addEventListener('click', async () => {
            const selectedOverlays = getSelectedOverlayNames();
            if (selectedOverlays.length === 0) {
                setOverlayProgress('Select at least one overlay to install.', 0, 0);
                updateOverlayInstallButtonState();
                return;
            }

            const tosuRoot = (tosuPathInput?.value || state.config.tosu_root_path || '').trim();
            if (!tosuRoot) {
                setOverlayProgress('Pick or detect the TOSU folder before installing overlays.', 0, 0);
                updateOverlayInstallButtonState();
                return;
            }

            await readAndSaveSetupSettings({ immediate: true, reason: 'before-overlay-install' });
            const isValidTosu = await invoke('validate_tosu_root', { path: tosuRoot });
            if (!isValidTosu) {
                setOverlayProgress('The selected TOSU folder is invalid. Fix it before installing overlays.', 0, 0);
                updateOverlayInstallButtonState();
                return;
            }

            setOverlayInstallBusy(true);
            setOverlayProgress('Preparing overlay installation...', 0, 0);

            try {
                await invoke('install_selected_overlays', {
                    tosuRoot,
                    overlays: selectedOverlays,
                });
                const bootstrap = await invoke('bootstrap_environment');
                if (bootstrap?.config) {
                    applyConfigSnapshot(bootstrap.config);
                    setPersistedConfigBaseline(bootstrap.config);
                }
                updateOverlayStatusUI(bootstrap?.overlay_statuses || []);
                setOverlayProgress('Overlay installation finished.', selectedOverlays.length, selectedOverlays.length);
                applyBootstrapState(bootstrap);
                await checkForUpdates({ silent: true });
            } catch (error) {
                console.error('Overlay install failed:', error);
                setOverlayProgress(`Overlay installation failed: ${String(error)}`, 0, 0);
            } finally {
                setOverlayInstallBusy(false);
            }
        });
    }

    if (overlayRefreshBtn) {
        overlayRefreshBtn.addEventListener('click', async () => {
            const bootstrap = await invoke('bootstrap_environment');
            if (bootstrap?.config) {
                applyConfigSnapshot(bootstrap.config);
                setPersistedConfigBaseline(bootstrap.config);
            }
            applyBootstrapState(bootstrap);
            await checkForUpdates({ silent: true });
        });
    }

    if (isTauri && !overlayListenerReady) {
        overlayListenerReady = true;
        window.__TAURI__.event.listen('overlay-install-progress', (event) => {
            const payload = event.payload || {};
            setOverlayProgress(
                payload.message || `Installing ${payload.overlay || 'overlay'}...`,
                Number(payload.overall_done || 0),
                Number(payload.overall_total || 0),
            );
        });
    }

    setOverlayProgress('Waiting to install overlays.', 0, 0);
    updateOverlayInstallButtonState();
}

function initUpdateListeners() {
    if (!isTauri) {
        if (checkUpdatesBtn) checkUpdatesBtn.disabled = true;
        if (updateAppBtn) updateAppBtn.disabled = true;
        if (updateOverlaysBtn) updateOverlaysBtn.disabled = true;
        setUpdateProgress('App updates are only available inside the desktop manager.', 0, 0);
        return;
    }

    if (checkUpdatesBtn) {
        checkUpdatesBtn.addEventListener('click', async () => {
            await checkForUpdates();
        });
    }

    if (updateAppBtn) {
        updateAppBtn.addEventListener('click', async () => {
            if (updateApplyBusy) return;
            updateApplyBusy = true;
            updateAppButtonState();
            setUpdateProgress('Preparing app update...', 0, 0);

            try {
                const message = await invoke('apply_latest_update');
                setUpdateProgress(message || 'The updater has been launched.', 1, 1);
            } catch (error) {
                console.error('App update failed:', error);
                setUpdateProgress(`App update failed: ${String(error)}`, 0, 0);
            } finally {
                updateApplyBusy = false;
                updateAppButtonState();
            }
        });
    }

    if (updateOverlaysBtn) {
        updateOverlaysBtn.addEventListener('click', async () => {
            const tosuRoot = (tosuPathInput?.value || state.config.tosu_root_path || '').trim();
            const overlays = getSelectedOverlayNames();

            if (!tosuRoot || overlays.length === 0) {
                setUpdateProgress('Pick a valid TOSU folder and keep at least one overlay selected.', 0, 0);
                updateAppButtonState();
                return;
            }

            setUpdateProgress('Updating installed overlays...', 0, overlays.length);
            setOverlayInstallBusy(true);
            try {
                await invoke('install_selected_overlays', { tosuRoot, overlays });
                const bootstrap = await invoke('bootstrap_environment');
                if (bootstrap?.config) {
                    applyConfigSnapshot(bootstrap.config);
                    setPersistedConfigBaseline(bootstrap.config);
                }
                applyBootstrapState(bootstrap);
                await checkForUpdates({ silent: true });
                setUpdateProgress('Installed overlays were updated to match this app build.', overlays.length, overlays.length);
            } catch (error) {
                console.error('Overlay update failed:', error);
                setUpdateProgress(`Overlay update failed: ${String(error)}`, 0, overlays.length);
            } finally {
                setOverlayInstallBusy(false);
            }
        });
    }

    if (isTauri && !window.__ORI_UPDATE_EVENT_WIRED__) {
        window.__ORI_UPDATE_EVENT_WIRED__ = true;
        window.__TAURI__.event.listen('app-update-progress', (event) => {
            const payload = event.payload || {};
            setUpdateProgress(
                payload.message || 'Updating application...',
                Number(payload.downloaded_bytes || 0),
                Number(payload.total_bytes || 0),
            );
        });
    }

    setUpdateProgress('Waiting to check for updates.', 0, 0);
    updateAppButtonState();
}

function applySavedTheme() {
    const savedTheme = localStorage.getItem('theme') === 'dark' ? 'dark' : 'light';
    document.documentElement.dataset.theme = savedTheme;
    if (document.body) {
        document.body.dataset.theme = savedTheme;
    }
    if (setupScreen) {
        setupScreen.classList.toggle('dark-mode', savedTheme === 'dark');
    }
}

function handleStartupError(error) {
    console.error('Startup failed:', error);
    if (setupScreen) setupScreen.classList.remove('hidden');
    if (overlay) overlay.classList.add('hidden');

    const descEl = document.querySelector('.setup-desc');
    if (descEl) {
        descEl.textContent = 'Startup failed. Open the developer console to inspect the error, then try again.';
    }

    const statusEl = document.getElementById('tosu-status-val');
    if (statusEl) {
        statusEl.textContent = 'Error';
        statusEl.className = 'status-pill error';
    }
}

async function init() {
    const startupStartedAt = performance.now();
    if (setupScreen) setupScreen.classList.remove('hidden');
    applySavedTheme();
    initChart('radar-chart');
    initOverlayInstallerListeners();
    initUpdateListeners();
    
    if (isTauri) {
        applyConfigSnapshot(await loadDefaultConfig());

        const loadStartedAt = performance.now();
        const loadedConfig = await invoke('get_config');
        applyConfigSnapshot(loadedConfig);
        recordPerf('get_config', performance.now() - loadStartedAt, { mode: 'tauri' });

        const bootstrapStartedAt = performance.now();
        const bootstrap = await invoke('bootstrap_environment');
        if (bootstrap && bootstrap.config) {
            applyConfigSnapshot(bootstrap.config);
            setPersistedConfigBaseline(bootstrap.config);
        } else {
            setPersistedConfigBaseline(state.config);
        }
        recordPerf('bootstrap_environment', performance.now() - bootstrapStartedAt, { mode: 'tauri' });
        applyVisualSettings();
        ensureHitCounterConfig();
        
        populateSettingsPanel();
        initHitCounterEditor();
        
        const guideBtn = document.getElementById('setup-guide-btn');
        if (guideBtn && !state.config.has_run_before) {
            guideBtn.classList.add('pulse-glow');
        }

        if (songsPathInput && state.config.osu_songs_path) {
            songsPathInput.value = state.config.osu_songs_path;
        }
        if (tosuPathInput && state.config.tosu_root_path) {
            tosuPathInput.value = state.config.tosu_root_path;
        }

        if (state.config.osu_songs_path) {
            validatePath(state.config.osu_songs_path);
        }
        if (state.config.tosu_root_path) {
            validateTosuPath(state.config.tosu_root_path);
        }
        applyBootstrapState(bootstrap);
        void checkForUpdates({ silent: true });
    } else {
        applyConfigSnapshot(await loadDefaultConfig());
        try {
            const res = await fetch(`config.json?t=${Date.now()}`);
            if (res.ok) {
                const loadedConfig = await res.json();
                applyConfigSnapshot(loadedConfig);
            }
        } catch (e) {
            console.warn("Could not fetch config.json in OBS", e);
        }
        setPersistedConfigBaseline(state.config);
        applyVisualSettings();
        ensureHitCounterConfig();
        initHitCounterEditor();
        showOverlay();
    }
    connectToTosu();
    recordPerf('startup_total', performance.now() - startupStartedAt, { mode: isTauri ? 'tauri' : 'browser' });
}

async function validatePath(path) {
    const msgEl = document.getElementById('path-validation-msg');
    const pathStatusVal = document.getElementById('path-status-val');
    
    if (!path) {
        if (msgEl) msgEl.textContent = '';
        state.config.osu_songs_path = '';
        return;
    }
    
    const isValid = await invoke('validate_path', { path });
    if (isValid) {
        if (msgEl) {
            msgEl.textContent = 'Valid osu! Songs folder';
            msgEl.className = 'validation-msg valid';
        }
        if (pathStatusVal) {
            pathStatusVal.textContent = 'Valid';
            pathStatusVal.className = 'status-pill ok';
        }
    } else {
        if (msgEl) {
            msgEl.textContent = 'Invalid path or folder does not exist';
            msgEl.className = 'validation-msg';
        }
        if (pathStatusVal) {
            pathStatusVal.textContent = 'Invalid';
            pathStatusVal.className = 'status-pill error';
        }
    }
}

async function validateTosuPath(path) {
    const msgEl = document.getElementById('tosu-validation-msg');

    if (!path) {
        if (msgEl) msgEl.textContent = '';
        state.config.tosu_root_path = '';
        updateOverlayInstallButtonState();
        return;
    }

    const isValid = await invoke('validate_tosu_root', { path });
    if (isValid) {
        if (msgEl) {
            msgEl.textContent = 'Valid TOSU folder';
            msgEl.className = 'validation-msg valid';
        }
        state.config.tosu_root_path = path;
    } else {
        if (msgEl) {
            msgEl.textContent = 'Invalid TOSU folder or tosu.exe not found';
            msgEl.className = 'validation-msg';
        }
        state.config.tosu_root_path = '';
    }
    updateOverlayInstallButtonState();
}

function connectToTosu() {
    setupTosuConnection({
        onData: (data) => handleTosuData(data),
        onStatus: (status) => {
            if (status === 'connected') {
                setDot('connected');
                if (state.config.osu_songs_path) showOverlay();
            } else {
                setDot('disconnected');
                if (state.config.tosu_root_path && state.config.osu_songs_path) {
                    showSetup('TOSU is not running yet. Start it and the manager will connect automatically.');
                } else {
                    showSetup();
                }
            }
        },
        onMsdCalculated: (ratings) => {
            state.currentRatings = ratings;
            updateUI(ratings);
            setDot('connected');
            showError('');
        }
    });
}

function handleTosuData(data) {
    let folder = '';
    let file = '';
    let title = '';
    let artist = '';

    if (data.menu?.bm) {
        const pathObj = data.menu.bm.path || {};
        folder = pathObj.folder || '';
        file = pathObj.file || '';
        title = data.menu.bm.metadata?.title || data.menu.bm.metadata?.titleUnicode || '';
        artist = data.menu.bm.metadata?.artist || data.menu.bm.metadata?.artistUnicode || '';
    }

    if ((!folder || !file) && data.beatmap) {
        const pathObj = data.beatmap.path || {};
        folder = pathObj.folder || '';
        file = pathObj.file || '';
        title = title || data.beatmap.title || '';
        artist = artist || data.beatmap.artist || '';
    }

    if (file && file.includes('||')) {
        file = file.split('||')[1];
    }

    if (title && songTitle) songTitle.textContent = title;
    if (artist && songArtist) songArtist.textContent = artist;

    let mode = data.menu?.bm?.mode ?? data.gameplay?.mode;
    let cs = data.menu?.bm?.stats?.CS;

    if (mode === undefined && data.beatmap) {
        mode = data.beatmap.mode;
        cs = data.beatmap.cs;
    }

    if (mode !== undefined) {
        if (Number(mode) !== 3) {
            showError("Only osu!mania maps are supported.");
            state.isMapValid = false;
        } else if (cs !== undefined && Number(cs) !== 4) {
            showError("Only 4K maps are supported.");
            state.isMapValid = false;
        } else {
            showError('');
            state.isMapValid = true;
        }
    }

    if (!state.isMapValid) return;

    const hits = data.gameplay?.hits || {};
    const maxEl = document.getElementById('val-max');
    if (maxEl) {
        maxEl.textContent = hits.geki || 0;
        document.getElementById('val-perf').textContent = hits['300'] || 0;
        document.getElementById('val-great').textContent = hits.katu || 0;
        document.getElementById('val-good').textContent = hits['100'] || 0;
        document.getElementById('val-bad').textContent = hits['50'] || 0;
        document.getElementById('val-miss').textContent = hits['0'] || 0;
    }

    const id = data.menu?.bm?.id || data.beatmap?.id || '';
    const md5 = data.menu?.bm?.md5 || data.beatmap?.md5 || '';
    const modsStr = data.menu?.mods?.str || data.beatmap?.mods || '';
    const useKey = md5 || id;
    const mapKey = `${useKey}||${modsStr}`;
    
    if (useKey && mapKey !== state.lastCalculatedKey) {
        state.currentMapKey = mapKey;
        state.lastCalculatedKey = mapKey;
        if (!isTauri) {
            const myId = ++state.calcId;
            triggerCalc(folder, file, mapKey, myId);
        } else {
            setDot('calculating');
        }
    }
}

async function triggerCalc(folder, file, mapKey, id) {
    if (id !== state.calcId) return;

    setDot('calculating');

    if (!state.waitingForCalcSince) state.waitingForCalcSince = Date.now();
    try {
        const res = await fetch(`msd.json?t=${Date.now()}`);
        if (res.ok) {
            const data = await res.json();
            if (data.map_key === mapKey) {
                if (data.error) {
                    showError(data.error);
                    state.waitingForCalcSince = 0;
                    setDot('connected');
                    return;
                }
                const ratings = data.ratings;
                state.currentRatings = ratings;
                updateUI(ratings);
                setDot('connected');
                showError('');
                state.waitingForCalcSince = 0;
            } else {
                setTimeout(() => triggerCalc(folder, file, mapKey, id), 50);
                return;
            }
        }
    } catch (e) {
        setTimeout(() => triggerCalc(folder, file, mapKey, id), 100);
    }
}

function updateUI(r) {
    try {
        applyHitCounterToDocument();
        document.getElementById('val-overall').textContent = r.overall.toFixed(2);
        document.getElementById('val-stream').textContent = r.stream.toFixed(2);
        document.getElementById('val-jumpstream').textContent = r.jumpstream.toFixed(2);
        document.getElementById('val-handstream').textContent = r.handstream.toFixed(2);
        document.getElementById('val-stamina').textContent = r.stamina.toFixed(2);
        document.getElementById('val-jackspeed').textContent = r.jackspeed.toFixed(2);
        document.getElementById('val-chordjack').textContent = r.chordjack.toFixed(2);
        document.getElementById('val-technical').textContent = r.technical.toFixed(2);

        updateMapTypeBadge(r.map_type);
        drawRadar(r);
    } catch (e) {
        showError(`UI Error: ${e.message}`);
    }
}

function resetUI() {
    const ids = ['val-overall', 'val-stream', 'val-jumpstream', 'val-handstream',
                  'val-stamina', 'val-jackspeed', 'val-chordjack', 'val-technical'];
    ids.forEach(id => {
        const el = document.getElementById(id);
        if (el) el.textContent = '-';
    });
    if (mapTypeBadge) mapTypeBadge.classList.add('hidden');
    clearRadar();
}

function showError(msg) {
    const bar = document.getElementById('debug-bar');
    const txt = document.getElementById('debug-msg');
    if (msg) {
        if (txt) txt.textContent = msg;
        if (bar) bar.classList.remove('hidden');
    } else {
        if (bar) bar.classList.add('hidden');
        if (txt) txt.textContent = '';
    }
}

function updateMapTypeBadge(mapType) {
    if (!mapTypeBadge) return;
    mapTypeBadge.textContent = mapType;
    mapTypeBadge.className = 'map-type-badge';
    if (mapType.startsWith('Speed')) mapTypeBadge.classList.add('speed');
    else if (mapType.startsWith('Jack')) mapTypeBadge.classList.add('jack');
    else if (mapType.startsWith('Stamina')) mapTypeBadge.classList.add('stamina');
    else if (mapType.startsWith('Tech')) mapTypeBadge.classList.add('tech');
    mapTypeBadge.classList.remove('hidden');
}

function setDot(state) {
    const connectionDot = document.getElementById('connection-dot');
    const tosuStatusVal = document.getElementById('tosu-status-val');
    
    if (connectionDot) connectionDot.className = `dot ${state}`;
    
    if (tosuStatusVal) {
        if (state === 'connected') {
            tosuStatusVal.textContent = 'Connected';
            tosuStatusVal.className = 'status-pill ok';
        } else if (state === 'calculating') {
            tosuStatusVal.textContent = 'Calculating';
            tosuStatusVal.className = 'status-pill warning';
        } else {
            tosuStatusVal.textContent = 'Disconnected';
            tosuStatusVal.className = 'status-pill error';
        }
    }
}

function showOverlay(msg) {
    if (isTauri) {
        if (setupScreen) setupScreen.classList.remove('hidden');
        if (overlay) overlay.classList.add('hidden');
        const titleEl = document.querySelector('.setup-title');
        if (titleEl) titleEl.textContent = "Server is Running";
        const descEl = document.querySelector('.setup-desc');
        if (descEl) descEl.textContent = msg || "MSD calculations are being sent to TOSU. You can leave this window minimized.";
    } else {
        if (setupScreen) setupScreen.classList.add('hidden');
        if (overlay) overlay.classList.remove('hidden');
    }
}

function showSetup(msg) {
    if (overlay) overlay.classList.add('hidden');
    if (setupScreen) setupScreen.classList.remove('hidden');
    if (msg) {
        const descEl = document.querySelector('.setup-desc');
        if (descEl) descEl.textContent = msg;
    }
}

window.handleTosuData = handleTosuData;

initUIListeners({
    onValidatePath: validatePath,
    onValidateTosuPath: validateTosuPath,
    onConnectTosu: connectToTosu,
    onBootstrapState: applyBootstrapState,
});
initWindowControls();
init().catch(handleStartupError);
