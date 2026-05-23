
import { state } from './state.js';
import { readAndSaveSetupSettings } from './events.js';
import { populateSettingsPanel, saveVisualSettings } from './config.js';
import { recordPerf } from './perf.js';
import {
    getTrailBounds as getSharedTrailBounds,
    getTransformedBounds as getSharedTransformedBounds,
    normalizeRotation as sharedNormalizeRotation,
    normalizeScale as sharedNormalizeScale,
    normalizeShape as sharedNormalizeShape,
    resolveTrailAnchor,
    toRadians as sharedToRadians,
    traceShapePath as sharedTraceShapePath,
    withElementTransform as sharedWithElementTransform,
} from './keystrokes-geometry.js';

const isTauri = typeof window.__TAURI__ !== 'undefined';
const invoke = isTauri ? window.__TAURI__.core.invoke : async () => {};

let selectedKeyIndex = -1;
let selectedIsTrail = false;
let menuActiveKeyIndex = -1;
let menuActiveIsTrail = false;
let isListeningForBind = false;
let history = [];
let redoStack = [];
let isPreviewPlaying = false;
let previewAnimationFrame = null;
let previewTrailHeights = [0, 0, 0, 0];
let previewKeyStates = [false, false, false, false];
let previewParticles = [];
let previewLastTime = 0;
let draggedKey = null;
let draggedTrailIndex = -1;
let selectedBackground = false;
let draggedBackground = null;
let scheduledPreviewFrame = null;

function clamp(value, min, max) {
    return Math.min(max, Math.max(min, value));
}

function normalizeScale(value, fallback = 1) {
    return sharedNormalizeScale(value, fallback);
}

function normalizeRotation(value, fallback = 0) {
    return sharedNormalizeRotation(value, fallback);
}

function normalizeShape(value, fallback = 'rounded') {
    return sharedNormalizeShape(value, fallback);
}

function ensureArraySetting(key, length, fallback) {
    const values = Array.isArray(state.config[key]) ? [...state.config[key]] : [];
    while (values.length < length) {
        values.push(typeof fallback === 'function' ? fallback(values.length) : fallback);
    }
    if (values.length > length) values.length = length;
    state.config[key] = values;
    return values;
}

function ensureVisualTransformConfig() {
    const keyScales = ensureArraySetting('key_scales', 4, 1);
    const keyRotations = ensureArraySetting('key_rotations', 4, 0);
    const keyShapes = ensureArraySetting('key_shapes', 4, 'rounded');
    const trailScales = ensureArraySetting('trail_scales', 4, 1);
    const trailRotations = ensureArraySetting('trail_rotations', 4, 0);
    const trailShapes = ensureArraySetting('trail_shapes', 4, 'rounded');

    state.config.key_scales = keyScales.map(v => normalizeScale(v, 1));
    state.config.key_rotations = keyRotations.map(v => normalizeRotation(v, 0));
    state.config.key_shapes = keyShapes.map(v => normalizeShape(v, 'rounded'));
    state.config.trail_scales = trailScales.map(v => normalizeScale(v, 1));
    state.config.trail_rotations = trailRotations.map(v => normalizeRotation(v, 0));
    state.config.trail_shapes = trailShapes.map(v => normalizeShape(v, 'rounded'));

    state.config.keys_bg_scale = normalizeScale(state.config.keys_bg_scale, 1);
    state.config.keys_bg_rotation = normalizeRotation(state.config.keys_bg_rotation, 0);
    state.config.keys_bg_shape = normalizeShape(state.config.keys_bg_shape, 'rounded');
}

function toRadians(degrees) {
    return sharedToRadians(degrees);
}

function traceShapePath(ctx, x, y, width, height, shape = 'rounded', baseRadius = 12) {
    return sharedTraceShapePath(ctx, x, y, width, height, shape, baseRadius);
}

function withElementTransform(ctx, x, y, width, height, scale = 1, rotation = 0, draw) {
    return sharedWithElementTransform(ctx, x, y, width, height, scale, rotation, draw);
}

function getTransformedBounds(x, y, width, height, scale = 1, rotation = 0) {
    return getSharedTransformedBounds(x, y, width, height, scale, rotation);
}

function getEditorMetrics() {
    ensureVisualTransformConfig();
    const canvas = document.getElementById('editor-canvas');
    const ws = document.getElementById('editor-workspace');
    const W = canvas?.width || ws?.clientWidth || 400;
    const H = canvas?.height || ws?.clientHeight || 350;
    const size = parseInt(document.getElementById('setup-key-size')?.value || 60);
    const keyH = parseInt(document.getElementById('setup-key-height')?.value || size);
    const gap = parseInt(document.getElementById('setup-key-gap')?.value || 10);
    const totalW = size * 4 + gap * 3;
    const bgOffsetX = parseInt(document.getElementById('setup-bg-offset-x')?.value || 0) || 0;
    const bgOffsetY = parseInt(document.getElementById('setup-bg-offset-y')?.value || 0) || 0;
    const bgPadding = parseInt(document.getElementById('setup-bg-padding')?.value || 15) || 0;
    const startX = (W - totalW) / 2;
    // Match runtime overlay geometry: wrapper bottom 20px + container padding before keys.
    const baseY = H - keyH - 20 - bgPadding;

    return {
        canvas,
        ws,
        W,
        H,
        size,
        keyH,
        gap,
        totalW,
        startX,
        baseY,
        colors: getKeyColors(),
        labels: state.config.key_labels || ['D', 'F', 'J', 'K'],
        offX: state.config.key_offsets_x || [0, 0, 0, 0],
        offY: state.config.key_offsets_y || [0, 0, 0, 0],
        keyScales: state.config.key_scales || [1, 1, 1, 1],
        keyRotations: state.config.key_rotations || [0, 0, 0, 0],
        keyShapes: state.config.key_shapes || ['rounded', 'rounded', 'rounded', 'rounded'],
        trailWidths: state.config.trail_widths || [50, 50, 50, 50],
        trailScales: state.config.trail_scales || [1, 1, 1, 1],
        trailRotations: state.config.trail_rotations || [0, 0, 0, 0],
        trailShapes: state.config.trail_shapes || ['rounded', 'rounded', 'rounded', 'rounded'],
        trailHeight: parseInt(document.getElementById('setup-trail-height-num')?.value || state.config.trail_height || 800) || 800,
        trailSpeed: parseFloat(document.getElementById('setup-trail-speed')?.value || state.config.trail_speed || 6) || 6,
        trailOpacity: parseFloat(document.getElementById('setup-trail-opacity')?.value || 0.6),
        trailFade: parseFloat(document.getElementById('setup-trail-fade')?.value || 0),
        showTrails: document.getElementById('setup-trails')?.checked ?? true,
        bgEnabled: document.getElementById('setup-bg-enabled')?.checked ?? true,
        bgColor: document.getElementById('setup-bg-color')?.value || '#0a0a12',
        bgOpacity: parseFloat(document.getElementById('setup-bg-opacity')?.value || 0.85),
        bgOffsetX: parseInt(document.getElementById('setup-bg-offset-x')?.value || 0) || 0,
        bgOffsetY: parseInt(document.getElementById('setup-bg-offset-y')?.value || 0) || 0,
        bgWidth: parseInt(document.getElementById('setup-bg-width')?.value || 0) || 0,
        bgHeight: parseInt(document.getElementById('setup-bg-height')?.value || 0) || 0,
        bgPadding: parseInt(document.getElementById('setup-bg-padding')?.value || 15) || 0,
        bgRadius: parseInt(document.getElementById('setup-bg-radius')?.value || 16) || 0,
        bgScale: normalizeScale(document.getElementById('setup-bg-scale')?.value || state.config.keys_bg_scale || 1, 1),
        bgRotation: normalizeRotation(document.getElementById('setup-bg-rotation')?.value || state.config.keys_bg_rotation || 0, 0),
        bgShape: normalizeShape(document.getElementById('setup-bg-shape')?.value || state.config.keys_bg_shape || 'rounded', 'rounded'),
        bgLayer: parseInt(document.getElementById('setup-bg-layer')?.value || state.config.bg_layer || 6) || 6,
        trailLayer: parseInt(document.getElementById('setup-trail-layer')?.value || state.config.trail_layer || 8) || 8,
        keyLayer: parseInt(document.getElementById('setup-key-layer')?.value || state.config.key_layer || 10) || 10,
        particleEnabled: document.getElementById('setup-particles')?.checked ?? true,
        particleCount: parseInt(document.getElementById('setup-particle-count')?.value || 8) || 0,
        particleMinSize: parseInt(document.getElementById('setup-particle-min-size')?.value || 1) || 1,
        particleMaxSize: parseInt(document.getElementById('setup-particle-size')?.value || 4) || 1,
        particleSpread: parseInt(document.getElementById('setup-particle-spread')?.value || 6) || 0,
        particleLife: parseFloat(document.getElementById('setup-particle-life')?.value || 1) || 0.1,
        particleGravity: parseFloat(document.getElementById('setup-particle-gravity')?.value || 0.2) || 0,
        particleSpeed: parseFloat(document.getElementById('setup-particle-speed')?.value || 8) || 0,
        particleRgb: document.getElementById('setup-particle-rgb')?.checked ?? false,
        particleShape: document.getElementById('setup-particle-shape')?.value || 'square',
    };
}

