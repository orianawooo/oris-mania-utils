import { initVisualEditor } from './editor.js';
import { state } from './state.js';
import { saveVisualSettings, populateSettingsPanel, applyVisualSettings } from './config.js';
import { setupTosuConnection } from './api.js';
import { applyConfigSnapshot, setPersistedConfigBaseline } from './config-store.js';
import { applyKeystrokeDefaults } from './default-config.js';

const isTauri = typeof window.__TAURI__ !== 'undefined';
const invoke = isTauri ? window.__TAURI__.core.invoke : async () => {};

let listeningForKeyIndex = null;
const TAB_REQUIREMENT_MESSAGE = 'Install this overlay first to unlock its editor.';

async function refreshBootstrapState(onValidatePath, onValidateTosuPath) {
    if (!isTauri) return null;

    const bootstrap = await invoke('bootstrap_environment');
    if (bootstrap && bootstrap.config) {
        applyConfigSnapshot(bootstrap.config);
        setPersistedConfigBaseline(bootstrap.config);
        populateSettingsPanel();
        syncVisibleSkillButtons();
        if (onValidatePath && state.config.osu_songs_path) onValidatePath(state.config.osu_songs_path);
        if (onValidateTosuPath && state.config.tosu_root_path) onValidateTosuPath(state.config.tosu_root_path);
    }
    return bootstrap;
}

function syncVisibleSkillButtons() {
    const skills = ['stream', 'jumpstream', 'handstream', 'stamina', 'jackspeed', 'chordjack', 'technical'];
    if (!state.config.visible_skills) state.config.visible_skills = {};

    skills.forEach(skill => {
        const pill = document.getElementById(`btn-${skill}`);
        const active = state.config.visible_skills[skill] !== false;
        if (pill) pill.classList.toggle('active', active);
    });
}

async function toggleVisibleSkill(skill) {
    if (!state.config.visible_skills) state.config.visible_skills = {};
    const nextValue = state.config.visible_skills[skill] === false;
    state.config.visible_skills[skill] = nextValue;
    syncVisibleSkillButtons();
    applyVisualSettings();
    await saveVisualSettings();
}

export function initUIListeners(callbacks = {}) {
    const { onValidatePath, onValidateTosuPath, onConnectTosu, onBootstrapState } = callbacks;
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
    const heightSlider = document.getElementById('setup-key-height');
    if (heightSlider) {
        heightSlider.addEventListener('input', () => readAndSaveSetupSettings());
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

    initVisualEditor();

    const skillButtons = {
        stream: document.getElementById('btn-stream'),
        jumpstream: document.getElementById('btn-jumpstream'),
        handstream: document.getElementById('btn-handstream'),
        stamina: document.getElementById('btn-stamina'),
        jackspeed: document.getElementById('btn-jackspeed'),
        chordjack: document.getElementById('btn-chordjack'),
        technical: document.getElementById('btn-technical'),
    };
    Object.entries(skillButtons).forEach(([skill, pill]) => {
        if (pill) {
            pill.type = 'button';
            pill.addEventListener('click', async (e) => {
                e.preventDefault();
                await toggleVisibleSkill(skill);
            });
        }
    });
    syncVisibleSkillButtons();

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
                await readAndSaveSetupSettings({ immediate: true, reason: 'songs-folder-picked' });
                const bootstrap = await refreshBootstrapState(onValidatePath, onValidateTosuPath);
                if (onBootstrapState) onBootstrapState(bootstrap);
            }
        });
    }

    const browseTosuBtn = document.getElementById('browse-tosu-btn');
    if (browseTosuBtn) {
        browseTosuBtn.addEventListener('click', async () => {
            const folder = await invoke('select_tosu_folder');
            if (folder) {
                const tosuPathInput = document.getElementById('tosu-path-input');
                if (tosuPathInput) {
                    tosuPathInput.value = folder;
                    if (onValidateTosuPath) onValidateTosuPath(folder);
                }
                await readAndSaveSetupSettings({ immediate: true, reason: 'tosu-folder-picked' });
                const bootstrap = await refreshBootstrapState(onValidatePath, onValidateTosuPath);
                if (onBootstrapState) onBootstrapState(bootstrap);
            }
        });
    }

    const detectTosuBtn = document.getElementById('detect-tosu-btn');
    if (detectTosuBtn) {
        detectTosuBtn.addEventListener('click', async () => {
            const folder = await invoke('detect_tosu_root_path');
            const tosuPathInput = document.getElementById('tosu-path-input');
            if (folder) {
                if (tosuPathInput) {
                    tosuPathInput.value = folder;
                    if (onValidateTosuPath) onValidateTosuPath(folder);
                }
                await readAndSaveSetupSettings({ immediate: true, reason: 'tosu-detected' });
                const bootstrap = await refreshBootstrapState(onValidatePath, onValidateTosuPath);
                if (onBootstrapState) onBootstrapState(bootstrap);
            } else if (tosuPathInput) {
                tosuPathInput.focus();
                if (onValidateTosuPath) onValidateTosuPath(tosuPathInput.value.trim());
            }
        });
    }

    const songsPathInput = document.getElementById('songs-path-input');
    if (songsPathInput) {
        songsPathInput.addEventListener('input', (e) => {
            if (onValidatePath) onValidatePath(e.target.value.trim());
        });
    }

    const tosuPathInput = document.getElementById('tosu-path-input');
    if (tosuPathInput) {
        tosuPathInput.addEventListener('input', (e) => {
            if (onValidateTosuPath) onValidateTosuPath(e.target.value.trim());
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
            await readAndSaveSetupSettings({ immediate: true, reason: 'manual-save' });
            const bootstrap = await refreshBootstrapState(onValidatePath, onValidateTosuPath);
            if (onBootstrapState) onBootstrapState(bootstrap);
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
                await readAndSaveSetupSettings({ immediate: true, reason: 'guide-opened' });
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
            applyKeystrokeDefaults(state.config);
            
            populateSettingsPanel();
            await readAndSaveSetupSettings({ immediate: true, reason: 'reset-keystrokes' });
        });
    }

    initTheme();
    initTabs();
}

