import { state } from './state.js';

const isTauri = typeof window.__TAURI__ !== 'undefined';
const invoke = isTauri ? window.__TAURI__.core.invoke : async () => {};

export function applyVisualSettings() {
    document.documentElement.style.setProperty('--bg-opacity', state.config.bg_opacity || 0.85);
    
    let accent = state.config.accent_color || '#a67c52';
    if (accent === '#ff69b4' || accent === '#7c3aed') accent = '#a67c52';
    document.documentElement.style.setProperty('--accent-color', accent);
    
    document.documentElement.style.setProperty('--ui-scale', state.config.scale || 1.0);
    
    if (state.config.visible_skills) {
        for (const [skill, visible] of Object.entries(state.config.visible_skills)) {
            const span = document.getElementById(`val-${skill}`);
            if (span) {
                const row = span.closest('.skill-row');
                if (row) {
                    if (visible) row.classList.remove('hidden');
                    else row.classList.add('hidden');
                }
            }
            const toggle = document.getElementById(`toggle-${skill}`);
            if (toggle) toggle.checked = visible;
        }
    }

    const chartWrap = document.querySelector('.chart-wrap');
    if (chartWrap) {
        if (state.config.show_radar === false) {
            chartWrap.classList.add('hidden');
        } else {
            chartWrap.classList.remove('hidden');
        }
    }
    
    if (state.config.keys && state.config.keys.length >= 4) {
        for (let i = 0; i < 4; i++) {
            const input = document.getElementById(`key-input-${i}`);
            if (input) input.value = state.config.keys[i].replace('Key', '');
        }
    }

    const opacitySlider = document.getElementById('opacity-slider');
    const accentColorPicker = document.getElementById('accent-color-picker');
    const scaleSlider = document.getElementById('scale-slider');
    const radarToggle = document.getElementById('radar-toggle');
    
    if (opacitySlider) opacitySlider.value = state.config.bg_opacity || 0.85;
    if (accentColorPicker) accentColorPicker.value = state.config.accent_color || '#7c3aed';
    if (scaleSlider) scaleSlider.value = state.config.scale || 1.0;
    if (radarToggle) radarToggle.checked = state.config.show_radar !== false;
}