function getPreviewKeyRect(i, metrics = getEditorMetrics()) {
    return {
        x: metrics.startX + i * (metrics.size + metrics.gap) + metrics.offX[i],
        y: metrics.baseY + metrics.offY[i],
        w: metrics.size,
        h: metrics.keyH,
    };
}

function getPreviewTrailGeometry(i, metrics = getEditorMetrics()) {
    const isLocked = state.config.lock_trails !== false;
    const tOffX = state.config.trail_offsets_x || [0, 0, 0, 0];
    const tOffY = state.config.trail_offsets_y || [0, 0, 0, 0];
    const keyRect = getPreviewKeyRect(i, metrics);
    const keyBounds = getTransformedBounds(
        keyRect.x,
        keyRect.y,
        keyRect.w,
        keyRect.h,
        normalizeScale(metrics.keyScales[i], 1),
        normalizeRotation(metrics.keyRotations[i], 0),
    );
    const { anchorX, anchorY } = resolveTrailAnchor({
        lockTrails: isLocked,
        transformedCenterX: keyBounds.x + keyBounds.w / 2,
        transformedTopY: keyBounds.y,
        keyOffsetX: metrics.offX[i],
        keyOffsetY: metrics.offY[i],
        trailOffsetX: tOffX[i],
        trailOffsetY: tOffY[i],
    });

    return {
        anchorX,
        anchorY,
        width: metrics.trailWidths[i] || metrics.size,
        scale: metrics.trailScales[i] || 1,
        rotation: metrics.trailRotations[i] || 0,
        shape: metrics.trailShapes[i] || 'rounded',
        guideHeight: Math.max(8, Math.min(metrics.H, anchorY)),
    };
}

function getTrailBounds(geometry, height = geometry.guideHeight) {
    return getSharedTrailBounds({
        anchorX: geometry.anchorX,
        anchorY: geometry.anchorY,
        width: geometry.width,
        height,
        scale: geometry.scale,
        rotation: geometry.rotation,
    });
}

function getBackgroundRect(metrics = getEditorMetrics()) {
    const autoW = metrics.totalW + metrics.bgPadding * 2;
    const autoH = metrics.keyH + metrics.bgPadding * 2;
    const width = metrics.bgWidth > 0 ? metrics.bgWidth : autoW;
    const height = metrics.bgHeight > 0 ? metrics.bgHeight : autoH;
    return {
        x: metrics.startX - (width - metrics.totalW) / 2 + metrics.bgOffsetX,
        y: metrics.baseY - (height - metrics.keyH) / 2 + metrics.bgOffsetY,
        w: width,
        h: height,
        radius: metrics.bgRadius,
        enabled: metrics.bgEnabled,
    };
}

function getBackgroundHandleRect(rect) {
    const scale = normalizeScale(state.config.keys_bg_scale, 1);
    const rotation = normalizeRotation(state.config.keys_bg_rotation, 0);
    const bounds = getTransformedBounds(rect.x, rect.y, rect.w, rect.h, scale, rotation);
    const size = 14;
    return {
        x: bounds.x + bounds.w - size - 4,
        y: bounds.y + bounds.h - size - 4,
        w: size,
        h: size,
    };
}

function syncBackgroundInputsFromState() {
    const pairs = [
        ['setup-bg-offset-x', 'val-bg-offset-x', state.config.keys_bg_offset_x ?? 0],
        ['setup-bg-offset-y', 'val-bg-offset-y', state.config.keys_bg_offset_y ?? 0],
        ['setup-bg-width', null, state.config.keys_bg_width ?? 0],
        ['setup-bg-height', null, state.config.keys_bg_height ?? 0],
        ['setup-bg-padding', 'val-bg-padding', state.config.keys_bg_padding ?? 15],
        ['setup-bg-radius', 'val-bg-radius', state.config.keys_bg_radius ?? 16],
        ['setup-bg-scale', 'val-bg-scale', state.config.keys_bg_scale ?? 1],
        ['setup-bg-rotation', 'val-bg-rotation', state.config.keys_bg_rotation ?? 0],
        ['setup-particle-count', 'val-particle-count', state.config.particle_count ?? 8],
        ['setup-particle-min-size', 'val-particle-min-size', state.config.particle_min_size ?? 1],
        ['setup-particle-size', 'val-particle-size', state.config.particle_max_size ?? 4],
        ['setup-particle-spread', 'val-particle-spread', state.config.particle_spread ?? 6],
        ['setup-particle-life', 'val-particle-life', state.config.particle_life ?? 1],
        ['setup-particle-gravity', 'val-particle-gravity', state.config.particle_gravity ?? 0.2],
        ['setup-particle-speed', 'val-particle-speed', state.config.particle_speed ?? 8],
    ];
    pairs.forEach(([inputId, labelId, value]) => {
        const input = document.getElementById(inputId);
        if (input) input.value = value;
        if (labelId) {
            const label = document.getElementById(labelId);
            if (label) {
                if (inputId === 'setup-particle-life') label.textContent = Number(value).toFixed(1);
                else if (inputId === 'setup-bg-scale') label.textContent = Number(value).toFixed(2);
                else if (inputId === 'setup-bg-rotation') label.textContent = Number(value).toFixed(0);
                else if (inputId === 'setup-particle-gravity' || inputId === 'setup-particle-speed') label.textContent = Number(value).toFixed(2).replace(/\.00$/, '.0');
                else label.textContent = value;
            }
        }
    });

    const bgShapeInput = document.getElementById('setup-bg-shape');
    if (bgShapeInput) bgShapeInput.value = state.config.keys_bg_shape ?? 'rounded';
}

function spawnPreviewParticles(keyIndex, metrics, time) {
    if (!metrics.particleEnabled || metrics.particleCount <= 0) return;
    const keyRect = getPreviewKeyRect(keyIndex, metrics);
    const centerX = keyRect.x + keyRect.w / 2;
    const sourceY = keyRect.y + keyRect.h - 6;
    const minSize = Math.min(metrics.particleMinSize, metrics.particleMaxSize);
    const maxSize = Math.max(metrics.particleMinSize, metrics.particleMaxSize);
    const baseColor = metrics.colors[keyIndex] || '#ffffff';
    const rgb = hexToRgb(baseColor);
    const count = clamp(metrics.particleCount, 0, 60);
    const spreadRadians = (metrics.particleSpread || 0) * (Math.PI / 180);
    const baseAngle = -Math.PI / 2;

    for (let i = 0; i < count; i++) {
        const size = minSize + Math.random() * Math.max(1, maxSize - minSize);
        const angle = baseAngle + (Math.random() - 0.5) * spreadRadians;
        const speed = metrics.particleSpeed * (0.65 + Math.random() * 0.7);
        previewParticles.push({
            active: true,
            x: centerX,
            y: sourceY,
            vx: Math.cos(angle) * speed,
            vy: Math.sin(angle) * speed,
            size,
            radius: Math.max(0.5, size / 2),
            life: metrics.particleLife,
            maxLife: metrics.particleLife,
            color: rgb,
            isRGB: metrics.particleRgb,
            shape: metrics.particleShape,
            hueSeed: time + i * 11,
        });
    }
    if (previewParticles.length > 240) {
        previewParticles = previewParticles.slice(-240);
    }
}

function updatePreviewParticles(deltaFactor, time, metrics) {
    const dt = Math.max(0.001, deltaFactor);
    const gravity = metrics.particleGravity;
    for (let i = previewParticles.length - 1; i >= 0; i--) {
        const p = previewParticles[i];
        if (!p.active) {
            previewParticles.splice(i, 1);
            continue;
        }
        p.x += p.vx * dt;
        p.y += p.vy * dt;
        p.vy += gravity * dt;
        p.life -= dt * 0.016;
        if (p.life <= 0) {
            previewParticles.splice(i, 1);
            continue;
        }
        p.alpha = clamp(p.life / p.maxLife, 0, 1);
        if (p.isRGB) {
            p.hueSeed += dt * 8;
        }
    }
}

