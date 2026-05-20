import { state } from './state.js';
import { saveVisualSettings, populateSettingsPanel } from './config.js';
import { setupTosuConnection } from './api.js';

const isTauri = typeof window.__TAURI__ !== 'undefined';
const invoke = isTauri ? window.__TAURI__.core.invoke : async () => {};

let listeningForKeyIndex = null;

export function initUIListeners(callbacks = {}) {
    const { onValidatePath, onConnectTosu } = callbacks;
    const opacitySlider = document.getElementById('setup-opacity');
    const scaleSlider = document.getElementById('setup-scale');
    const radarToggle = document.getElementById('setup-radar');
    const particlesToggle = document.getElementById('setup-particles');
    
    if (opacitySlider) {
        opacitySlider.addEventListener('input', (e) => {
            state.config.bg_opacity = parseFloat(e.target.value);
            document.documentElement.style.setProperty('--bg-opacity', state.config.bg_opacity);
            readAndSaveSetupSettings();
        });
    }
    
    if (scaleSlider) {
        scaleSlider.addEventListener('input', (e) => {
            state.config.scale = parseFloat(e.target.value);
            document.documentElement.style.setProperty('--ui-scale', state.config.scale);
            readAndSaveSetupSettings();
        });
    }

    if (particlesToggle) {
        particlesToggle.addEventListener('change', (e) => {
            state.config.show_particles = e.target.checked;
            readAndSaveSetupSettings();
        });
    }

    const hitOpacitySlider = document.getElementById('setup-hit-opacity');
    const hitScaleSlider = document.getElementById('setup-hit-scale');
    const hitBgColorInput = document.getElementById('setup-hit-bg-color');
    const hitTextColorInput = document.getElementById('setup-hit-text-color');
    const hitBorderStyleSelect = document.getElementById('setup-hit-border-style');
    const hitOrientationSelect = document.getElementById('setup-hit-orientation');
    
    if (hitOpacitySlider) {
        hitOpacitySlider.addEventListener('input', (e) => {
            state.config.hitcounter_opacity = parseFloat(e.target.value);
            readAndSaveSetupSettings();
        });
    }
    
    if (hitScaleSlider) {
        hitScaleSlider.addEventListener('input', (e) => {
            state.config.hitcounter_scale = parseFloat(e.target.value);
            readAndSaveSetupSettings();
        });
    }

    if (hitBgColorInput) {
        hitBgColorInput.addEventListener('input', (e) => {
            state.config.hitcounter_bg_color = e.target.value;
            readAndSaveSetupSettings();
        });
    }

    if (hitTextColorInput) {
        hitTextColorInput.addEventListener('input', (e) => {
            state.config.hitcounter_text_color = e.target.value;
            readAndSaveSetupSettings();
        });
    }

    if (hitBorderStyleSelect) {
        hitBorderStyleSelect.addEventListener('change', (e) => {
            state.config.hitcounter_border_style = e.target.value;
            readAndSaveSetupSettings();
        });
    }

    if (hitOrientationSelect) {
        hitOrientationSelect.addEventListener('change', (e) => {
            state.config.hitcounter_orientation = e.target.value;
            readAndSaveSetupSettings();
        });
    }

    if (radarToggle) {
        radarToggle.addEventListener('change', (e) => {
            state.config.show_radar = e.target.checked;
            const chartWrap = document.querySelector('.chart-wrap');
            if (chartWrap) {
                if (state.config.show_radar) {
                    chartWrap.classList.remove('hidden');
                } else {
                    chartWrap.classList.add('hidden');
                }
            }
            readAndSaveSetupSettings();
        });
    }

    const colorOuterEl = document.getElementById('setup-key-color-outer');
    const colorInnerEl = document.getElementById('setup-key-color-inner');
    const sizeSlider = document.getElementById('setup-key-size');
    const gapSlider = document.getElementById('setup-key-gap');
    
    if (colorOuterEl) {
        colorOuterEl.addEventListener('input', () => readAndSaveSetupSettings());
    }
    if (colorInnerEl) {
        colorInnerEl.addEventListener('input', () => readAndSaveSetupSettings());
    }
    if (sizeSlider) {
        sizeSlider.addEventListener('input', () => readAndSaveSetupSettings());
    }
    if (gapSlider) {
        gapSlider.addEventListener('input', () => readAndSaveSetupSettings());
    }

    const trailsToggle = document.getElementById('setup-trails');
    const trailOpacitySlider = document.getElementById('setup-trail-opacity');
    
    if (trailsToggle) {
        trailsToggle.addEventListener('change', () => readAndSaveSetupSettings());
    }
    if (trailOpacitySlider) {
        trailOpacitySlider.addEventListener('input', () => readAndSaveSetupSettings());
    }

    const trailFadeSlider = document.getElementById('setup-trail-fade');
    if (trailFadeSlider) {
        trailFadeSlider.addEventListener('input', () => readAndSaveSetupSettings());
    }

    const bgOpacitySlider = document.getElementById('setup-bg-opacity');
    if (bgOpacitySlider) {
        bgOpacitySlider.addEventListener('input', () => readAndSaveSetupSettings());
    }

    const trailHeightSlider = document.getElementById('setup-trail-height-slider');
    const trailHeightNum = document.getElementById('setup-trail-height-num');
    if (trailHeightSlider && trailHeightNum) {
        trailHeightSlider.addEventListener('input', (e) => {
            trailHeightNum.value = e.target.value;
            readAndSaveSetupSettings();
        });
        trailHeightNum.addEventListener('input', (e) => {
            trailHeightSlider.value = e.target.value;
            readAndSaveSetupSettings();
        });
    }

    const basicInputs = [
        'setup-bg-color', 'setup-trail-speed', 'setup-rgb-speed'
    ];
    basicInputs.forEach(id => {
        const el = document.getElementById(id);
        if (el) el.addEventListener('input', () => readAndSaveSetupSettings());
    });

    for (let i = 0; i < 4; i++) {
        const lbl = document.getElementById(`setup-key-label-${i}`);
        if (lbl) lbl.addEventListener('input', () => readAndSaveSetupSettings());

        const offX = document.getElementById(`setup-key-offset-x-${i}`);
        const offY = document.getElementById(`setup-key-offset-y-${i}`);
        if (offX) offX.addEventListener('input', () => readAndSaveSetupSettings());
        if (offY) offY.addEventListener('input', () => readAndSaveSetupSettings());

        const rgbChk = document.getElementById(`setup-rgb-key-${i}`);
        if (rgbChk) rgbChk.addEventListener('change', () => readAndSaveSetupSettings());

        const trWidth = document.getElementById(`setup-trail-width-${i}`);
        if (trWidth) trWidth.addEventListener('input', () => readAndSaveSetupSettings());

        const trOffset = document.getElementById(`setup-trail-offset-${i}`);
        if (trOffset) trOffset.addEventListener('input', () => readAndSaveSetupSettings());
    }

    const lockTrailsChk = document.getElementById('setup-lock-trails');
    if (lockTrailsChk) lockTrailsChk.addEventListener('change', () => {
        readAndSaveSetupSettings();
        const group = document.getElementById('trail-offsets-group');
        if (group) group.style.display = lockTrailsChk.checked ? 'none' : 'flex';
    });

    const grid = document.querySelector('.checkbox-grid');
    if (grid) {
        grid.addEventListener('click', (e) => {
            const pill = e.target.closest('.pill-btn');
            if (pill) {
                const skill = pill.id.replace('btn-', '');
                const isActive = pill.classList.toggle('active');
                
                if (!state.config.visible_skills) state.config.visible_skills = {};
                state.config.visible_skills[skill] = isActive;
                
                readAndSaveSetupSettings();
            }
        });
    }

    for (let i = 0; i < 4; i++) {
        const input = document.getElementById(`setup-key-${i}`);
        if (input) {
            input.addEventListener('click', () => {
                for (let j = 0; j < 4; j++) {
                    const otherInput = document.getElementById(`setup-key-${j}`);
                    if (otherInput && j !== i && otherInput.classList.contains('listening')) {
                        if (state.config.keys && state.config.keys[j]) {
                            otherInput.value = state.config.keys[j].replace('Key', '');
                        }
                        otherInput.classList.remove('listening');
                    }
                }
                input.value = "Press a key...";
                input.classList.add('listening');
                listeningForKeyIndex = i;
            });
        }
    }

    const exploreBtn = document.getElementById('explore-btn');
    if (exploreBtn) {
        exploreBtn.addEventListener('click', async () => {
            const folder = await invoke('select_folder');
            if (folder) {
                const songsPathInput = document.getElementById('songs-path-input');
                if (songsPathInput) {
                    songsPathInput.value = folder;
                    if (onValidatePath) onValidatePath(folder);
                }
            }
        });
    }

    const songsPathInput = document.getElementById('songs-path-input');
    if (songsPathInput) {
        songsPathInput.addEventListener('input', (e) => {
            if (onValidatePath) onValidatePath(e.target.value.trim());
        });
    }

    const retryBtn = document.getElementById('retry-btn');
    if (retryBtn) {
        retryBtn.addEventListener('click', async () => {
            const descEl = document.querySelector('.setup-desc');
            descEl.textContent = 'Checking if tosu is running...';
            const isRunning = await invoke('check_tosu', { port: state.config.tosu_port });
            if (isRunning) {
                descEl.textContent = 'tosu detected! Connecting...';
                if (onConnectTosu) onConnectTosu();
            } else {
                descEl.textContent = 'tosu is still not running. Please start it and try again.';
            }
        });
    }

    const saveConfigBtn = document.getElementById('save-config-btn');
    if (saveConfigBtn) {
        saveConfigBtn.addEventListener('click', async () => {
            await readAndSaveSetupSettings();
        });
    }

    const downloadTosuBtn = document.getElementById('download-tosu-btn');
    if (downloadTosuBtn) {
        downloadTosuBtn.addEventListener('click', (e) => {
            e.preventDefault();
            invoke('open_url', { url: 'https://tosu.app/' });
        });
    }

    document.querySelectorAll('.copiable-link').forEach(link => {
        link.addEventListener('click', async (e) => {
            const url = link.getAttribute('data-url');
            try {
                await navigator.clipboard.writeText(url);
            } catch (err) {
                console.error("Could not copy:", err);
            }
            if (isTauri) {
                invoke('open_url', { url }).catch(() => {});
            } else {
                window.open(url, '_blank');
            }
            
            let toast = document.getElementById('autosave-toast');
            if (!toast) {
                toast = document.createElement('div');
                toast.id = 'autosave-toast';
                toast.className = 'autosave-toast';
                document.body.appendChild(toast);
            }
            toast.textContent = 'Copied to clipboard & opened!';
            toast.classList.add('visible');
            clearTimeout(window._toastTimeout);
            window._toastTimeout = setTimeout(() => {
                toast.classList.remove('visible');
            }, 2000);
        });
    });

    const guideBtn = document.getElementById('setup-guide-btn');
    const guideModal = document.getElementById('guide-modal');
    const guideClose = document.querySelector('.guide-modal-close');

    if (guideBtn && guideModal) {
        guideBtn.addEventListener('click', async () => {
            guideModal.classList.add('visible');
            guideBtn.classList.remove('pulse-glow');
            if (state.config && !state.config.has_run_before) {
                state.config.has_run_before = true;
                await readAndSaveSetupSettings();
            }
        });
    }

    if (guideClose && guideModal) {
        guideClose.addEventListener('click', () => {
            guideModal.classList.remove('visible');
        });
        
        guideModal.addEventListener('click', (e) => {
            if (e.target === guideModal) {
                guideModal.classList.remove('visible');
            }
        });

        window.addEventListener('keydown', (e) => {
            if (e.key === 'Escape' && guideModal.classList.contains('visible')) {
                guideModal.classList.remove('visible');
            }
        });
    }

    let keyWs = null;
    function connectKeyWs() {
        keyWs = new WebSocket('ws://127.0.0.1:24051');
        keyWs.onmessage = (event) => {
            const data = JSON.parse(event.data);
            if (data.event === 'key-down' && listeningForKeyIndex !== null) {
                const keyStr = data.key;
                const input = document.getElementById(`setup-key-${listeningForKeyIndex}`);
                if (input) {
                    input.value = keyStr.replace('Key', '');
                    input.classList.remove('listening');
                    if (!state.config.keys) state.config.keys = ['KeyD', 'KeyF', 'KeyJ', 'KeyK'];
                    state.config.keys[listeningForKeyIndex] = keyStr;
                    readAndSaveSetupSettings();
                }
                listeningForKeyIndex = null;
            }
        };
        keyWs.onclose = () => {
            setTimeout(connectKeyWs, 2000);
        };
    }
    connectKeyWs();



    window.addEventListener('keydown', (e) => {
        if (listeningForKeyIndex !== null) {
            e.preventDefault();
            const keyStr = e.code;
            const input = document.getElementById(`setup-key-${listeningForKeyIndex}`);
            if (input) {
                input.value = keyStr.replace('Key', '');
                input.classList.remove('listening');
                if (!state.config.keys) state.config.keys = ['KeyD', 'KeyF', 'KeyJ', 'KeyK'];
                state.config.keys[listeningForKeyIndex] = keyStr;
                readAndSaveSetupSettings();
            }
            listeningForKeyIndex = null;
        }
    });

    const resetKeysBtn = document.getElementById('setup-keys-reset-btn');
    if (resetKeysBtn) {
        resetKeysBtn.addEventListener('click', async () => {
            state.config.key_color_outer = "#00d2ff";
            state.config.key_color_inner = "#ff007f";
            state.config.key_size = 60;
            state.config.key_gap = 10;
            state.config.show_trails = true;
            state.config.trail_opacity = 0.6;
            state.config.trail_fade = 0.0;
            state.config.trail_speed = 6.0;
            state.config.trail_height = 800;
            state.config.trail_widths = [50, 50, 50, 50];
            state.config.key_labels = ["D", "F", "J", "K"];
            state.config.rgb_enabled_keys = [false, false, false, false];
            state.config.rgb_speed = 1.0;
            state.config.keys_bg_color = "#0a0a12";
            state.config.keys_bg_opacity = 0.7;
            state.config.key_offsets_x = [0, 0, 0, 0];
            state.config.key_offsets_y = [0, 0, 0, 0];
            state.config.lock_trails = true;
            state.config.trail_offsets_x = [0, 0, 0, 0];
            state.config.keys = ["KeyD", "KeyF", "KeyJ", "KeyK"];
            
            populateSettingsPanel();
            await readAndSaveSetupSettings();
        });
    }

    initTheme();
    initTabs();
}

