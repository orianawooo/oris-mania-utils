import { state } from './state.js';
import { saveConfig } from './config-store.js';

const isTauri = typeof window.__TAURI__ !== 'undefined';
const invoke = isTauri ? window.__TAURI__.core.invoke : async () => {};

function normalizeHexHitColor(color, fallback) {
    const match = String(color || '').match(/^#([0-9a-f]{6})$/i);
    return match ? match[0] : fallback;
}

export function applyVisualSettings() {
    document.documentElement.style.setProperty('--bg-opacity', state.config.bg_opacity || 0.85);
    
    const accent = state.config.accent_color || '#ff8ab3';
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
            const pill = document.getElementById(`btn-${skill}`);
            if (pill) pill.classList.toggle('active', !!visible);
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
    if (accentColorPicker) accentColorPicker.value = state.config.accent_color || '#ff8ab3';
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

    const songsPathInput = document.getElementById('songs-path-input');
    if (songsPathInput && state.config.osu_songs_path) {
        songsPathInput.value = state.config.osu_songs_path;
    }

    const tosuPathInput = document.getElementById('tosu-path-input');
    if (tosuPathInput && state.config.tosu_root_path) {
        tosuPathInput.value = state.config.tosu_root_path;
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

    const hitBorderColorEl = document.getElementById('setup-hit-border-color');
    if (hitBorderColorEl && state.config.hitcounter_border_color) hitBorderColorEl.value = normalizeHexHitColor(state.config.hitcounter_border_color, '#e6bfd4');

    const hitOrientationEl = document.getElementById('setup-hit-orientation');
    if (hitOrientationEl && state.config.hitcounter_orientation) hitOrientationEl.value = state.config.hitcounter_orientation;

    const hitFontEl = document.getElementById('setup-hit-font');
    if (hitFontEl && state.config.hitcounter_font) hitFontEl.value = state.config.hitcounter_font;
    
    const particlesEl = document.getElementById('setup-particles');
    if (particlesEl && state.config.show_particles !== undefined) particlesEl.checked = state.config.show_particles;

    const particleCountEl = document.getElementById('setup-particle-count');
    if (particleCountEl && state.config.particle_count !== undefined) particleCountEl.value = state.config.particle_count;
    const particleCountVal = document.getElementById('val-particle-count');
    if (particleCountVal && state.config.particle_count !== undefined) particleCountVal.textContent = state.config.particle_count;

    const particleSizeEl = document.getElementById('setup-particle-size');
    if (particleSizeEl && state.config.particle_max_size !== undefined) particleSizeEl.value = state.config.particle_max_size;
    const particleSizeVal = document.getElementById('val-particle-size');
    if (particleSizeVal && state.config.particle_max_size !== undefined) particleSizeVal.textContent = state.config.particle_max_size;

    const particleMinSizeEl = document.getElementById('setup-particle-min-size');
    if (particleMinSizeEl && state.config.particle_min_size !== undefined) particleMinSizeEl.value = state.config.particle_min_size;
    const particleMinSizeVal = document.getElementById('val-particle-min-size');
    if (particleMinSizeVal && state.config.particle_min_size !== undefined) particleMinSizeVal.textContent = state.config.particle_min_size;

    const particleSpreadEl = document.getElementById('setup-particle-spread');
    if (particleSpreadEl && state.config.particle_spread !== undefined) particleSpreadEl.value = state.config.particle_spread;
    const particleSpreadVal = document.getElementById('val-particle-spread');
    if (particleSpreadVal && state.config.particle_spread !== undefined) particleSpreadVal.textContent = state.config.particle_spread;

    const particleLifeEl = document.getElementById('setup-particle-life');
    if (particleLifeEl && state.config.particle_life !== undefined) particleLifeEl.value = state.config.particle_life;
    const particleLifeVal = document.getElementById('val-particle-life');
    if (particleLifeVal && state.config.particle_life !== undefined) particleLifeVal.textContent = Number(state.config.particle_life).toFixed(1);

    const particleGravityEl = document.getElementById('setup-particle-gravity');
    if (particleGravityEl && state.config.particle_gravity !== undefined) particleGravityEl.value = state.config.particle_gravity;
    const particleGravityVal = document.getElementById('val-particle-gravity');
    if (particleGravityVal && state.config.particle_gravity !== undefined) particleGravityVal.textContent = Number(state.config.particle_gravity).toFixed(2);

    const particleSpeedEl = document.getElementById('setup-particle-speed');
    if (particleSpeedEl && state.config.particle_speed !== undefined) particleSpeedEl.value = state.config.particle_speed;
    const particleSpeedVal = document.getElementById('val-particle-speed');
    if (particleSpeedVal && state.config.particle_speed !== undefined) particleSpeedVal.textContent = Number(state.config.particle_speed).toFixed(1);

    const particleRgbEl = document.getElementById('setup-particle-rgb');
    if (particleRgbEl && state.config.particle_rgb !== undefined) particleRgbEl.checked = state.config.particle_rgb;

    const particleShapeEl = document.getElementById('setup-particle-shape');
    if (particleShapeEl && state.config.particle_shape) particleShapeEl.value = state.config.particle_shape;
    
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

    const bgEnabledEl = document.getElementById('setup-bg-enabled');
    if (bgEnabledEl && state.config.keys_bg_enabled !== undefined) bgEnabledEl.checked = state.config.keys_bg_enabled;

    const bgOffsetXEl = document.getElementById('setup-bg-offset-x');
    if (bgOffsetXEl && state.config.keys_bg_offset_x !== undefined) bgOffsetXEl.value = state.config.keys_bg_offset_x;
    const bgOffsetXVal = document.getElementById('val-bg-offset-x');
    if (bgOffsetXVal && state.config.keys_bg_offset_x !== undefined) bgOffsetXVal.textContent = state.config.keys_bg_offset_x;

    const bgOffsetYEl = document.getElementById('setup-bg-offset-y');
    if (bgOffsetYEl && state.config.keys_bg_offset_y !== undefined) bgOffsetYEl.value = state.config.keys_bg_offset_y;
    const bgOffsetYVal = document.getElementById('val-bg-offset-y');
    if (bgOffsetYVal && state.config.keys_bg_offset_y !== undefined) bgOffsetYVal.textContent = state.config.keys_bg_offset_y;

    const bgWidthEl = document.getElementById('setup-bg-width');
    if (bgWidthEl && state.config.keys_bg_width !== undefined) bgWidthEl.value = state.config.keys_bg_width;

    const bgHeightEl = document.getElementById('setup-bg-height');
    if (bgHeightEl && state.config.keys_bg_height !== undefined) bgHeightEl.value = state.config.keys_bg_height;

    const bgPaddingEl = document.getElementById('setup-bg-padding');
    if (bgPaddingEl && state.config.keys_bg_padding !== undefined) bgPaddingEl.value = state.config.keys_bg_padding;
    const bgPaddingVal = document.getElementById('val-bg-padding');
    if (bgPaddingVal && state.config.keys_bg_padding !== undefined) bgPaddingVal.textContent = state.config.keys_bg_padding;

    const bgRadiusEl = document.getElementById('setup-bg-radius');
    if (bgRadiusEl && state.config.keys_bg_radius !== undefined) bgRadiusEl.value = state.config.keys_bg_radius;
    const bgRadiusVal = document.getElementById('val-bg-radius');
    if (bgRadiusVal && state.config.keys_bg_radius !== undefined) bgRadiusVal.textContent = state.config.keys_bg_radius;

    const bgScaleEl = document.getElementById('setup-bg-scale');
    if (bgScaleEl && state.config.keys_bg_scale !== undefined) bgScaleEl.value = state.config.keys_bg_scale;
    const bgScaleVal = document.getElementById('val-bg-scale');
    if (bgScaleVal && state.config.keys_bg_scale !== undefined) bgScaleVal.textContent = Number(state.config.keys_bg_scale).toFixed(2);

    const bgRotationEl = document.getElementById('setup-bg-rotation');
    if (bgRotationEl && state.config.keys_bg_rotation !== undefined) bgRotationEl.value = state.config.keys_bg_rotation;
    const bgRotationVal = document.getElementById('val-bg-rotation');
    if (bgRotationVal && state.config.keys_bg_rotation !== undefined) bgRotationVal.textContent = Number(state.config.keys_bg_rotation).toFixed(0);

    const bgShapeEl = document.getElementById('setup-bg-shape');
    if (bgShapeEl && state.config.keys_bg_shape) bgShapeEl.value = state.config.keys_bg_shape;

    const bgLayerEl = document.getElementById('setup-bg-layer');
    if (bgLayerEl) bgLayerEl.value = state.config.bg_layer !== undefined ? state.config.bg_layer : 6;

    const trailLayerEl = document.getElementById('setup-trail-layer');
    if (trailLayerEl) trailLayerEl.value = state.config.trail_layer !== undefined ? state.config.trail_layer : 8;

    const keyLayerEl = document.getElementById('setup-key-layer');
    if (keyLayerEl) keyLayerEl.value = state.config.key_layer !== undefined ? state.config.key_layer : 10;

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

export async function saveVisualSettings(options = {}) {
    await saveConfig({ reason: 'visual-settings', ...options });
}