function drawPreviewParticles(ctx, time, metrics = getEditorMetrics()) {
    if (!metrics.particleEnabled) return;
    for (const p of previewParticles) {
        if (!p.active) continue;
        let fill = `rgba(${p.color[0]}, ${p.color[1]}, ${p.color[2]}, ${p.alpha ?? 1})`;
        if (p.isRGB) {
            const hue = (time * 0.08 + p.hueSeed) % 360;
            fill = `hsla(${hue}, 100%, 50%, ${p.alpha ?? 1})`;
        }
        ctx.fillStyle = fill;
        if (p.shape === 'circle') {
            ctx.beginPath();
            ctx.arc(p.x, p.y, p.radius || (p.size / 2), 0, Math.PI * 2);
            ctx.fill();
        } else {
            const radius = p.radius || (p.size / 2);
            ctx.fillRect(p.x - radius, p.y - radius, radius * 2, radius * 2);
        }
    }
}

export function initVisualEditor() {
    try {
        const canvas = document.getElementById('editor-canvas');
        if (!canvas) return;
        document.querySelectorAll('.prop-tab').forEach(tab => {
            tab.addEventListener('click', () => {
                document.querySelectorAll('.prop-tab').forEach(t => t.classList.remove('active'));
                document.querySelectorAll('.prop-panel').forEach(p => p.classList.remove('active'));
                tab.classList.add('active');
                document.getElementById(tab.dataset.panel)?.classList.add('active');
            });
        });
        const sliderPairs = [
            ['setup-key-size', 'val-key-size'],
            ['setup-key-gap', 'val-key-gap'],
            ['setup-key-height', 'val-key-height'],
            ['setup-bg-opacity', 'val-bg-opacity'],
            ['setup-bg-offset-x', 'val-bg-offset-x'],
            ['setup-bg-offset-y', 'val-bg-offset-y'],
            ['setup-bg-padding', 'val-bg-padding'],
            ['setup-bg-radius', 'val-bg-radius'],
            ['setup-bg-scale', 'val-bg-scale'],
            ['setup-bg-rotation', 'val-bg-rotation'],
            ['setup-trail-speed', 'val-trail-speed'],
            ['setup-trail-opacity', 'val-trail-opacity'],
            ['setup-trail-fade', 'val-trail-fade'],
            ['setup-particle-count', 'val-particle-count'],
            ['setup-particle-min-size', 'val-particle-min-size'],
            ['setup-particle-size', 'val-particle-size'],
            ['setup-particle-spread', 'val-particle-spread'],
            ['setup-particle-life', 'val-particle-life'],
            ['setup-particle-gravity', 'val-particle-gravity'],
            ['setup-particle-speed', 'val-particle-speed'],
        ];
        sliderPairs.forEach(([sliderId, labelId]) => {
            const sl = document.getElementById(sliderId);
            const lb = document.getElementById(labelId);
            if (sl && lb) {
                sl.addEventListener('input', () => {
                    if (sliderId === 'setup-particle-life') {
                        lb.textContent = Number(sl.value).toFixed(1);
                    } else if (sliderId === 'setup-bg-scale') {
                        lb.textContent = Number(sl.value).toFixed(2);
                    } else if (sliderId === 'setup-bg-rotation') {
                        lb.textContent = Number(sl.value).toFixed(0);
                    } else if (sliderId === 'setup-particle-gravity' || sliderId === 'setup-particle-speed') {
                        lb.textContent = Number(sl.value).toFixed(2).replace(/\.00$/, '.0');
                    } else {
                        lb.textContent = sl.value;
                    }
                    readAndSaveSetupSettings();
                    renderPreview();
                });
            }
        });
        ['setup-bg-enabled', 'setup-bg-shape', 'setup-particle-rgb', 'setup-particle-shape', 'setup-trails'].forEach(id => {
            const el = document.getElementById(id);
            if (el) {
                el.addEventListener('change', () => {
                    readAndSaveSetupSettings();
                    renderPreview();
                });
            }
        });
        document.getElementById('setup-particles')?.addEventListener('change', (e) => {
            if (!e.target.checked) {
                previewParticles = [];
            }
            readAndSaveSetupSettings();
            renderPreview();
        });
        ['setup-bg-width', 'setup-bg-height'].forEach(id => {
            const el = document.getElementById(id);
            if (el) {
                el.addEventListener('input', () => {
                    readAndSaveSetupSettings();
                    renderPreview();
                });
            }
        });
        ['setup-bg-layer', 'setup-trail-layer', 'setup-key-layer'].forEach(id => {
            const el = document.getElementById(id);
            if (el) {
                el.addEventListener('input', () => {
                    readAndSaveSetupSettings();
                    renderPreview();
                });
            }
        });
        document.getElementById('setup-bg-reset-btn')?.addEventListener('click', () => {
            saveHistory();
            state.config.keys_bg_enabled = true;
            state.config.keys_bg_offset_x = 0;
            state.config.keys_bg_offset_y = 0;
            state.config.keys_bg_width = 0;
            state.config.keys_bg_height = 0;
            state.config.keys_bg_padding = 15;
            state.config.keys_bg_radius = 16;
            state.config.keys_bg_scale = 1;
            state.config.keys_bg_rotation = 0;
            state.config.keys_bg_shape = 'rounded';
            populateSettingsPanel();
            saveVisualSettings();
            renderPreview();
        });
        document.getElementById('setup-trail-height-num')?.addEventListener('input', () => {
            readAndSaveSetupSettings();
            renderPreview();
        });
        document.getElementById('setup-lock-trails')?.addEventListener('change', (e) => {
            state.config.lock_trails = e.target.checked;
            populateSettingsPanel();
            saveVisualSettings();
            renderPreview();
        });
        for (let i = 0; i < 4; i++) {
            document.getElementById(`setup-key-color-${i}`)?.addEventListener('input', () => {
                updateKeyColorBadge(i);
                renderPreview();
                populateSettingsPanel(); saveVisualSettings();
            });
        }
        document.getElementById('editor-preset')?.addEventListener('change', (e) => {
            const v = e.target.value;
            if (!v) return;
            saveHistory();
            if (v === 'classic') {
                state.config.key_offsets_x = [0,0,0,0];
                state.config.key_offsets_y = [0,0,0,0];
            } else if (v === 'vshape') {
                state.config.key_offsets_y = [-20, 0, 0, -20];
                state.config.key_offsets_x = [0,0,0,0];
            } else if (v === 'arc') {
                state.config.key_offsets_y = [0, -20, -20, 0];
                state.config.key_offsets_x = [-10, -5, 5, 10];
            }
            populateSettingsPanel();
            saveVisualSettings();
            renderPreview();
            e.target.value = '';
        });

        document.getElementById('editor-undo')?.addEventListener('click', undo);
        document.getElementById('editor-redo')?.addEventListener('click', redo);
        document.getElementById('editor-export')?.addEventListener('click', exportLayout);
        document.getElementById('editor-import')?.addEventListener('click', () => {
            document.getElementById('editor-import-file')?.click();
        });
        document.getElementById('editor-import-file')?.addEventListener('change', importLayout);
        document.getElementById('editor-preview-btn')?.addEventListener('click', togglePreview);
        const ws = document.getElementById('editor-workspace');
        if (ws) {
            canvas.addEventListener('mousedown', onCanvasMouseDown);
            canvas.addEventListener('contextmenu', onCanvasRightClick);
            ws.addEventListener('mousemove', onMouseMove);
            ws.addEventListener('mouseup', onMouseUp);
            ws.addEventListener('mouseleave', onMouseUp);
        }


        document.getElementById('ctx-reset')?.addEventListener('click', () => {
            if (menuActiveKeyIndex === -1) return;
            saveHistory();
            const applyAll = document.getElementById('ctx-apply-all')?.checked || false;
            if (menuActiveIsTrail) {
                if (!state.config.trail_offsets_x) state.config.trail_offsets_x = [0,0,0,0];
                if (!state.config.trail_offsets_y) state.config.trail_offsets_y = [0,0,0,0];
                if (applyAll) {
                    state.config.trail_offsets_x = [0,0,0,0];
                    state.config.trail_offsets_y = [0,0,0,0];
                } else {
                    state.config.trail_offsets_x[menuActiveKeyIndex] = 0;
                    state.config.trail_offsets_y[menuActiveKeyIndex] = 0;
                }
            } else {
                if (!state.config.key_offsets_x) state.config.key_offsets_x = [0,0,0,0];
                if (!state.config.key_offsets_y) state.config.key_offsets_y = [0,0,0,0];
                if (applyAll) {
                    state.config.key_offsets_x = [0,0,0,0];
                    state.config.key_offsets_y = [0,0,0,0];
                } else {
                    state.config.key_offsets_x[menuActiveKeyIndex] = 0;
                    state.config.key_offsets_y[menuActiveKeyIndex] = 0;
                }
            }
            document.getElementById('ctx-x').value = 0;
            document.getElementById('ctx-y').value = 0;
            populateSettingsPanel();
            saveVisualSettings();
            renderPreview();
        });

        window.addEventListener('resize', () => {
            schedulePreviewRefresh();
        });

        document.addEventListener('ori:tab-activated', (event) => {
            if (event?.detail?.tabId === 'keys') {
                schedulePreviewRefresh();
            }
        });

        if (isTauri) {
            window.__TAURI__.event.listen('bind-key', (event) => {
                if (!isListeningForBind || menuActiveKeyIndex === -1) return;
                const keyCode = event.payload;
                if (!state.config.keys) state.config.keys = ['KeyD', 'KeyF', 'KeyJ', 'KeyK'];
                const existingIndex = state.config.keys.indexOf(keyCode);
                if (existingIndex !== -1 && existingIndex !== menuActiveKeyIndex) {
                    state.config.keys[existingIndex] = state.config.keys[menuActiveKeyIndex];
                }
                state.config.keys[menuActiveKeyIndex] = keyCode;
                const ctxBind = document.getElementById('ctx-bind');
                if (ctxBind) {
                    ctxBind.value = keyCode.replace('Key', '');
                    ctxBind.style.borderColor = '';
                }
                isListeningForBind = false;
                invoke('set_bind_listening_mode', { listening: false }).catch(() => {});
                populateSettingsPanel();
                saveVisualSettings();
                renderPreview();
            });
        }
        document.getElementById('ctx-close')?.addEventListener('click', () => {
            document.getElementById('context-menu').style.display = 'none';
            menuActiveKeyIndex = -1;
            selectedKeyIndex = -1;
            isListeningForBind = false;
            renderPreview();
        });
        const ctxBind = document.getElementById('ctx-bind');
        if (ctxBind) {
            ctxBind.addEventListener('click', () => {
                isListeningForBind = true;
                ctxBind.value = 'Press a key...';
                ctxBind.style.borderColor = '#a67c52';
                if (isTauri) {
                    invoke('set_bind_listening_mode', { listening: true }).catch(() => {});
                }
            });
        }
        function mapJsCodeToRdev(code) {
            if (code.startsWith('Key')) return code;
            if (code.startsWith('Digit')) return code.replace('Digit', 'Num');
            if (code === 'Semicolon') return 'SemiColon';
            if (code === 'Period') return 'Dot';
            if (code === 'BracketLeft') return 'LeftBracket';
            if (code === 'BracketRight') return 'RightBracket';
            return code;
        }
        document.addEventListener('keydown', (e) => {
            if (!isListeningForBind || menuActiveKeyIndex === -1) return;
            e.preventDefault();
            const rdevCode = mapJsCodeToRdev(e.code);
            if (!state.config.keys) state.config.keys = ['KeyD', 'KeyF', 'KeyJ', 'KeyK'];
            const existingIndex = state.config.keys.indexOf(rdevCode);
            if (existingIndex !== -1 && existingIndex !== menuActiveKeyIndex) {
                state.config.keys[existingIndex] = state.config.keys[menuActiveKeyIndex];
            }
            state.config.keys[menuActiveKeyIndex] = rdevCode;
            const ctxBind = document.getElementById('ctx-bind');
            if (ctxBind) {
                ctxBind.value = rdevCode.replace('Key', '');
                ctxBind.style.borderColor = '';
            }
            isListeningForBind = false;
            if (isTauri) {
                invoke('set_bind_listening_mode', { listening: false }).catch(() => {});
            }
            populateSettingsPanel();
            saveVisualSettings();
            renderPreview();
        });
        ['ctx-label', 'ctx-x', 'ctx-y', 'ctx-width', 'ctx-scale', 'ctx-rotation', 'ctx-shape', 'ctx-rgb', 'ctx-apply-all'].forEach(id => {
            const el = document.getElementById(id);
            if (el) el.addEventListener('change', () => {
                if (menuActiveKeyIndex === -1) return;
                updateScopeIndicator();
                applyContextToState(menuActiveKeyIndex);
                saveVisualSettings();
                renderPreview();
            });
        });
        document.getElementById('ctx-x')?.addEventListener('input', () => {
            if (menuActiveKeyIndex === -1) return;
            applyContextToState(menuActiveKeyIndex);
            renderPreview();
        });
        document.getElementById('ctx-y')?.addEventListener('input', () => {
            if (menuActiveKeyIndex === -1) return;
            applyContextToState(menuActiveKeyIndex);
            renderPreview();
        });
        ['ctx-width', 'ctx-scale', 'ctx-rotation'].forEach(id => document.getElementById(id)?.addEventListener('input', () => {
            if (menuActiveKeyIndex === -1) return;
            applyContextToState(menuActiveKeyIndex);
            renderPreview();
        }));
        syncColorBadges();
        renderPreview();

    } catch (e) {
        console.error('Visual editor init failed:', e);
    }
}