function initTheme() {
    const themeBtn = document.getElementById('theme-toggle-btn');
    const setupScreen = document.getElementById('setup-screen');
    
    if (themeBtn && setupScreen) {
        themeBtn.addEventListener('click', () => {
            setupScreen.classList.toggle('dark-mode');
            
            if (setupScreen.classList.contains('dark-mode')) {
                themeBtn.textContent = 'Light Mode';
            } else {
                themeBtn.textContent = 'Dark Mode';
            }
            
            localStorage.setItem('theme', setupScreen.classList.contains('dark-mode') ? 'dark' : 'light');
        });
        
        const savedTheme = localStorage.getItem('theme');
        if (savedTheme === 'dark') {
            setupScreen.classList.add('dark-mode');
            themeBtn.textContent = 'Light Mode';
        }
    }
}

function initTabs() {
    const tabButtons = document.querySelectorAll('.tab-btn');
    const tabPanes = document.querySelectorAll('.tab-pane');

    tabButtons.forEach(btn => {
        btn.addEventListener('click', () => {
            const tabId = btn.getAttribute('data-tab');
            
            tabButtons.forEach(b => b.classList.remove('active'));
            tabPanes.forEach(p => p.classList.remove('active'));
            
            btn.classList.add('active');
            const targetPane = document.getElementById(`${tabId}-tab`);
            if (targetPane) targetPane.classList.add('active');
        });
    });
}



