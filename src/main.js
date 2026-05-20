import { state } from './state.js';
import { setupTosuConnection } from './api.js';
import { applyVisualSettings, populateSettingsPanel } from './config.js';
import { initUIListeners, initWindowControls } from './events.js';
import { drawRadar, clearRadar, initChart } from './chart.js';

const isTauri = typeof window.__TAURI__ !== 'undefined';
const invoke = isTauri ? window.__TAURI__.core.invoke : async () => {};

const setupScreen = document.getElementById('setup-screen');
const overlay = document.getElementById('overlay');
const songTitle = document.getElementById('song-title');
const songArtist = document.getElementById('song-artist');
const mapTypeBadge = document.getElementById('map-type-badge');
const songsPathInput = document.getElementById('songs-path-input');

async function init() {
    initChart('radar-chart');
    
    if (isTauri) {
        const loadedConfig = await invoke('get_config');
        Object.assign(state.config, loadedConfig);
        applyVisualSettings();
        
        populateSettingsPanel();
        
        const guideBtn = document.getElementById('setup-guide-btn');
        if (guideBtn && !state.config.has_run_before) {
            guideBtn.classList.add('pulse-glow');
        }
        
        if (state.config.osu_songs_path) {
            if (songsPathInput) songsPathInput.value = state.config.osu_songs_path;
            validatePath(state.config.osu_songs_path);
            showOverlay();
        } else {
            showSetup('Set your osu! Songs folder path so the overlay can find and calculate maps.');
            const defPath = await invoke('get_default_osu_path');
            if (defPath) {
                if (songsPathInput) {
                    songsPathInput.placeholder = defPath;
                    songsPathInput.value = defPath;
                }
                validatePath(defPath);
            }
        }
    } else {
        try {
            const res = await fetch(`config.json?t=${Date.now()}`);
            if (res.ok) {
                const loadedConfig = await res.json();
                Object.assign(state.config, loadedConfig);
            }
        } catch (e) {
            console.warn("Could not fetch config.json in OBS", e);
        }
        applyVisualSettings();
        showOverlay();
    }
    connectToTosu();
}

async function validatePath(path) {
    const msgEl = document.getElementById('path-validation-msg');
    const pathStatusVal = document.getElementById('path-status-val');
    
    if (!path) {
        if (msgEl) msgEl.textContent = '';
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

function connectToTosu() {
    setupTosuConnection({
        onData: (data) => handleTosuData(data),
        onStatus: (status) => {
            if (status === 'connected') {
                setDot('connected');
                if (state.config.osu_songs_path) showOverlay();
            } else {
                setDot('disconnected');
                showSetup();
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

function showOverlay() {
    if (isTauri) {
        if (setupScreen) setupScreen.classList.remove('hidden');
        if (overlay) overlay.classList.add('hidden');
        const titleEl = document.querySelector('.setup-title');
        if (titleEl) titleEl.textContent = "Server is Running";
        const descEl = document.querySelector('.setup-desc');
        if (descEl) descEl.textContent = "MSD calculations are being sent to TOSU. You can leave this window minimized.";
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
    onConnectTosu: connectToTosu
});
initWindowControls();
init();