function getLayoutSnapshot() {
    ensureVisualTransformConfig();
    return {
        key_offsets_x: [...(state.config.key_offsets_x || [0, 0, 0, 0])],
        key_offsets_y: [...(state.config.key_offsets_y || [0, 0, 0, 0])],
        key_colors: [...(state.config.key_colors || ['#00d2ff', '#ff007f', '#ff007f', '#00d2ff'])],
        key_scales: [...(state.config.key_scales || [1, 1, 1, 1])],
        key_rotations: [...(state.config.key_rotations || [0, 0, 0, 0])],
        key_shapes: [...(state.config.key_shapes || ['rounded', 'rounded', 'rounded', 'rounded'])],
        key_labels: [...(state.config.key_labels || ['D', 'F', 'J', 'K'])],
        trail_widths: [...(state.config.trail_widths || [50, 50, 50, 50])],
        trail_scales: [...(state.config.trail_scales || [1, 1, 1, 1])],
        trail_rotations: [...(state.config.trail_rotations || [0, 0, 0, 0])],
        trail_shapes: [...(state.config.trail_shapes || ['rounded', 'rounded', 'rounded', 'rounded'])],
        trail_offsets_x: [...(state.config.trail_offsets_x || [0, 0, 0, 0])],
        trail_offsets_y: [...(state.config.trail_offsets_y || [0, 0, 0, 0])],
        rgb_enabled_keys: [...(state.config.rgb_enabled_keys || [false, false, false, false])],
        keys: [...(state.config.keys || ['KeyD', 'KeyF', 'KeyJ', 'KeyK'])],
        key_size: state.config.key_size,
        key_height: state.config.key_height,
        key_gap: state.config.key_gap,
        show_trails: state.config.show_trails,
        show_particles: state.config.show_particles,
        trail_opacity: state.config.trail_opacity,
        trail_fade: state.config.trail_fade,
        trail_speed: state.config.trail_speed,
        trail_height: state.config.trail_height,
        rgb_speed: state.config.rgb_speed,
        keys_bg_color: state.config.keys_bg_color,
        keys_bg_opacity: state.config.keys_bg_opacity,
        keys_bg_enabled: state.config.keys_bg_enabled,
        keys_bg_offset_x: state.config.keys_bg_offset_x,
        keys_bg_offset_y: state.config.keys_bg_offset_y,
        keys_bg_width: state.config.keys_bg_width,
        keys_bg_height: state.config.keys_bg_height,
        keys_bg_radius: state.config.keys_bg_radius,
        keys_bg_padding: state.config.keys_bg_padding,
        keys_bg_scale: state.config.keys_bg_scale,
        keys_bg_rotation: state.config.keys_bg_rotation,
        keys_bg_shape: state.config.keys_bg_shape,
        bg_layer: state.config.bg_layer,
        trail_layer: state.config.trail_layer,
        key_layer: state.config.key_layer,
        particle_count: state.config.particle_count,
        particle_min_size: state.config.particle_min_size,
        particle_max_size: state.config.particle_max_size,
        particle_spread: state.config.particle_spread,
        particle_speed: state.config.particle_speed,
        particle_life: state.config.particle_life,
        particle_gravity: state.config.particle_gravity,
        particle_rgb: state.config.particle_rgb,
        particle_shape: state.config.particle_shape,
        lock_trails: state.config.lock_trails,
    };
}

function saveHistory() {
    history.push(JSON.stringify(getLayoutSnapshot()));
    if (history.length > 20) history.shift();
    redoStack = [];
}

function undo() {
    if (!history.length) return;
    const curr = JSON.stringify(getLayoutSnapshot());
    redoStack.push(curr);
    const snap = JSON.parse(history.pop());
    Object.assign(state.config, snap);
    ensureVisualTransformConfig();
    populateSettingsPanel(); saveVisualSettings();
    renderPreview();
}

function redo() {
    if (!redoStack.length) return;
    const curr = JSON.stringify(getLayoutSnapshot());
    history.push(curr);
    const snap = JSON.parse(redoStack.pop());
    Object.assign(state.config, snap);
    ensureVisualTransformConfig();
    populateSettingsPanel(); saveVisualSettings();
    renderPreview();
}