export function populateSettingsPanel() {
    if (!state.config) return;
    
    if (state.config.keys) {
        for (let i = 0; i < 4; i++) {
            const input = document.getElementById(`setup-key-${i}`);
            if (input && state.config.keys[i]) {
                input.value = state.config.keys[i].replace('Key', '');
            }
        }
    }
    
    const opacityEl = document.getElementById('setup-opacity');
    if (opacityEl && state.config.bg_opacity !== undefined) opacityEl.value = state.config.bg_opacity;
    
    const accentEl = document.getElementById('setup-accent');
    if (accentEl && state.config.accent_color) accentEl.value = state.config.accent_color;
    
    const scaleEl = document.getElementById('setup-scale');
    if (scaleEl && state.config.scale !== undefined) scaleEl.value = state.config.scale;

    const hitOpacityEl = document.getElementById('setup-hit-opacity');
    if (hitOpacityEl && state.config.hitcounter_opacity !== undefined) hitOpacityEl.value = state.config.hitcounter_opacity;

    const hitScaleEl = document.getElementById('setup-hit-scale');
    if (hitScaleEl && state.config.hitcounter_scale !== undefined) hitScaleEl.value = state.config.hitcounter_scale;

    const hitBgColorEl = document.getElementById('setup-hit-bg-color');
    if (hitBgColorEl && state.config.hitcounter_bg_color) hitBgColorEl.value = state.config.hitcounter_bg_color;

    const hitTextColorEl = document.getElementById('setup-hit-text-color');
    if (hitTextColorEl && state.config.hitcounter_text_color) hitTextColorEl.value = state.config.hitcounter_text_color;

    const hitBorderStyleEl = document.getElementById('setup-hit-border-style');
    if (hitBorderStyleEl && state.config.hitcounter_border_style) hitBorderStyleEl.value = state.config.hitcounter_border_style;

    const hitOrientationEl = document.getElementById('setup-hit-orientation');
    if (hitOrientationEl && state.config.hitcounter_orientation) hitOrientationEl.value = state.config.hitcounter_orientation;
    
    const particlesEl = document.getElementById('setup-particles');
    if (particlesEl && state.config.show_particles !== undefined) particlesEl.checked = state.config.show_particles;
    
    const radarEl = document.getElementById('setup-radar');
    if (radarEl && state.config.show_radar !== undefined) radarEl.checked = state.config.show_radar;
    
    const colorOuterEl = document.getElementById('setup-key-color-outer');
    if (colorOuterEl && state.config.key_color_outer) colorOuterEl.value = state.config.key_color_outer;
    
    const colorInnerEl = document.getElementById('setup-key-color-inner');
    if (colorInnerEl && state.config.key_color_inner) colorInnerEl.value = state.config.key_color_inner;
    
    const sizeEl = document.getElementById('setup-key-size');
    if (sizeEl && state.config.key_size) sizeEl.value = state.config.key_size;
    
    const gapEl = document.getElementById('setup-key-gap');
    if (gapEl && state.config.key_gap) gapEl.value = state.config.key_gap;

    const heightEl = document.getElementById('setup-key-height');
    if (heightEl && state.config.key_height !== undefined) {
        heightEl.value = state.config.key_height;
        const valHeight = document.getElementById('val-key-height');
        if (valHeight) valHeight.textContent = state.config.key_height;
    }

    const sizeVal = document.getElementById('val-key-size');
    if (sizeVal && state.config.key_size) sizeVal.textContent = state.config.key_size;

    const gapVal = document.getElementById('val-key-gap');
    if (gapVal && state.config.key_gap) gapVal.textContent = state.config.key_gap;

    const opVal = document.getElementById('val-trail-opacity');
    if (opVal && state.config.trail_opacity !== undefined) opVal.textContent = state.config.trail_opacity;

    const fadeVal = document.getElementById('val-trail-fade');
    if (fadeVal && state.config.trail_fade !== undefined) fadeVal.textContent = state.config.trail_fade;

    const bgOpVal = document.getElementById('val-bg-opacity');
    if (bgOpVal && state.config.keys_bg_opacity !== undefined) bgOpVal.textContent = state.config.keys_bg_opacity;

    const speedVal = document.getElementById('val-trail-speed');
    if (speedVal && state.config.trail_speed !== undefined) speedVal.textContent = state.config.trail_speed;

    if (state.config.key_colors) {
        for (let i = 0; i < 4; i++) {
            const picker = document.getElementById(`setup-key-color-${i}`);
            if (picker && state.config.key_colors[i]) {
                picker.value = state.config.key_colors[i];
            }
        }
    }
    
    const trailsEl = document.getElementById('setup-trails');
    if (trailsEl && state.config.show_trails !== undefined) trailsEl.checked = state.config.show_trails;
    
    const trailOpacityEl = document.getElementById('setup-trail-opacity');
    if (trailOpacityEl && state.config.trail_opacity !== undefined) trailOpacityEl.value = state.config.trail_opacity;
    
    const trailFadeEl = document.getElementById('setup-trail-fade');
    if (trailFadeEl && state.config.trail_fade !== undefined) trailFadeEl.value = state.config.trail_fade;
    
    const bgOpacityEl = document.getElementById('setup-bg-opacity');
    if (bgOpacityEl && state.config.keys_bg_opacity !== undefined) bgOpacityEl.value = state.config.keys_bg_opacity;

    const bgColEl = document.getElementById('setup-bg-color');
    if (bgColEl && state.config.keys_bg_color) bgColEl.value = state.config.keys_bg_color;

    const trailSpeedEl = document.getElementById('setup-trail-speed');
    if (trailSpeedEl && state.config.trail_speed !== undefined) trailSpeedEl.value = state.config.trail_speed;

    const trailHeightSlider = document.getElementById('setup-trail-height-slider');
    const trailHeightNum = document.getElementById('setup-trail-height-num');
    if (trailHeightSlider && state.config.trail_height !== undefined) trailHeightSlider.value = state.config.trail_height;
    if (trailHeightNum && state.config.trail_height !== undefined) trailHeightNum.value = state.config.trail_height;

    if (state.config.trail_widths) {
        for (let i = 0; i < 4; i++) {
            const el = document.getElementById(`setup-trail-width-${i}`);
            if (el && state.config.trail_widths[i] !== undefined) el.value = state.config.trail_widths[i];
        }
    }

    const lockTrailsEl = document.getElementById('setup-lock-trails');
    const trailOffsetsGroup = document.getElementById('trail-offsets-group');
    if (lockTrailsEl && state.config.lock_trails !== undefined) {
        lockTrailsEl.checked = state.config.lock_trails;
        if (trailOffsetsGroup) {
            trailOffsetsGroup.style.display = state.config.lock_trails ? 'none' : 'flex';
        }
    }

    if (state.config.trail_offsets_x) {
        for (let i = 0; i < 4; i++) {
            const el = document.getElementById(`setup-trail-offset-${i}`);
            if (el && state.config.trail_offsets_x[i] !== undefined) el.value = state.config.trail_offsets_x[i];
        }
    }

    if (state.config.key_labels) {
        for (let i = 0; i < 4; i++) {
            const el = document.getElementById(`setup-key-label-${i}`);
            if (el && state.config.key_labels[i] !== undefined) el.value = state.config.key_labels[i];
        }
    }

    if (state.config.key_offsets_x && state.config.key_offsets_y) {
        for (let i = 0; i < 4; i++) {
            const elX = document.getElementById(`setup-key-offset-x-${i}`);
            const elY = document.getElementById(`setup-key-offset-y-${i}`);
            if (elX && state.config.key_offsets_x[i] !== undefined) elX.value = state.config.key_offsets_x[i];
            if (elY && state.config.key_offsets_y[i] !== undefined) elY.value = state.config.key_offsets_y[i];
        }
    }

    if (state.config.rgb_enabled_keys) {
        for (let i = 0; i < 4; i++) {
            const el = document.getElementById(`setup-rgb-key-${i}`);
            if (el && state.config.rgb_enabled_keys[i] !== undefined) el.checked = state.config.rgb_enabled_keys[i];
        }
    }

    const rgbSpeedEl = document.getElementById('setup-rgb-speed');
    if (rgbSpeedEl && state.config.rgb_speed !== undefined) rgbSpeedEl.value = state.config.rgb_speed;
    
    const skillsArray = ['stream', 'jumpstream', 'handstream', 'stamina', 'jackspeed', 'chordjack', 'technical'];
    if (state.config.visible_skills) {
        skillsArray.forEach(skill => {
            const toggle = document.getElementById(`setup-toggle-${skill}`);
            if (toggle && state.config.visible_skills[skill] !== undefined) {
                toggle.checked = state.config.visible_skills[skill];
            }
        });
    }
}

export async function saveVisualSettings() {
    await invoke('save_config', { config: state.config });
}