function initTheme() {
    const themeBtn = document.getElementById('theme-toggle-btn');
    const setupScreen = document.getElementById('setup-screen');
    const applyTheme = (theme) => {
        document.documentElement.dataset.theme = theme;
        document.body.dataset.theme = theme;
        if (setupScreen) {
            setupScreen.classList.toggle('dark-mode', theme === 'dark');
        }
        if (themeBtn) {
            themeBtn.textContent = theme === 'dark' ? 'Light Mode' : 'Dark Mode';
        }
    };
    
    if (themeBtn && setupScreen) {
        themeBtn.addEventListener('click', () => {
            const nextTheme = document.body.dataset.theme === 'dark' ? 'light' : 'dark';
            applyTheme(nextTheme);
            localStorage.setItem('theme', nextTheme);
        });
        
        const savedTheme = localStorage.getItem('theme') === 'dark' ? 'dark' : 'light';
        applyTheme(savedTheme);
    }
}

function initTabs() {
    const tabButtons = document.querySelectorAll('.tab-btn');
    const tabPanes = document.querySelectorAll('.tab-pane');

    function activateTab(tabId) {
        tabButtons.forEach((b) => b.classList.remove('active'));
        tabPanes.forEach((p) => p.classList.remove('active'));

        const activeButton = document.querySelector(`.tab-btn[data-tab="${tabId}"]`);
        const targetPane = document.getElementById(`${tabId}-tab`);
        if (activeButton) activeButton.classList.add('active');
        if (targetPane) {
            targetPane.classList.add('active');
            requestAnimationFrame(() => {
                requestAnimationFrame(() => {
                    targetPane.dispatchEvent(new CustomEvent('ori:tab-activated', { bubbles: true, detail: { tabId } }));
                    document.dispatchEvent(new CustomEvent('ori:tab-activated', { detail: { tabId } }));
                });
            });
        }
    }

    tabButtons.forEach(btn => {
        btn.addEventListener('click', () => {
            if (btn.disabled) {
                return;
            }
            const tabId = btn.getAttribute('data-tab');
            activateTab(tabId);
        });
    });

    setEditorTabAvailability([]);
}