function exportLayout() {
    const layout = {
        ...getLayoutSnapshot(),
    };
    const content = JSON.stringify(layout, null, 2);

    if (isTauri) {
        invoke('save_text_file', {
            default_name: 'keystrokes-layout.json',
            contents: content,
        }).then((path) => {
            if (!path) return;
            let toast = document.getElementById('autosave-toast');
            if (!toast) {
                toast = document.createElement('div');
                toast.id = 'autosave-toast';
                toast.className = 'autosave-toast';
                document.body.appendChild(toast);
            }
            toast.textContent = `Exported to ${path}`;
            toast.classList.add('visible');
            clearTimeout(window._toastTimeout);
            window._toastTimeout = setTimeout(() => {
                toast.classList.remove('visible');
                toast.textContent = 'Saved automatically';
            }, 1800);
        }).catch((err) => {
            console.error('Export failed:', err);
        });
        return;
    }

    const a = document.createElement('a');
    a.href = 'data:text/json;charset=utf-8,' + encodeURIComponent(content);
    a.download = 'keystrokes-layout.json';
    a.click();
}

function importLayout(e) {
    const file = e.target.files[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (ev) => {
        try {
            const data = JSON.parse(ev.target.result);
            saveHistory();
            Object.assign(state.config, data);
            ensureVisualTransformConfig();
            syncColorBadges();
            populateSettingsPanel(); saveVisualSettings();
            renderPreview();
        } catch {
            alert('Invalid layout file');
        }
    };
    reader.readAsText(file);
    e.target.value = '';
}

function togglePreview() {
    isPreviewPlaying = !isPreviewPlaying;
    const btn = document.getElementById('editor-preview-btn');
    if (btn) btn.textContent = isPreviewPlaying ? 'Stop' : 'Preview';
    if (isPreviewPlaying) {
        previewTrailHeights = [0, 0, 0, 0];
        previewParticles = [];
        previewLastTime = 0;
        runPreviewAnimation();
    } else {
        cancelAnimationFrame(previewAnimationFrame);
        previewTrailHeights = [0, 0, 0, 0];
        previewKeyStates = [false, false, false, false];
        previewParticles = [];
        renderPreview();
    }
}

function runPreviewAnimation() {
    if (!isPreviewPlaying) return;
    const metrics = getEditorMetrics();
    const time = Date.now();
    const elapsed = previewLastTime ? (time - previewLastTime) / 16.666 : 1;
    previewLastTime = time;

    for (let i = 0; i < 4; i++) {
        const nextPressed = Math.sin(time * 0.0045 + i * 1.5) > 0.25;
        if (nextPressed && !previewKeyStates[i]) {
            spawnPreviewParticles(i, metrics, time);
        }
        previewKeyStates[i] = nextPressed;
        
        if (previewKeyStates[i]) {
            previewTrailHeights[i] = Math.min(metrics.trailHeight, previewTrailHeights[i] + metrics.trailSpeed * 2);
        } else {
            previewTrailHeights[i] = Math.max(0, previewTrailHeights[i] - metrics.trailSpeed * 1.2);
        }
    }
    updatePreviewParticles(elapsed, time, metrics);
    renderPreview();
    previewAnimationFrame = requestAnimationFrame(runPreviewAnimation);
}

function schedulePreviewRefresh(frameCount = 2) {
    if (scheduledPreviewFrame) {
        cancelAnimationFrame(scheduledPreviewFrame);
        scheduledPreviewFrame = null;
    }

    const run = (remaining) => {
        scheduledPreviewFrame = requestAnimationFrame(() => {
            if (remaining > 1) {
                run(remaining - 1);
                return;
            }

            scheduledPreviewFrame = null;
            renderPreview();
        });
    };

    run(Math.max(1, frameCount));
}

function getKeyColors() {
    const colors = state.config.key_colors;
    if (colors && colors.length === 4) return colors;
    return [0,1,2,3].map(i => {
        return document.getElementById(`setup-key-color-${i}`)?.value || ['#00d2ff','#ff007f','#ff007f','#00d2ff'][i];
    });
}

function hexToRgb(hex) {
    const r = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
    return r ? [parseInt(r[1],16), parseInt(r[2],16), parseInt(r[3],16)] : [0,210,255];
}


function hslToRgb(h, s, l) {
    h = (h % 360 + 360) % 360;
    s /= 100;
    l /= 100;
    const k = n => (n + h / 30) % 12;
    const a = s * Math.min(l, 1 - l);
    const f = n => l - a * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1));
    return [Math.round(255 * f(0)), Math.round(255 * f(8)), Math.round(255 * f(4))];
}

const rgbToHex = (r, g, b) => '#' + [r, g, b].map(x => x.toString(16).padStart(2, '0')).join('');