async function readAndSaveSetupSettings() {
    try {
        const songsPathInput = document.getElementById('songs-path-input');
        const path = songsPathInput ? songsPathInput.value.trim() : '';
        state.config.osu_songs_path = path;
        
        if (!state.config.keys) state.config.keys = ["KeyD", "KeyF", "KeyJ", "KeyK"];
        for (let i = 0; i < 4; i++) {
            const input = document.getElementById(`setup-key-${i}`);
            if (input) {
                const val = input.value.trim();
                if (val && val !== "Presione...") {
                    state.config.keys[i] = val.startsWith('Key') ? val : `Key${val}`;
                }
            }
        }

        const opacityEl = document.getElementById('setup-opacity');
        const accentEl = document.getElementById('setup-accent');
        const scaleEl = document.getElementById('setup-scale');
        const radarEl = document.getElementById('setup-radar');
        const particlesEl = document.getElementById('setup-particles');
        const hitOpacityEl = document.getElementById('setup-hit-opacity');
        const hitScaleEl = document.getElementById('setup-hit-scale');
        const hitBgColorEl = document.getElementById('setup-hit-bg-color');
        const hitTextColorEl = document.getElementById('setup-hit-text-color');
        const hitBorderStyleEl = document.getElementById('setup-hit-border-style');
        const hitOrientationEl = document.getElementById('setup-hit-orientation');
        
        if (opacityEl) state.config.bg_opacity = parseFloat(opacityEl.value);
        if (accentEl) state.config.accent_color = accentEl.value;
        if (scaleEl) state.config.scale = parseFloat(scaleEl.value);
        if (radarEl) state.config.show_radar = radarEl.checked;
        if (hitOpacityEl) state.config.hitcounter_opacity = parseFloat(hitOpacityEl.value);
        if (hitScaleEl) state.config.hitcounter_scale = parseFloat(hitScaleEl.value);
        if (hitBgColorEl) state.config.hitcounter_bg_color = hitBgColorEl.value;
        if (hitTextColorEl) state.config.hitcounter_text_color = hitTextColorEl.value;
        if (hitBorderStyleEl) state.config.hitcounter_border_style = hitBorderStyleEl.value;
        if (hitOrientationEl) state.config.hitcounter_orientation = hitOrientationEl.value;
        if (particlesEl) state.config.show_particles = particlesEl.checked;
        
        const colorOuterEl = document.getElementById('setup-key-color-outer');
        const colorInnerEl = document.getElementById('setup-key-color-inner');
        const sizeEl = document.getElementById('setup-key-size');
        const gapEl = document.getElementById('setup-key-gap');
        
        if (colorOuterEl) state.config.key_color_outer = colorOuterEl.value;
        if (colorInnerEl) state.config.key_color_inner = colorInnerEl.value;
        if (sizeEl) state.config.key_size = parseInt(sizeEl.value);
        if (gapEl) state.config.key_gap = parseInt(gapEl.value);

        const trailsEl = document.getElementById('setup-trails');
        const trailOpacityEl = document.getElementById('setup-trail-opacity');
        
        if (trailsEl) state.config.show_trails = trailsEl.checked;
        if (trailOpacityEl) state.config.trail_opacity = parseFloat(trailOpacityEl.value);

        const trailFadeEl = document.getElementById('setup-trail-fade');
        if (trailFadeEl) state.config.trail_fade = parseFloat(trailFadeEl.value);

        const bgOpacityEl = document.getElementById('setup-bg-opacity');
        if (bgOpacityEl) state.config.keys_bg_opacity = parseFloat(bgOpacityEl.value);

        const keysBgColorEl = document.getElementById('setup-bg-color');
        if (keysBgColorEl) state.config.keys_bg_color = keysBgColorEl.value;

        const trailSpeedEl = document.getElementById('setup-trail-speed');
        if (trailSpeedEl) state.config.trail_speed = parseFloat(trailSpeedEl.value);

        const trailHeightEl = document.getElementById('setup-trail-height-num');
        if (trailHeightEl) state.config.trail_height = parseInt(trailHeightEl.value) || 800;

        if (!state.config.trail_widths) state.config.trail_widths = [50, 50, 50, 50];
        for (let i = 0; i < 4; i++) {
            const el = document.getElementById(`setup-trail-width-${i}`);
            if (el) state.config.trail_widths[i] = parseInt(el.value) || 50;
        }

        if (!state.config.key_labels) state.config.key_labels = ["D", "F", "J", "K"];
        for (let i = 0; i < 4; i++) {
            const el = document.getElementById(`setup-key-label-${i}`);
            if (el) state.config.key_labels[i] = el.value.trim();
        }

        if (!state.config.key_offsets_x) state.config.key_offsets_x = [0, 0, 0, 0];
        if (!state.config.key_offsets_y) state.config.key_offsets_y = [0, 0, 0, 0];
        for (let i = 0; i < 4; i++) {
            const elX = document.getElementById(`setup-key-offset-x-${i}`);
            const elY = document.getElementById(`setup-key-offset-y-${i}`);
            if (elX) state.config.key_offsets_x[i] = parseInt(elX.value) || 0;
            if (elY) state.config.key_offsets_y[i] = parseInt(elY.value) || 0;
        }

        const lockTrailsEl = document.getElementById('setup-lock-trails');
        if (lockTrailsEl) state.config.lock_trails = lockTrailsEl.checked;

        if (!state.config.trail_offsets_x) state.config.trail_offsets_x = [0, 0, 0, 0];
        for (let i = 0; i < 4; i++) {
            const el = document.getElementById(`setup-trail-offset-${i}`);
            if (el) state.config.trail_offsets_x[i] = parseInt(el.value) || 0;
        }

        if (!state.config.rgb_enabled_keys) state.config.rgb_enabled_keys = [false, false, false, false];
        for (let i = 0; i < 4; i++) {
            const el = document.getElementById(`setup-rgb-key-${i}`);
            if (el) state.config.rgb_enabled_keys[i] = el.checked;
        }

        const rgbSpeedEl = document.getElementById('setup-rgb-speed');
        if (rgbSpeedEl) state.config.rgb_speed = parseFloat(rgbSpeedEl.value) || 1.0;

        const skillsArray = ['stream', 'jumpstream', 'handstream', 'stamina', 'jackspeed', 'chordjack', 'technical'];
        if (!state.config.visible_skills) state.config.visible_skills = {};
        skillsArray.forEach(skill => {
            const pill = document.getElementById(`btn-${skill}`);
            if (pill) state.config.visible_skills[skill] = pill.classList.contains('active');
        });
        
        await saveVisualSettings();
        if (onConnectTosu) onConnectTosu();
        
        let toast = document.getElementById('autosave-toast');
        if (!toast) {
            toast = document.createElement('div');
            toast.id = 'autosave-toast';
            toast.className = 'autosave-toast';
            toast.textContent = 'Saved automatically';
            document.body.appendChild(toast);
        }
        toast.classList.add('visible');
        clearTimeout(window._toastTimeout);
        window._toastTimeout = setTimeout(() => {
            toast.classList.remove('visible');
        }, 1500);
    } catch (err) {
        console.error("Error saving settings:", err);
    }
}

export function initWindowControls() {
    const minBtn = document.getElementById('min-btn');
    const maxBtn = document.getElementById('max-btn');
    const closeBtn = document.getElementById('close-btn');

    if (minBtn) {
        minBtn.addEventListener('click', () => invoke('minimize_window'));
    }
    if (maxBtn) {
        maxBtn.addEventListener('click', () => invoke('maximize_window'));
    }
    if (closeBtn) {
        closeBtn.addEventListener('click', () => invoke('close_window'));
    }

    const header = document.querySelector('.setup-screen');
    if (header) {
        header.addEventListener('mousedown', (e) => {
            if (e.target.closest('input, button, a, .close-btn, .guide-modal-close, .copiable-link, #guide-modal')) return;
            try {
                window.__TAURI__.window.getCurrentWindow().startDragging();
            } catch (e) {}
        });
    }
}