export function setEditorTabAvailability(overlayStatuses = []) {
    const statusMap = new Map((overlayStatuses || []).map((status) => [status.name, !!status.installed]));
    const tabButtons = document.querySelectorAll('.tab-btn[data-required-overlay]');

    tabButtons.forEach((button) => {
        const overlayName = button.getAttribute('data-required-overlay');
        const installed = statusMap.get(overlayName) === true;
        button.disabled = !installed;
        button.classList.toggle('locked', !installed);
        button.setAttribute('aria-disabled', installed ? 'false' : 'true');
        button.title = installed ? '' : TAB_REQUIREMENT_MESSAGE;
    });

    const activeLockedTab = document.querySelector('.tab-btn.active[data-required-overlay][disabled]');
    if (activeLockedTab) {
        const msdButton = document.querySelector('.tab-btn[data-tab="msd"]');
        const msdPane = document.getElementById('msd-tab');
        document.querySelectorAll('.tab-btn').forEach((button) => button.classList.remove('active'));
        document.querySelectorAll('.tab-pane').forEach((pane) => pane.classList.remove('active'));
        if (msdButton) msdButton.classList.add('active');
        if (msdPane) msdPane.classList.add('active');
    }
}



export async function readAndSaveSetupSettings(options = {}) {
    const { immediate = false, reason = 'ui' } = options;
    try {
        const songsPathInput = document.getElementById('songs-path-input');
        const path = songsPathInput ? songsPathInput.value.trim() : '';
        state.config.osu_songs_path = path;

        const tosuPathInput = document.getElementById('tosu-path-input');
        const tosuPath = tosuPathInput ? tosuPathInput.value.trim() : '';
        state.config.tosu_root_path = tosuPath;
        
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
        const bgEnabledEl = document.getElementById('setup-bg-enabled');
        const bgOffsetXEl = document.getElementById('setup-bg-offset-x');
        const bgOffsetYEl = document.getElementById('setup-bg-offset-y');
        const bgWidthEl = document.getElementById('setup-bg-width');
        const bgHeightEl = document.getElementById('setup-bg-height');
        const bgPaddingEl = document.getElementById('setup-bg-padding');
        const bgRadiusEl = document.getElementById('setup-bg-radius');
        const bgScaleEl = document.getElementById('setup-bg-scale');
        const bgRotationEl = document.getElementById('setup-bg-rotation');
        const bgShapeEl = document.getElementById('setup-bg-shape');
        const bgLayerEl = document.getElementById('setup-bg-layer');
        const trailLayerEl = document.getElementById('setup-trail-layer');
        const keyLayerEl = document.getElementById('setup-key-layer');
        const hitBorderColorEl = document.getElementById('setup-hit-border-color');
        const hitFontEl = document.getElementById('setup-hit-font');
        const hitPosXEl = document.getElementById('setup-hit-pos-x');
        const hitPosYEl = document.getElementById('setup-hit-pos-y');
        const hitPaddingEl = document.getElementById('setup-hit-padding');
        const hitGapEl = document.getElementById('setup-hit-gap');
        const hitItemWidthEl = document.getElementById('setup-hit-item-width');
        const hitItemHeightEl = document.getElementById('setup-hit-item-height');
        const hitRadiusEl = document.getElementById('setup-hit-radius');
        const hitDotSizeEl = document.getElementById('setup-hit-dot-size');
        const hitLabelSizeEl = document.getElementById('setup-hit-label-size');
        const hitValueSizeEl = document.getElementById('setup-hit-value-size');
        const hitGlowEl = document.getElementById('setup-hit-glow');
        const particleCountEl = document.getElementById('setup-particle-count');
        const particleMinSizeEl = document.getElementById('setup-particle-min-size');
        const particleSizeEl = document.getElementById('setup-particle-size');
        const particleSpreadEl = document.getElementById('setup-particle-spread');
        const particleLifeEl = document.getElementById('setup-particle-life');
        const particleGravityEl = document.getElementById('setup-particle-gravity');
        const particleSpeedEl = document.getElementById('setup-particle-speed');
        const particleRgbEl = document.getElementById('setup-particle-rgb');
        const particleShapeEl = document.getElementById('setup-particle-shape');
        
        if (opacityEl) state.config.bg_opacity = parseFloat(opacityEl.value);
        if (accentEl) state.config.accent_color = accentEl.value;
        if (scaleEl) state.config.scale = parseFloat(scaleEl.value);
        if (radarEl) state.config.show_radar = radarEl.checked;
        if (hitOpacityEl) state.config.hitcounter_opacity = parseFloat(hitOpacityEl.value);
        if (hitScaleEl) state.config.hitcounter_scale = parseFloat(hitScaleEl.value);
        if (hitBgColorEl) state.config.hitcounter_bg_color = hitBgColorEl.value;
        if (hitTextColorEl) state.config.hitcounter_text_color = hitTextColorEl.value;
        if (hitBorderStyleEl) state.config.hitcounter_border_style = hitBorderStyleEl.value;
        if (hitBorderColorEl) state.config.hitcounter_border_color = hitBorderColorEl.value;
        if (hitOrientationEl) state.config.hitcounter_orientation = hitOrientationEl.value;
        if (hitFontEl) state.config.hitcounter_font = hitFontEl.value;
        if (hitPosXEl) state.config.hitcounter_position_x = parseInt(hitPosXEl.value) || 0;
        if (hitPosYEl) state.config.hitcounter_position_y = parseInt(hitPosYEl.value) || 0;
        if (hitPaddingEl) state.config.hitcounter_padding = parseInt(hitPaddingEl.value) || 0;
        if (hitGapEl) state.config.hitcounter_gap = parseInt(hitGapEl.value) || 0;
        if (hitItemWidthEl) state.config.hitcounter_item_width = parseInt(hitItemWidthEl.value) || 118;
        if (hitItemHeightEl) state.config.hitcounter_item_height = parseInt(hitItemHeightEl.value) || 72;
        if (hitRadiusEl) state.config.hitcounter_item_radius = parseInt(hitRadiusEl.value) || 16;
        if (hitDotSizeEl) state.config.hitcounter_dot_size = parseInt(hitDotSizeEl.value) || 8;
        if (hitLabelSizeEl) state.config.hitcounter_label_size = parseFloat(hitLabelSizeEl.value) || 12;
        if (hitValueSizeEl) state.config.hitcounter_value_size = parseFloat(hitValueSizeEl.value) || 24;
        if (hitGlowEl) state.config.hitcounter_glow_strength = parseFloat(hitGlowEl.value) || 0.35;
        if (particlesEl) state.config.show_particles = particlesEl.checked;
        if (bgEnabledEl) state.config.keys_bg_enabled = bgEnabledEl.checked;
        if (bgOffsetXEl) state.config.keys_bg_offset_x = parseInt(bgOffsetXEl.value) || 0;
        if (bgOffsetYEl) state.config.keys_bg_offset_y = parseInt(bgOffsetYEl.value) || 0;
        if (bgWidthEl) state.config.keys_bg_width = parseInt(bgWidthEl.value) || 0;
        if (bgHeightEl) state.config.keys_bg_height = parseInt(bgHeightEl.value) || 0;
        if (bgPaddingEl) state.config.keys_bg_padding = parseInt(bgPaddingEl.value) || 0;
        if (bgRadiusEl) state.config.keys_bg_radius = parseInt(bgRadiusEl.value) || 0;
        if (bgScaleEl) state.config.keys_bg_scale = parseFloat(bgScaleEl.value) || 1;
        if (bgRotationEl) state.config.keys_bg_rotation = parseFloat(bgRotationEl.value) || 0;
        if (bgShapeEl) state.config.keys_bg_shape = bgShapeEl.value || 'rounded';
        if (bgLayerEl) state.config.bg_layer = parseInt(bgLayerEl.value) || 6;
        if (trailLayerEl) state.config.trail_layer = parseInt(trailLayerEl.value) || 8;
        if (keyLayerEl) state.config.key_layer = parseInt(keyLayerEl.value) || 10;
        if (particleCountEl) state.config.particle_count = parseInt(particleCountEl.value) || 0;
        if (particleMinSizeEl) state.config.particle_min_size = parseInt(particleMinSizeEl.value) || 1;
        if (particleSizeEl) state.config.particle_max_size = parseInt(particleSizeEl.value) || 1;
        if (particleSpreadEl) state.config.particle_spread = parseInt(particleSpreadEl.value) || 0;
        if (particleLifeEl) state.config.particle_life = parseFloat(particleLifeEl.value) || 0;
        if (particleGravityEl) state.config.particle_gravity = parseFloat(particleGravityEl.value) || 0;
        if (particleSpeedEl) state.config.particle_speed = parseFloat(particleSpeedEl.value) || 0;
        if (particleRgbEl) state.config.particle_rgb = particleRgbEl.checked;
        if (particleShapeEl) state.config.particle_shape = particleShapeEl.value;
        
        const colorOuterEl = document.getElementById('setup-key-color-outer');
        const colorInnerEl = document.getElementById('setup-key-color-inner');
        const sizeEl = document.getElementById('setup-key-size');
        const gapEl = document.getElementById('setup-key-gap');
        
        if (colorOuterEl) state.config.key_color_outer = colorOuterEl.value;
        if (colorInnerEl) state.config.key_color_inner = colorInnerEl.value;
        if (sizeEl) {
            state.config.key_size = parseInt(sizeEl.value);
            const valLabel = document.getElementById('val-key-size');
            if (valLabel) valLabel.textContent = sizeEl.value;
        }
        if (gapEl) {
            state.config.key_gap = parseInt(gapEl.value);
            const valLabel = document.getElementById('val-key-gap');
            if (valLabel) valLabel.textContent = gapEl.value;
        }
        const keyHeightEl = document.getElementById('setup-key-height');
        if (keyHeightEl) {
            state.config.key_height = parseInt(keyHeightEl.value);
            const valLabel = document.getElementById('val-key-height');
            if (valLabel) valLabel.textContent = keyHeightEl.value;
        }
        if (!state.config.key_colors) state.config.key_colors = ['#00d2ff','#ff007f','#ff007f','#00d2ff'];
        for (let i = 0; i < 4; i++) {
            const elColor = document.getElementById(`setup-key-color-${i}`);
            if (elColor) state.config.key_colors[i] = elColor.value;
        }

        const trailsEl = document.getElementById('setup-trails');
        const trailOpacityEl = document.getElementById('setup-trail-opacity');
        
        if (trailsEl) state.config.show_trails = trailsEl.checked;
        if (trailOpacityEl) { state.config.trail_opacity = parseFloat(trailOpacityEl.value); const valLabel = document.getElementById('val-trail-opacity'); if (valLabel) valLabel.textContent = trailOpacityEl.value; }

        const trailFadeEl = document.getElementById('setup-trail-fade');
        if (trailFadeEl) { state.config.trail_fade = parseFloat(trailFadeEl.value); const valLabel = document.getElementById('val-trail-fade'); if (valLabel) valLabel.textContent = trailFadeEl.value; }

        const bgOpacityEl = document.getElementById('setup-bg-opacity');
        if (bgOpacityEl) { state.config.keys_bg_opacity = parseFloat(bgOpacityEl.value); const valLabel = document.getElementById('val-bg-opacity'); if (valLabel) valLabel.textContent = bgOpacityEl.value; }

        const keysBgColorEl = document.getElementById('setup-bg-color');
        if (keysBgColorEl) state.config.keys_bg_color = keysBgColorEl.value;

        const trailSpeedEl = document.getElementById('setup-trail-speed');
        if (trailSpeedEl) { state.config.trail_speed = parseFloat(trailSpeedEl.value); const valLabel = document.getElementById('val-trail-speed'); if (valLabel) valLabel.textContent = trailSpeedEl.value; }

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
        if (!state.config.trail_offsets_y) state.config.trail_offsets_y = [0, 0, 0, 0];

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
        
        await saveVisualSettings({ immediate, reason });
        
        let toast = document.getElementById('autosave-toast');
        if (!toast) {
            toast = document.createElement('div');
            toast.id = 'autosave-toast';
            toast.className = 'autosave-toast';
            toast.textContent = 'Saved automatically';
            document.body.appendChild(toast);
        }
        toast.classList.add('visible');
        document.dispatchEvent(new CustomEvent('config-saved', { detail: { config: state.config } }));
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

    const dragHeader = document.querySelector('.header-minimal');
    if (dragHeader) {
        dragHeader.addEventListener('mousedown', (e) => {
            if (e.target.closest('button, a, select, input')) return;
            try {
                window.__TAURI__.window.getCurrentWindow().startDragging();
            } catch (e) {}
        });
    }
}