export function renderPreview() {
    const startedAt = performance.now();
    const canvas = document.getElementById('editor-canvas');
    const ws = document.getElementById('editor-workspace');
    if (!canvas || !ws) return;

    const W = ws.clientWidth || 400;
    const H = ws.clientHeight || 350;
    canvas.width = W;
    canvas.height = H;
    const ctx = canvas.getContext('2d');
    ctx.fillStyle = '#1a1a2e';
    ctx.fillRect(0, 0, W, H);

    const metrics = getEditorMetrics();
    const bgRect = getBackgroundRect(metrics);
    const bgRgb = hexToRgb(metrics.bgColor);
    const time = Date.now();
    const drawBackgroundLayer = () => {
        const bgBounds = getTransformedBounds(bgRect.x, bgRect.y, bgRect.w, bgRect.h, metrics.bgScale, metrics.bgRotation);
        if (metrics.bgEnabled) {
            withElementTransform(ctx, bgRect.x, bgRect.y, bgRect.w, bgRect.h, metrics.bgScale, metrics.bgRotation, () => {
                ctx.save();
                ctx.globalAlpha = metrics.bgOpacity;
                ctx.fillStyle = `rgb(${bgRgb[0]},${bgRgb[1]},${bgRgb[2]})`;
                traceShapePath(ctx, bgRect.x, bgRect.y, bgRect.w, bgRect.h, metrics.bgShape, metrics.bgRadius);
                ctx.fill();
                ctx.restore();
                ctx.save();
                ctx.strokeStyle = 'rgba(255,255,255,0.1)';
                ctx.lineWidth = 1;
                traceShapePath(ctx, bgRect.x, bgRect.y, bgRect.w, bgRect.h, metrics.bgShape, metrics.bgRadius);
                ctx.stroke();
                ctx.restore();
            });
        } else if (selectedBackground || draggedBackground) {
            ctx.save();
            ctx.strokeStyle = 'rgba(255,255,255,0.18)';
            ctx.lineWidth = 1.5;
            ctx.setLineDash([6, 4]);
            ctx.strokeRect(bgBounds.x, bgBounds.y, bgBounds.w, bgBounds.h);
            ctx.restore();
        }
    };

    const drawTrailLayer = () => {
        if (metrics.showTrails && state.config.lock_trails === false) {
            ctx.save();
            ctx.strokeStyle = 'rgba(255, 255, 255, 0.08)';
            ctx.lineWidth = 1;
            ctx.setLineDash([5, 5]);
            const colW = W / 4;
            for (let i = 1; i < 4; i++) {
                ctx.beginPath();
                ctx.moveTo(i * colW, 0);
                ctx.lineTo(i * colW, H);
                ctx.stroke();
            }
            ctx.restore();
        }

        if (metrics.showTrails) {
            for (let i = 0; i < 4; i++) {
                const trailH = previewTrailHeights[i] || 0;
                const geometry = getPreviewTrailGeometry(i, metrics);
                const clampedTrailY = Math.max(0, Math.min(H, geometry.anchorY));
                const isSelectedTrail = selectedKeyIndex === i && selectedIsTrail === true;
                const guideBounds = getTrailBounds(geometry, geometry.guideHeight);

                ctx.save();
                ctx.translate(geometry.anchorX, geometry.anchorY);
                ctx.rotate(toRadians(geometry.rotation));
                ctx.setLineDash(isSelectedTrail ? [4, 4] : [3, 5]);
                ctx.lineWidth = isSelectedTrail ? 1.5 : 1;
                ctx.strokeStyle = isSelectedTrail ? metrics.colors[i] : 'rgba(255,255,255,0.22)';
                traceShapePath(
                    ctx,
                    -(geometry.width * geometry.scale) / 2,
                    -(geometry.guideHeight * geometry.scale),
                    geometry.width * geometry.scale,
                    geometry.guideHeight * geometry.scale,
                    geometry.shape,
                    10,
                );
                ctx.stroke();
                ctx.setLineDash([]);
                ctx.fillStyle = isSelectedTrail ? metrics.colors[i] : 'rgba(255,255,255,0.34)';
                ctx.fillRect(-(geometry.width * geometry.scale) / 2, -2, geometry.width * geometry.scale, 2);
                ctx.restore();

                ctx.save();
                ctx.fillStyle = isSelectedTrail ? metrics.colors[i] : 'rgba(255,255,255,0.34)';
                ctx.font = '700 10px Nunito, sans-serif';
                ctx.textAlign = 'center';
                ctx.fillText(`T${i + 1}`, guideBounds.x + guideBounds.w / 2, Math.max(10, guideBounds.y - 6));
                ctx.restore();

                if (isSelectedTrail) {
                    ctx.save();
                    ctx.strokeStyle = metrics.colors[i];
                    ctx.lineWidth = 1.5;
                    ctx.setLineDash([4, 4]);
                    ctx.strokeRect(guideBounds.x, guideBounds.y, guideBounds.w, guideBounds.h);
                    ctx.fillStyle = metrics.colors[i];
                    ctx.font = '600 10px Inter, sans-serif';
                    ctx.textAlign = 'center';
                    ctx.fillText('Selected Trail', guideBounds.x + guideBounds.w / 2, Math.max(15, guideBounds.y - 10));
                    ctx.restore();
                }

                if (trailH >= 2) {
                    const rgb = hexToRgb(metrics.colors[i]);
                    const scaledTrailHeight = trailH * geometry.scale;
                    const grad = ctx.createLinearGradient(0, clampedTrailY, 0, clampedTrailY - scaledTrailHeight);
                    grad.addColorStop(0, `rgba(${rgb[0]},${rgb[1]},${rgb[2]},${metrics.trailOpacity})`);
                    grad.addColorStop(1, `rgba(${rgb[0]},${rgb[1]},${rgb[2]},${metrics.trailFade})`);
                    ctx.save();
                    ctx.translate(geometry.anchorX, geometry.anchorY);
                    ctx.rotate(toRadians(geometry.rotation));
                    ctx.fillStyle = grad;
                    traceShapePath(
                        ctx,
                        -(geometry.width * geometry.scale) / 2,
                        -scaledTrailHeight,
                        geometry.width * geometry.scale,
                        scaledTrailHeight,
                        geometry.shape,
                        10,
                    );
                    ctx.fill();
                    ctx.restore();
                }
            }
        }

        drawPreviewParticles(ctx, time, metrics);
    };

    const drawKeyLayer = () => {
        for (let i = 0; i < 4; i++) {
            const keyRect = getPreviewKeyRect(i, metrics);
            const x = keyRect.x;
            const y = keyRect.y;

            let col = metrics.colors[i];
            let rgb = hexToRgb(col);
            if (state.config.rgb_enabled_keys?.[i]) {
                const hue = (time * (state.config.rgb_speed || 1.0) * 0.08) % 360;
                rgb = hslToRgb(hue, 100, 50);
                col = rgbToHex(rgb[0], rgb[1], rgb[2]);
            }

            const isSelected = selectedKeyIndex === i && selectedIsTrail === false;
            const isActive = isPreviewPlaying && previewKeyStates[i];
            const keyScale = normalizeScale(metrics.keyScales[i], 1);
            const keyRotation = normalizeRotation(metrics.keyRotations[i], 0);
            const keyShape = normalizeShape(metrics.keyShapes[i], 'rounded');
            const keyBounds = getTransformedBounds(x, y, metrics.size, metrics.keyH, keyScale * (isActive ? 0.96 : 1), keyRotation);

            withElementTransform(ctx, x, y, metrics.size, metrics.keyH, keyScale * (isActive ? 0.96 : 1), keyRotation, () => {
                ctx.save();
                ctx.fillStyle = isActive ? `rgba(${rgb[0]}, ${rgb[1]}, ${rgb[2]}, 0.35)` : 'rgba(255,255,255,0.05)';
                ctx.strokeStyle = isSelected || isActive ? col : 'rgba(255,255,255,0.1)';
                ctx.lineWidth = isSelected || isActive ? 2 : 1;
                if (isActive) {
                    ctx.shadowColor = col;
                    ctx.shadowBlur = 15;
                }
                traceShapePath(ctx, x, y, metrics.size, metrics.keyH, keyShape, 12);
                ctx.fill();
                ctx.stroke();
                ctx.restore();

                ctx.fillStyle = 'rgba(255,255,255,0.6)';
                ctx.font = `700 ${Math.round(metrics.size * 0.28)}px Inter, sans-serif`;
                ctx.textAlign = 'center';
                ctx.textBaseline = 'middle';
                ctx.fillText(metrics.labels[i] || ['D','F','J','K'][i], x + metrics.size / 2, y + metrics.keyH / 2 - 6);
                ctx.beginPath();
                ctx.arc(x + metrics.size / 2, y + metrics.keyH - 10, 3, 0, Math.PI * 2);
                ctx.fillStyle = col;
                ctx.fill();
            });

            if (isSelected) {
                ctx.save();
                ctx.shadowColor = col;
                ctx.shadowBlur = 12;
                ctx.strokeStyle = col;
                ctx.lineWidth = 2;
                ctx.strokeRect(keyBounds.x, keyBounds.y, keyBounds.w, keyBounds.h);
                ctx.restore();
                ctx.fillStyle = col;
                ctx.font = `600 10px Inter, sans-serif`;
                ctx.textAlign = 'center';
                ctx.fillText('Selected', keyBounds.x + keyBounds.w / 2, keyBounds.y - 8);
            }
        }
    };

    [
        { layer: metrics.bgLayer, draw: drawBackgroundLayer },
        { layer: metrics.trailLayer, draw: drawTrailLayer },
        { layer: metrics.keyLayer, draw: drawKeyLayer },
    ]
        .sort((a, b) => a.layer - b.layer)
        .forEach(({ draw }) => draw());

    if (selectedBackground || draggedBackground) {
        const bgBounds = getTransformedBounds(bgRect.x, bgRect.y, bgRect.w, bgRect.h, metrics.bgScale, metrics.bgRotation);
        ctx.save();
        ctx.strokeStyle = metrics.bgEnabled ? 'rgba(166, 124, 82, 0.95)' : 'rgba(255,255,255,0.45)';
        ctx.lineWidth = 2;
        ctx.setLineDash([6, 4]);
        ctx.strokeRect(bgBounds.x, bgBounds.y, bgBounds.w, bgBounds.h);
        ctx.setLineDash([]);
        const handle = getBackgroundHandleRect(bgRect);
        ctx.fillStyle = 'rgba(255,255,255,0.9)';
        ctx.fillRect(handle.x, handle.y, handle.w, handle.h);
        ctx.strokeStyle = 'rgba(166, 124, 82, 0.95)';
        ctx.strokeRect(handle.x, handle.y, handle.w, handle.h);
        ctx.fillStyle = 'rgba(166, 124, 82, 0.95)';
        ctx.font = '600 10px Inter, sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText('Background', bgBounds.x + bgBounds.w / 2, bgBounds.y - 8);
        ctx.restore();
    }

    if (draggedKey) {
        const i = draggedKey.index;
        const tooltip = document.getElementById('editor-tooltip');
        if (tooltip) {
            tooltip.style.display = 'block';
            tooltip.textContent = `X: ${metrics.offX[i] >= 0 ? '+' : ''}${metrics.offX[i]}, Y: ${metrics.offY[i] >= 0 ? '+' : ''}${metrics.offY[i]}`;
        }
    } else {
        const tooltip = document.getElementById('editor-tooltip');
        if (tooltip) tooltip.style.display = 'none';
    }

    recordPerf('keystrokes_preview', performance.now() - startedAt);
}

function getKeyAtPoint(canvasX, canvasY) {
    ensureVisualTransformConfig();
    const canvas = document.getElementById('editor-canvas');
    const ws = document.getElementById('editor-workspace');
    if (!canvas || !ws) return -1;

    const W = canvas.width;
    const H = canvas.height;
    const size = parseInt(document.getElementById('setup-key-size')?.value || 60);
    const keyH = parseInt(document.getElementById('setup-key-height')?.value || size);
    const gap = parseInt(document.getElementById('setup-key-gap')?.value || 10);
    const offX = state.config.key_offsets_x || [0,0,0,0];
    const offY = state.config.key_offsets_y || [0,0,0,0];
    const keyScales = state.config.key_scales || [1,1,1,1];
    const keyRotations = state.config.key_rotations || [0,0,0,0];
    const bgPadding = parseInt(document.getElementById('setup-bg-padding')?.value || 15) || 0;
    const totalW = size * 4 + gap * 3;
    const startX = (W - totalW) / 2;
    const baseY = H - keyH - 20 - bgPadding;

    for (let i = 0; i < 4; i++) {
        const x = startX + i * (size + gap) + offX[i];
        const y = baseY + offY[i];
        const bounds = getTransformedBounds(x, y, size, keyH, normalizeScale(keyScales[i], 1), normalizeRotation(keyRotations[i], 0));
        if (canvasX >= bounds.x && canvasX <= bounds.x + bounds.w && canvasY >= bounds.y && canvasY <= bounds.y + bounds.h) {
            return i;
        }
    }
    return -1;
}

function getTrailAtPoint(canvasX, canvasY) {
    ensureVisualTransformConfig();
    const metrics = getEditorMetrics();
    for (let i = 0; i < 4; i++) {
        const geometry = getPreviewTrailGeometry(i, metrics);
        const bounds = getTrailBounds(geometry, geometry.guideHeight);
        if (canvasX >= bounds.x && canvasX <= bounds.x + bounds.w && canvasY >= bounds.y && canvasY <= bounds.y + bounds.h) {
            return i;
        }
    }
    return -1;
}

function unlockTrailsForEditing() {
    if (state.config.lock_trails === false) return;

    state.config.lock_trails = false;

    const lockEl = document.getElementById('setup-lock-trails');
    if (lockEl) lockEl.checked = false;
    const trailOffsetsGroup = document.getElementById('trail-offsets-group');
    if (trailOffsetsGroup) trailOffsetsGroup.style.display = 'flex';
}

function onCanvasMouseDown(e) {
    if (e.button !== 0) return;
    const rect = e.target.getBoundingClientRect();
    const cx = e.clientX - rect.left;
    const cy = e.clientY - rect.top;
    const metrics = getEditorMetrics();
    
    const trailIdx = getTrailAtPoint(cx, cy);
    if (trailIdx !== -1) {
        unlockTrailsForEditing();
        selectedKeyIndex = trailIdx;
        selectedIsTrail = true;
        selectedBackground = false;
        document.getElementById('context-menu').style.display = 'none';
        saveHistory();
        draggedKey = {
            isTrail: true,
            index: trailIdx,
            startClientX: e.clientX,
            startClientY: e.clientY,
            origOffX: (state.config.trail_offsets_x || [0,0,0,0])[trailIdx],
            origOffY: (state.config.trail_offsets_y || [0,0,0,0])[trailIdx],
        };
        populateSettingsPanel();
        renderPreview();
        return;
    }

    const idx = getKeyAtPoint(cx, cy);
    if (idx !== -1) {
        selectedKeyIndex = idx;
        selectedIsTrail = false;
        selectedBackground = false;
        document.getElementById('context-menu').style.display = 'none';
        saveHistory();
        draggedKey = {
            index: idx,
            startClientX: e.clientX,
            startClientY: e.clientY,
            origOffX: (state.config.key_offsets_x || [0,0,0,0])[idx],
            origOffY: (state.config.key_offsets_y || [0,0,0,0])[idx],
        };
        renderPreview();
        return;
    }

    const bgRect = getBackgroundRect(metrics);
    const handleRect = getBackgroundHandleRect(bgRect);
    const bgBounds = getTransformedBounds(bgRect.x, bgRect.y, bgRect.w, bgRect.h, metrics.bgScale, metrics.bgRotation);
    const inBackground = cx >= bgBounds.x && cx <= bgBounds.x + bgBounds.w && cy >= bgBounds.y && cy <= bgBounds.y + bgBounds.h;
    if (metrics.bgEnabled && inBackground) {
        selectedKeyIndex = -1;
        selectedIsTrail = false;
        selectedBackground = true;
        document.getElementById('context-menu').style.display = 'none';
        saveHistory();
        draggedBackground = {
            mode: cx >= handleRect.x && cx <= handleRect.x + handleRect.w && cy >= handleRect.y && cy <= handleRect.y + handleRect.h ? 'resize' : 'move',
            startClientX: e.clientX,
            startClientY: e.clientY,
            origOffsetX: metrics.bgOffsetX,
            origOffsetY: metrics.bgOffsetY,
            origWidth: metrics.bgWidth > 0 ? metrics.bgWidth : bgRect.w,
            origHeight: metrics.bgHeight > 0 ? metrics.bgHeight : bgRect.h,
        };
        renderPreview();
        return;
    }
    
    selectedKeyIndex = -1;
    selectedIsTrail = false;
    selectedBackground = false;
    document.getElementById('context-menu').style.display = 'none';
    renderPreview();
}

function onCanvasRightClick(e) {
    e.preventDefault();
    const rect = e.target.getBoundingClientRect();
    const cx = e.clientX - rect.left;
    const cy = e.clientY - rect.top;

    const trailIdx = getTrailAtPoint(cx, cy);
    if (trailIdx !== -1) {
        unlockTrailsForEditing();
        selectedKeyIndex = trailIdx;
        selectedIsTrail = true;
        selectedBackground = false;
        populateSettingsPanel();
        renderPreview();
        openContextMenu(e.clientX, e.clientY, trailIdx, true);
        return;
    }

    const idx = getKeyAtPoint(cx, cy);
    if (idx !== -1) {
        selectedKeyIndex = idx;
        selectedIsTrail = false;
        renderPreview();
        openContextMenu(e.clientX, e.clientY, idx, false);
        return;
    }

    const metrics = getEditorMetrics();
    const bgRect = getBackgroundRect(metrics);
    const bgBounds = getTransformedBounds(bgRect.x, bgRect.y, bgRect.w, bgRect.h, metrics.bgScale, metrics.bgRotation);
    if (metrics.bgEnabled && cx >= bgBounds.x && cx <= bgBounds.x + bgBounds.w && cy >= bgBounds.y && cy <= bgBounds.y + bgBounds.h) {
        selectedKeyIndex = -1;
        selectedIsTrail = false;
        selectedBackground = true;
        renderPreview();
    }
}

function onMouseMove(e) {
    if (!draggedKey && !draggedBackground) return;

    if (draggedBackground) {
        const dx = e.clientX - draggedBackground.startClientX;
        const dy = e.clientY - draggedBackground.startClientY;
        const snap = e.altKey ? 1 : 10;
        const snapValue = (value) => snap > 1 ? Math.round(value / snap) * snap : value;

        if (draggedBackground.mode === 'resize') {
            const newWidth = Math.max(40, snapValue(draggedBackground.origWidth + dx));
            const newHeight = Math.max(40, snapValue(draggedBackground.origHeight + dy));
            state.config.keys_bg_width = newWidth;
            state.config.keys_bg_height = newHeight;
            const widthEl = document.getElementById('setup-bg-width');
            const heightEl = document.getElementById('setup-bg-height');
            if (widthEl) widthEl.value = newWidth;
            if (heightEl) heightEl.value = newHeight;
        } else {
            const newOffsetX = snapValue(draggedBackground.origOffsetX + dx);
            const newOffsetY = snapValue(draggedBackground.origOffsetY + dy);
            state.config.keys_bg_offset_x = newOffsetX;
            state.config.keys_bg_offset_y = newOffsetY;
            const offXEl = document.getElementById('setup-bg-offset-x');
            const offYEl = document.getElementById('setup-bg-offset-y');
            const offXVal = document.getElementById('val-bg-offset-x');
            const offYVal = document.getElementById('val-bg-offset-y');
            if (offXEl) offXEl.value = newOffsetX;
            if (offYEl) offYEl.value = newOffsetY;
            if (offXVal) offXVal.textContent = newOffsetX;
            if (offYVal) offYVal.textContent = newOffsetY;
        }

        renderPreview();
        return;
    }

    const dx = e.clientX - draggedKey.startClientX;
    const dy = e.clientY - draggedKey.startClientY;

    let newX = draggedKey.origOffX + dx;
    let newY = draggedKey.origOffY + dy;

    if (!e.altKey) {
        const snap = 10;
        newX = Math.round(newX / snap) * snap;
        newY = Math.round(newY / snap) * snap;
    }

    if (draggedKey.isTrail) {
        if (!state.config.trail_offsets_x) state.config.trail_offsets_x = [0,0,0,0];
        if (!state.config.trail_offsets_y) state.config.trail_offsets_y = [0,0,0,0];
        state.config.trail_offsets_x[draggedKey.index] = newX;
        state.config.trail_offsets_y[draggedKey.index] = newY;
        if (selectedKeyIndex === draggedKey.index && selectedIsTrail === true) {
            const cx = document.getElementById('ctx-x');
            const cy = document.getElementById('ctx-y');
            if (cx) cx.value = newX;
            if (cy) cy.value = newY;
        }
    } else {
        if (!state.config.key_offsets_x) state.config.key_offsets_x = [0,0,0,0];
        if (!state.config.key_offsets_y) state.config.key_offsets_y = [0,0,0,0];
        state.config.key_offsets_x[draggedKey.index] = newX;
        state.config.key_offsets_y[draggedKey.index] = newY;

        if (selectedKeyIndex === draggedKey.index && selectedIsTrail === !!draggedKey.isTrail) {
            const cx = document.getElementById('ctx-x');
            const cy = document.getElementById('ctx-y');
            if (cx) cx.value = newX;
            if (cy) cy.value = newY;
        }
    }

    const tooltip = document.getElementById('editor-tooltip');
    if (tooltip) {
        const ws = document.getElementById('editor-workspace');
        const wsRect = ws.getBoundingClientRect();
        tooltip.style.left = (e.clientX - wsRect.left + 12) + 'px';
        tooltip.style.top = (e.clientY - wsRect.top - 24) + 'px';
    }

    renderPreview();
}

function onMouseUp() {
    if (draggedKey || draggedBackground) {
        populateSettingsPanel();
        saveVisualSettings();
        draggedKey = null;
        draggedBackground = null;
        renderPreview();
    }
}

function openContextMenu(x, y, i, isTrail = false) {
    ensureVisualTransformConfig();
    menuActiveKeyIndex = i;
    menuActiveIsTrail = isTrail;
    const menu = document.getElementById('context-menu');
    menu.style.display = 'block';
    menu.style.left = Math.min(x, window.innerWidth - 240) + 'px';
    menu.style.top = Math.min(y, window.innerHeight - 430) + 'px';

    const header = menu.querySelector('.context-header');
    if (header) {
        header.innerHTML = isTrail 
            ? `Trail <span id="ctx-index">${i + 1}</span> Settings` 
            : `Key <span id="ctx-index">${i + 1}</span> Settings`;
    }

    const bindRow = document.getElementById('ctx-bind')?.closest('.ctx-row');
    const labelRow = document.getElementById('ctx-label')?.closest('.ctx-row');
    const rgbRow = document.getElementById('ctx-rgb')?.closest('.ctx-row');
    if (bindRow) bindRow.style.display = isTrail ? 'none' : 'flex';
    if (labelRow) labelRow.style.display = isTrail ? 'none' : 'flex';
    if (rgbRow) rgbRow.style.display = isTrail ? 'none' : 'flex';

    if (isTrail) {
        document.getElementById('ctx-x').value = state.config.trail_offsets_x?.[i] || 0;
        document.getElementById('ctx-y').value = state.config.trail_offsets_y?.[i] || 0;
        document.getElementById('ctx-scale').value = normalizeScale(state.config.trail_scales?.[i], 1).toFixed(2);
        document.getElementById('ctx-rotation').value = normalizeRotation(state.config.trail_rotations?.[i], 0).toFixed(0);
        document.getElementById('ctx-shape').value = normalizeShape(state.config.trail_shapes?.[i], 'rounded');
    } else {
        document.getElementById('ctx-x').value = state.config.key_offsets_x?.[i] || 0;
        document.getElementById('ctx-y').value = state.config.key_offsets_y?.[i] || 0;
        document.getElementById('ctx-scale').value = normalizeScale(state.config.key_scales?.[i], 1).toFixed(2);
        document.getElementById('ctx-rotation').value = normalizeRotation(state.config.key_rotations?.[i], 0).toFixed(0);
        document.getElementById('ctx-shape').value = normalizeShape(state.config.key_shapes?.[i], 'rounded');
        const keys = state.config.keys || ['KeyD', 'KeyF', 'KeyJ', 'KeyK'];
        const labels = state.config.key_labels || ['D', 'F', 'J', 'K'];
        document.getElementById('ctx-bind').value = (keys[i] || '').replace('Key', '');
        document.getElementById('ctx-label').value = labels[i] || '';
    }

    document.getElementById('ctx-width').value = state.config.trail_widths?.[i] || 50;
    document.getElementById('ctx-rgb').checked = state.config.rgb_enabled_keys?.[i] || false;

    const applyAllCheckbox = document.getElementById('ctx-apply-all');
    if (applyAllCheckbox) applyAllCheckbox.checked = false;

    updateScopeIndicator();
    isListeningForBind = false;
}

function updateScopeIndicator() {
    const indicator = document.getElementById('ctx-scope-indicator');
    const applyAll = document.getElementById('ctx-apply-all')?.checked || false;
    if (indicator) {
        if (applyAll) {
            indicator.textContent = menuActiveIsTrail 
                ? 'Configuring all trails (Global)' 
                : 'Configuring all keys (Global)';
        } else {
            indicator.textContent = menuActiveIsTrail 
                ? `Configuring specific trail (Trail ${menuActiveKeyIndex + 1})` 
                : `Configuring specific key (Key ${menuActiveKeyIndex + 1})`;
        }
    }
}

function applyContextToState(i) {
    if (menuActiveKeyIndex === -1) return;
    ensureVisualTransformConfig();
    const idxToUse = menuActiveKeyIndex;

    if (!state.config.key_labels) state.config.key_labels = ['D','F','J','K'];
    if (!state.config.key_offsets_x) state.config.key_offsets_x = [0,0,0,0];
    if (!state.config.key_offsets_y) state.config.key_offsets_y = [0,0,0,0];
    if (!state.config.trail_offsets_x) state.config.trail_offsets_x = [0,0,0,0];
    if (!state.config.trail_offsets_y) state.config.trail_offsets_y = [0,0,0,0];
    if (!state.config.trail_widths) state.config.trail_widths = [50,50,50,50];
    if (!state.config.rgb_enabled_keys) state.config.rgb_enabled_keys = [false,false,false,false];

    const applyAll = document.getElementById('ctx-apply-all')?.checked || false;
    const xVal = parseInt(document.getElementById('ctx-x')?.value) || 0;
    const yVal = parseInt(document.getElementById('ctx-y')?.value) || 0;
    const wVal = parseInt(document.getElementById('ctx-width')?.value) || 50;
    const scaleVal = normalizeScale(document.getElementById('ctx-scale')?.value, 1);
    const rotationVal = normalizeRotation(document.getElementById('ctx-rotation')?.value, 0);
    const shapeVal = normalizeShape(document.getElementById('ctx-shape')?.value, 'rounded');
    const rgbVal = document.getElementById('ctx-rgb')?.checked || false;

    if (applyAll) {
        for (let idx = 0; idx < 4; idx++) {
            if (menuActiveIsTrail) {
                state.config.trail_offsets_x[idx] = xVal;
                state.config.trail_offsets_y[idx] = yVal;
                state.config.trail_scales[idx] = scaleVal;
                state.config.trail_rotations[idx] = rotationVal;
                state.config.trail_shapes[idx] = shapeVal;
            } else {
                state.config.key_offsets_x[idx] = xVal;
                state.config.key_offsets_y[idx] = yVal;
                state.config.key_scales[idx] = scaleVal;
                state.config.key_rotations[idx] = rotationVal;
                state.config.key_shapes[idx] = shapeVal;
                state.config.rgb_enabled_keys[idx] = rgbVal;
            }
            state.config.trail_widths[idx] = wVal;
        }
    } else {
        if (menuActiveIsTrail) {
            state.config.trail_offsets_x[idxToUse] = xVal;
            state.config.trail_offsets_y[idxToUse] = yVal;
            state.config.trail_scales[idxToUse] = scaleVal;
            state.config.trail_rotations[idxToUse] = rotationVal;
            state.config.trail_shapes[idxToUse] = shapeVal;
        } else {
            state.config.key_labels[idxToUse] = document.getElementById('ctx-label')?.value || '';
            state.config.key_offsets_x[idxToUse] = xVal;
            state.config.key_offsets_y[idxToUse] = yVal;
            state.config.key_scales[idxToUse] = scaleVal;
            state.config.key_rotations[idxToUse] = rotationVal;
            state.config.key_shapes[idxToUse] = shapeVal;
            state.config.rgb_enabled_keys[idxToUse] = rgbVal;
        }
        state.config.trail_widths[idxToUse] = wVal;
    }
}

function syncColorBadges() {
    for (let i = 0; i < 4; i++) {
        const col = state.config.key_colors?.[i];
        if (col) {
            const picker = document.getElementById(`setup-key-color-${i}`);
            if (picker) picker.value = col;
        }
        updateKeyColorBadge(i);
    }
}

function updateKeyColorBadge(i) {
    const col = document.getElementById(`setup-key-color-${i}`)?.value || '#00d2ff';
    const badge = document.getElementById(`key-badge-${i}`);
    const item = document.getElementById(`key-color-item-${i}`);
    if (badge) {
        badge.style.setProperty('background', col, 'important');
        badge.style.setProperty('box-shadow', `0 0 0 1px ${col}44 inset`, 'important');
    }
    if (item) {
        item.style.setProperty('border-color', `${col}cc`, 'important');
        item.style.setProperty('box-shadow', `inset 0 1px 0 rgba(255,255,255,0.72), 0 0 0 1px ${col}33`, 'important');
    }
    if (!state.config.key_colors) state.config.key_colors = ['#00d2ff','#ff007f','#ff007f','#00d2ff'];
    state.config.key_colors[i] = col;
}
