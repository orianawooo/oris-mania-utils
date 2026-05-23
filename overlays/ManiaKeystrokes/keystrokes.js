import {
    normalizeRotation,
    normalizeScale,
    normalizeShape,
    resolveTrailAnchor,
    toRadians,
    traceShapePath,
} from './keystrokes-geometry.js';

const canvas = document.getElementById('keystrokes-canvas');
const ctx = canvas.getContext('2d', { alpha: true, desynchronized: true });

function applySavedTheme() {
    const theme = localStorage.getItem('theme') === 'dark' ? 'dark' : 'light';
    document.documentElement.dataset.theme = theme;
    document.body.dataset.theme = theme;
}

applySavedTheme();

let canvasWidth = 300;
let canvasHeight = 720;
let trailMaxHeight = 800;
const numKeys = 4;

let keyStates = [false, false, false, false];
let keyPressTimes = [0, 0, 0, 0];
let activeBlocks = [];
const MAX_PARTICLES = 300;
const particlesPool = [];
for (let i = 0; i < MAX_PARTICLES; i++) {
    particlesPool.push({
        active: false,
        x: 0,
        y: 0,
        vx: 0,
        vy: 0,
        radius: 0,
        size: 0,
        color: null,
        alpha: 0,
        life: 0,
        maxLife: 0,
        isRGB: false
    });
}
let showParticles = true;
let showTrails = true;
let trailOpacity = 0.6;
let trailFade = 0.0;
let trailWidths = [50, 50, 50, 50];
let lockTrails = true;
let trailOffsetsX = [0, 0, 0, 0];
let trailOffsetsY = [0, 0, 0, 0];
let speed = 6;
let rgbSpeed = 1.0;
let rgbEnabledKeys = [false, false, false, false];
let keyLabels = ["D", "F", "J", "K"];
let keyOffsetsX = [0, 0, 0, 0];
let keyOffsetsY = [0, 0, 0, 0];
let keyYPositions = [0, 0, 0, 0];
let keysBgEnabled = true;
let keysBgOffsetX = 0;
let keysBgOffsetY = 0;
let keysBgWidth = 0;
let keysBgHeight = 0;
let keysBgRadius = 16;
let keysBgPadding = 15;
let keysBgScale = 1;
let keysBgRotation = 0;
let keysBgShape = 'rounded';
let bgLayer = 6;
let trailLayer = 8;
let keyLayer = 10;
let particleCount = 8;
let particleMinSize = 1;
let particleMaxSize = 4;
let particleSpread = 6;
let particleSpeed = 8;
let particleLife = 1.0;
let particleGravity = 0.2;
let particleRgb = false;
let particleShape = 'square';
let keyScales = [1, 1, 1, 1];
let keyRotations = [0, 0, 0, 0];
let keyShapes = ['rounded', 'rounded', 'rounded', 'rounded'];
let trailScales = [1, 1, 1, 1];
let trailRotations = [0, 0, 0, 0];
let trailShapes = ['rounded', 'rounded', 'rounded', 'rounded'];
let debugOverlayGuides = new URLSearchParams(window.location.search).get('debugGuides') === '1';

const colors = [
    [0, 210, 255],
    [255, 0, 127],
    [255, 0, 127],
    [0, 210, 255]
];

const keyBindings = ['KeyD', 'KeyF', 'KeyJ', 'KeyK'];
let columnGradients = [];

let keyXPositions = [37.5, 112.5, 187.5, 262.5];
let keyWidths = [60, 60, 60, 60];
let keySize = 60;
let keyHeight = 60;
let keyGap = 10;
let keysFont = '';
let geometrySyncFrame = null;

function arraysEqual(left, right) {
    return Array.isArray(left)
        && Array.isArray(right)
        && left.length === right.length
        && left.every((value, index) => value === right[index]);
}

function hexToRgbColor(hex) {
    const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
    return result ? [
        parseInt(result[1], 16),
        parseInt(result[2], 16),
        parseInt(result[3], 16)
    ] : null;
}

function ensureVisualArrays() {
    keyScales = Array.from({ length: 4 }, (_, i) => normalizeScale(keyScales[i], 1));
    keyRotations = Array.from({ length: 4 }, (_, i) => normalizeRotation(keyRotations[i], 0));
    keyShapes = Array.from({ length: 4 }, (_, i) => normalizeShape(keyShapes[i], 'rounded'));
    trailScales = Array.from({ length: 4 }, (_, i) => normalizeScale(trailScales[i], 1));
    trailRotations = Array.from({ length: 4 }, (_, i) => normalizeRotation(trailRotations[i], 0));
    trailShapes = Array.from({ length: 4 }, (_, i) => normalizeShape(trailShapes[i], 'rounded'));
    keysBgScale = normalizeScale(keysBgScale, 1);
    keysBgRotation = normalizeRotation(keysBgRotation, 0);
    keysBgShape = normalizeShape(keysBgShape, 'rounded');
}

function drawTrailBlock(centerX, baseY, width, height, scale, rotation, shape, fillStyle) {
    const scaledWidth = Math.max(4, width * normalizeScale(scale, 1));
    const scaledHeight = Math.max(1, height * normalizeScale(scale, 1));
    ctx.save();
    ctx.translate(centerX, baseY);
    ctx.rotate(toRadians(rotation));
    ctx.fillStyle = fillStyle;
    traceShapePath(ctx, -(scaledWidth / 2), -scaledHeight, scaledWidth, scaledHeight, shape, 10);
    ctx.fill();
    ctx.restore();
}

function getTrailAnchorForIndex(index) {
    return resolveTrailAnchor({
        lockTrails,
        transformedCenterX: keyXPositions[index] || 0,
        transformedTopY: keyYPositions[index] || canvasHeight,
        keyOffsetX: keyOffsetsX[index] || 0,
        keyOffsetY: keyOffsetsY[index] || 0,
        trailOffsetX: trailOffsetsX[index] || 0,
        trailOffsetY: trailOffsetsY[index] || 0,
    });
}

function applyKeyVisuals() {
    ensureVisualArrays();
    for (let i = 0; i < 4; i++) {
        const keyEl = document.getElementById(`key-${i}`);
        if (!keyEl) continue;
        keyEl.style.setProperty('--key-offset-x', `${keyOffsetsX[i] || 0}px`);
        keyEl.style.setProperty('--key-offset-y', `${keyOffsetsY[i] || 0}px`);
        keyEl.style.setProperty('--key-scale', `${normalizeScale(keyScales[i], 1)}`);
        keyEl.style.setProperty('--key-rotation', `${normalizeRotation(keyRotations[i], 0)}deg`);
        keyEl.dataset.shape = normalizeShape(keyShapes[i], 'rounded');
    }
}

function updateGradients() {
    columnGradients = [];
    for (let i = 0; i < numKeys; i++) {
        const grad = ctx.createLinearGradient(0, canvasHeight, 0, 0);
        if (colors[i]) {
            grad.addColorStop(0, `rgba(${colors[i][0]}, ${colors[i][1]}, ${colors[i][2]}, ${trailOpacity})`);
            grad.addColorStop(1, `rgba(${colors[i][0]}, ${colors[i][1]}, ${colors[i][2]}, ${trailFade})`);
        }
        columnGradients.push(grad);
    }
}

function resizeCanvas() {
    const wrapper = document.querySelector('.overlay-wrapper');
    const container = document.getElementById('keystrokes-container');
    const wrapperRect = wrapper?.getBoundingClientRect();

    if (wrapperRect) {
        canvasWidth = Math.max(1, Math.round(wrapperRect.width));
        canvasHeight = Math.max(1, Math.round(wrapperRect.height));
    }

    canvas.width = canvasWidth;
    canvas.height = canvasHeight;

    if (container) {
        const canvasRect = canvas.getBoundingClientRect();
        for (let i = 0; i < numKeys; i++) {
            const keyEl = document.getElementById(`key-${i}`);
            if (keyEl) {
                const keyRect = keyEl.getBoundingClientRect();
                // Use canvas coordinates so trails match in-game overlay space.
                keyXPositions[i] = (keyRect.left - canvasRect.left) + keyRect.width / 2;
                keyYPositions[i] = keyRect.top - canvasRect.top;
                keyWidths[i] = keyRect.width;
            } else {
                keyXPositions[i] = i * (canvasWidth / numKeys) + (canvasWidth / numKeys) / 2;
                keyYPositions[i] = Math.max(0, canvasHeight - 60);
                keyWidths[i] = canvasWidth / numKeys;
            }
        }
    }
    updateGradients();
}

function updateBackgroundFrame() {
    const frame = document.getElementById('keystrokes-frame');
    if (!frame) return;
    ensureVisualArrays();

    const container = document.getElementById('keystrokes-container');
    const width = keysBgWidth > 0
        ? `${keysBgWidth}px`
        : `${(container?.offsetWidth || 0)}px`;
    const height = keysBgHeight > 0
        ? `${keysBgHeight}px`
        : `${(container?.offsetHeight || 0)}px`;

    frame.style.width = width;
    frame.style.height = height;
    frame.style.borderRadius = `${keysBgRadius}px`;
    frame.style.setProperty('--bg-offset-x', `${keysBgOffsetX}px`);
    frame.style.setProperty('--bg-offset-y', `${keysBgOffsetY}px`);
    frame.style.setProperty('--bg-padding', `${keysBgPadding}px`);
    frame.style.setProperty('--bg-scale', `${keysBgScale}`);
    frame.style.setProperty('--bg-rotation', `${keysBgRotation}deg`);
    frame.dataset.shape = keysBgShape;
    frame.classList.toggle('bg-disabled', !keysBgEnabled);
}

function updateLayerOrder() {
    const root = document.documentElement;
    root.style.setProperty('--bg-layer', String(bgLayer));
    root.style.setProperty('--trail-layer', String(trailLayer));
    root.style.setProperty('--key-layer', String(keyLayer));
}

function parseLayer(value, fallback) {
    const n = Number(value);
    return Number.isFinite(n) ? Math.max(0, Math.min(40, Math.round(n))) : fallback;
}

function scheduleGeometrySync(frameCount = 2) {
    if (geometrySyncFrame) {
        cancelAnimationFrame(geometrySyncFrame);
        geometrySyncFrame = null;
    }

    const run = (remaining) => {
        geometrySyncFrame = requestAnimationFrame(() => {
            resizeCanvas();
            if (remaining > 1) {
                run(remaining - 1);
                return;
            }
            geometrySyncFrame = null;
        });
    };

    run(Math.max(1, frameCount));
}

function updateDisplayedKeyLabels() {
    for (let i = 0; i < numKeys; i++) {
        const keyEl = document.getElementById(`key-${i}`);
        const label = keyEl?.querySelector('.key-label');
        if (label) {
            const binding = keyBindings[i] || '';
            label.textContent = (keyLabels && keyLabels[i] && keyLabels[i].trim() !== '')
                ? keyLabels[i]
                : binding.replace('Key', '');
        }
    }
}

function applyKeyColor(index, hex) {
    document.documentElement.style.setProperty(`--key-color-${index}`, hex);
    const rgb = hexToRgbColor(hex);
    if (rgb) colors[index] = rgb;
}

window.addEventListener('resize', () => scheduleGeometrySync());
resizeCanvas();
applyKeyVisuals();
updateBackgroundFrame();
updateLayerOrder();

const containerEl = document.getElementById('keystrokes-container');
if (containerEl) {
    const ro = new ResizeObserver(() => {
        scheduleGeometrySync(1);
    });
    ro.observe(containerEl);
    for (let i = 0; i < 4; i++) {
        const kEl = document.getElementById(`key-${i}`);
        if (kEl) ro.observe(kEl);
    }
}

function connect() {
    const ws = new WebSocket('ws://127.0.0.1:24051');

    ws.onopen = () => {
        document.getElementById('debug-info').textContent = "Connected to keys server (Rust)";
    };

    ws.onmessage = (event) => {
        const data = JSON.parse(event.data);
        handleKeyData(data);
    };

    ws.onclose = () => {
        document.getElementById('debug-info').textContent = "Disconnected. Retrying...";
        setTimeout(connect, 2000);
    };

    ws.onerror = () => {
        document.getElementById('debug-info').textContent = "Connection error.";
    };
}

function handleKeyData(data) {
    if (data.event === 'bindings' || data.event === 'config-updated') {
        const scopes = Array.isArray(data.changed_scopes) ? data.changed_scopes : [];
        if (data.event === 'config-updated' && scopes.length > 0 && !scopes.includes('ManiaKeystrokes') && !scopes.includes('all')) {
            return;
        }

        let needsGradientRefresh = false;
        let needsGeometrySync = false;
        let needsKeyVisualRefresh = false;
        let needsBackgroundRefresh = false;
        let needsLayerRefresh = false;
        let needsLabelRefresh = false;

        if (Array.isArray(data.key_labels) && !arraysEqual(keyLabels, data.key_labels)) {
            keyLabels = [...data.key_labels];
            needsLabelRefresh = true;
        }

        if (Array.isArray(data.keys) && !arraysEqual(keyBindings, data.keys)) {
            for (let i = 0; i < Math.min(numKeys, data.keys.length); i++) {
                keyBindings[i] = data.keys[i];
            }
            needsLabelRefresh = true;
        }

        if (data.show_particles !== undefined && showParticles !== data.show_particles) {
            showParticles = data.show_particles;
            if (!showParticles) {
                for (const p of particlesPool) {
                    p.active = false;
                }
            }
        }

        if (data.show_trails !== undefined) {
            showTrails = data.show_trails;
        }
        if (data.debug_overlay_guides !== undefined) {
            debugOverlayGuides = !!data.debug_overlay_guides;
        }
        if (data.trail_opacity !== undefined && trailOpacity !== data.trail_opacity) {
            trailOpacity = data.trail_opacity;
            needsGradientRefresh = true;
        }
        if (data.trail_fade !== undefined && trailFade !== data.trail_fade) {
            trailFade = data.trail_fade;
            needsGradientRefresh = true;
        }
        if (data.keys_bg_opacity !== undefined) {
            document.documentElement.style.setProperty('--bg-opacity', data.keys_bg_opacity);
        }
        if (data.keys_bg_color !== undefined) {
            document.documentElement.style.setProperty('--bg-color-raw', data.keys_bg_color);
        }

        if (data.keys_bg_enabled !== undefined && keysBgEnabled !== data.keys_bg_enabled) {
            keysBgEnabled = data.keys_bg_enabled;
            needsBackgroundRefresh = true;
        }
        if (data.keys_bg_offset_x !== undefined && keysBgOffsetX !== data.keys_bg_offset_x) {
            keysBgOffsetX = data.keys_bg_offset_x;
            needsBackgroundRefresh = true;
        }
        if (data.keys_bg_offset_y !== undefined && keysBgOffsetY !== data.keys_bg_offset_y) {
            keysBgOffsetY = data.keys_bg_offset_y;
            needsBackgroundRefresh = true;
        }
        if (data.keys_bg_width !== undefined && keysBgWidth !== data.keys_bg_width) {
            keysBgWidth = data.keys_bg_width;
            needsBackgroundRefresh = true;
        }
        if (data.keys_bg_height !== undefined && keysBgHeight !== data.keys_bg_height) {
            keysBgHeight = data.keys_bg_height;
            needsBackgroundRefresh = true;
        }
        if (data.keys_bg_radius !== undefined && keysBgRadius !== data.keys_bg_radius) {
            keysBgRadius = data.keys_bg_radius;
            needsBackgroundRefresh = true;
        }
        if (data.keys_bg_padding !== undefined && keysBgPadding !== data.keys_bg_padding) {
            keysBgPadding = data.keys_bg_padding;
            needsBackgroundRefresh = true;
        }
        if (data.keys_bg_scale !== undefined) {
            const next = normalizeScale(data.keys_bg_scale, 1);
            if (keysBgScale !== next) {
                keysBgScale = next;
                needsBackgroundRefresh = true;
            }
        }
        if (data.keys_bg_rotation !== undefined) {
            const next = normalizeRotation(data.keys_bg_rotation, 0);
            if (keysBgRotation !== next) {
                keysBgRotation = next;
                needsBackgroundRefresh = true;
            }
        }
        if (data.keys_bg_shape !== undefined) {
            const next = normalizeShape(data.keys_bg_shape, 'rounded');
            if (keysBgShape !== next) {
                keysBgShape = next;
                needsBackgroundRefresh = true;
            }
        }

        if (data.bg_layer !== undefined) {
            const next = parseLayer(data.bg_layer, bgLayer);
            if (bgLayer !== next) {
                bgLayer = next;
                needsLayerRefresh = true;
            }
        }
        if (data.trail_layer !== undefined) {
            const next = parseLayer(data.trail_layer, trailLayer);
            if (trailLayer !== next) {
                trailLayer = next;
                needsLayerRefresh = true;
            }
        }
        if (data.key_layer !== undefined) {
            const next = parseLayer(data.key_layer, keyLayer);
            if (keyLayer !== next) {
                keyLayer = next;
                needsLayerRefresh = true;
            }
        }

        if (data.keys_font !== undefined && keysFont !== data.keys_font) {
            keysFont = data.keys_font;
            document.body.style.fontFamily = `"${data.keys_font}", sans-serif`;
            needsGeometrySync = true;
        }

        if (Array.isArray(data.rgb_enabled_keys) && !arraysEqual(rgbEnabledKeys, data.rgb_enabled_keys)) {
            rgbEnabledKeys = [...data.rgb_enabled_keys];
        }
        if (data.rgb_speed !== undefined) {
            rgbSpeed = data.rgb_speed;
        }
        if (data.particle_count !== undefined) {
            particleCount = data.particle_count;
        }
        if (data.particle_min_size !== undefined) {
            particleMinSize = data.particle_min_size;
        }
        if (data.particle_max_size !== undefined) {
            particleMaxSize = data.particle_max_size;
        }
        if (data.particle_spread !== undefined) {
            particleSpread = data.particle_spread;
        }
        if (data.particle_speed !== undefined) {
            particleSpeed = data.particle_speed;
        }
        if (data.particle_life !== undefined) {
            particleLife = data.particle_life;
        }
        if (data.particle_gravity !== undefined) {
            particleGravity = data.particle_gravity;
        }
        if (data.particle_rgb !== undefined) {
            particleRgb = data.particle_rgb;
        }
        if (data.particle_shape !== undefined) {
            particleShape = data.particle_shape;
        }

        if (Array.isArray(data.key_scales) && !arraysEqual(keyScales, data.key_scales)) {
            keyScales = [...data.key_scales];
            needsKeyVisualRefresh = true;
            needsGeometrySync = true;
        }
        if (Array.isArray(data.key_rotations) && !arraysEqual(keyRotations, data.key_rotations)) {
            keyRotations = [...data.key_rotations];
            needsKeyVisualRefresh = true;
            needsGeometrySync = true;
        }
        if (Array.isArray(data.key_shapes) && !arraysEqual(keyShapes, data.key_shapes)) {
            keyShapes = [...data.key_shapes];
            needsKeyVisualRefresh = true;
            needsGeometrySync = true;
        }
        if (Array.isArray(data.trail_widths) && !arraysEqual(trailWidths, data.trail_widths)) {
            trailWidths = [...data.trail_widths];
        }
        if (Array.isArray(data.trail_scales) && !arraysEqual(trailScales, data.trail_scales)) {
            trailScales = [...data.trail_scales];
        }
        if (Array.isArray(data.trail_rotations) && !arraysEqual(trailRotations, data.trail_rotations)) {
            trailRotations = [...data.trail_rotations];
        }
        if (Array.isArray(data.trail_shapes) && !arraysEqual(trailShapes, data.trail_shapes)) {
            trailShapes = [...data.trail_shapes];
        }
        if (data.lock_trails !== undefined) {
            lockTrails = data.lock_trails;
        }
        if (Array.isArray(data.trail_offsets_x) && !arraysEqual(trailOffsetsX, data.trail_offsets_x)) {
            trailOffsetsX = [...data.trail_offsets_x];
        }
        if (Array.isArray(data.trail_offsets_y) && !arraysEqual(trailOffsetsY, data.trail_offsets_y)) {
            trailOffsetsY = [...data.trail_offsets_y];
        }
        if (data.trail_speed !== undefined) {
            speed = data.trail_speed;
        }
        if (data.trail_height !== undefined) {
            trailMaxHeight = data.trail_height;
        }

        if (Array.isArray(data.key_offsets_x) && !arraysEqual(keyOffsetsX, data.key_offsets_x)) {
            keyOffsetsX = [...data.key_offsets_x];
            needsKeyVisualRefresh = true;
            needsGeometrySync = true;
        }
        if (Array.isArray(data.key_offsets_y) && !arraysEqual(keyOffsetsY, data.key_offsets_y)) {
            keyOffsetsY = [...data.key_offsets_y];
            needsKeyVisualRefresh = true;
            needsGeometrySync = true;
        }

        if (Array.isArray(data.key_colors) && data.key_colors.length >= numKeys) {
            for (let i = 0; i < numKeys; i++) {
                const nextColor = data.key_colors[i];
                const nextRgb = hexToRgbColor(nextColor);
                if (nextColor && nextRgb && (colors[i][0] !== nextRgb[0] || colors[i][1] !== nextRgb[1] || colors[i][2] !== nextRgb[2])) {
                    applyKeyColor(i, nextColor);
                    needsGradientRefresh = true;
                }
            }
        } else {
            if (data.key_color_outer) {
                const nextRgb = hexToRgbColor(data.key_color_outer);
                if (nextRgb && (colors[0][0] !== nextRgb[0] || colors[0][1] !== nextRgb[1] || colors[0][2] !== nextRgb[2] || colors[3][0] !== nextRgb[0] || colors[3][1] !== nextRgb[1] || colors[3][2] !== nextRgb[2])) {
                    applyKeyColor(0, data.key_color_outer);
                    applyKeyColor(3, data.key_color_outer);
                    needsGradientRefresh = true;
                }
            }
            if (data.key_color_inner) {
                const nextRgb = hexToRgbColor(data.key_color_inner);
                if (nextRgb && (colors[1][0] !== nextRgb[0] || colors[1][1] !== nextRgb[1] || colors[1][2] !== nextRgb[2] || colors[2][0] !== nextRgb[0] || colors[2][1] !== nextRgb[1] || colors[2][2] !== nextRgb[2])) {
                    applyKeyColor(1, data.key_color_inner);
                    applyKeyColor(2, data.key_color_inner);
                    needsGradientRefresh = true;
                }
            }
        }

        if (data.key_size !== undefined && keySize !== Number(data.key_size)) {
            keySize = Number(data.key_size);
            document.documentElement.style.setProperty('--key-size', `${keySize}px`);
            needsGeometrySync = true;
        }
        if (data.key_height !== undefined && keyHeight !== Number(data.key_height)) {
            keyHeight = Number(data.key_height);
            document.documentElement.style.setProperty('--key-height', `${keyHeight}px`);
            needsGeometrySync = true;
        }
        if (data.key_gap !== undefined && keyGap !== Number(data.key_gap)) {
            keyGap = Number(data.key_gap);
            document.documentElement.style.setProperty('--key-gap', `${keyGap}px`);
            needsGeometrySync = true;
        }

        if (needsLabelRefresh) {
            updateDisplayedKeyLabels();
        }
        if (needsKeyVisualRefresh) {
            applyKeyVisuals();
        }
        if (needsBackgroundRefresh) {
            updateBackgroundFrame();
        }
        if (needsLayerRefresh) {
            updateLayerOrder();
        }
        if (needsGradientRefresh) {
            updateGradients();
        }
        if (needsGeometrySync) {
            scheduleGeometrySync();
        }

        document.getElementById('debug-info').textContent = `Keys: ${keyBindings.map(k => k.replace('Key','')).join(', ')}`;
        wakeUp();
        return;
    }

    if (data.event === 'key-down' || data.event === 'key-up') {
        const index = Number.isInteger(data.index) ? data.index : -1;
        if (index < 0 || index >= keyBindings.length) return;

        document.getElementById('debug-info').textContent = `Received: ${data.event} ${index}`;

        if (data.event === 'key-down') {
            handleKeyPress(index);
        } else {
            handleKeyRelease(index);
        }
        return;
    }
}

function drawDebugGuides() {
    if (!debugOverlayGuides) return;

    ctx.save();
    ctx.setLineDash([4, 4]);
    ctx.lineWidth = 1;
    ctx.font = '11px Inter, sans-serif';
    ctx.textAlign = 'center';

    for (let i = 0; i < numKeys; i++) {
        const { anchorX: centerX, anchorY: trailBaseY } = getTrailAnchorForIndex(i);

        ctx.strokeStyle = 'rgba(255,255,255,0.22)';
        ctx.beginPath();
        ctx.moveTo(centerX, 0);
        ctx.lineTo(centerX, canvasHeight);
        ctx.stroke();

        ctx.fillStyle = 'rgba(255,255,255,0.8)';
        ctx.fillText(`T${i + 1}`, centerX, Math.max(14, trailBaseY - 8));
    }

    ctx.restore();
}

let isAnimating = false;
let lastFrameTime = 0;

function wakeUp() {
    if (!isAnimating) {
        isAnimating = true;
        lastFrameTime = performance.now();
        requestAnimationFrame(animate);
    }
}

function handleKeyPress(i) {
    if (keyStates[i]) return;
    keyStates[i] = true;
    keyPressTimes[i] = Date.now();

    document.getElementById(`key-${i}`).classList.add('active');

    activeBlocks.push({
        key: i,
        y: keyYPositions[i] || canvasHeight,
        h: 0,
        color: colors[i],
        holding: true
    });

    if (showParticles) {
        createParticles(keyXPositions[i], keyYPositions[i] || canvasHeight, colors[i], rgbEnabledKeys[i]);
    }

    wakeUp();
}

function handleKeyRelease(i) {
    if (!keyStates[i]) return;
    keyStates[i] = false;

    document.getElementById(`key-${i}`).classList.remove('active');

    const duration = Date.now() - keyPressTimes[i];
    document.querySelector(`#key-${i} .key-ms`).textContent = `${duration}ms`;

    activeBlocks.forEach(b => {
        if (b.key === i && b.holding) {
            b.holding = false;
        }
    });

    wakeUp();
}

function createParticles(x, y, color, isRGB) {
    const burstCount = Math.max(0, Math.min(particleCount || 0, particlesPool.length));
    const minSize = Math.min(particleMinSize || 1, particleMaxSize || 1);
    const maxSize = Math.max(particleMinSize || 1, particleMaxSize || 1);
    const spreadRadians = (particleSpread || 0) * (Math.PI / 180);
    const baseAngle = -Math.PI / 2;
    let spawned = 0;
    for (let i = 0; i < particlesPool.length; i++) {
        const p = particlesPool[i];
        if (!p.active) {
            p.active = true;
            p.x = x;
            p.y = y;
            const angle = baseAngle + (Math.random() - 0.5) * spreadRadians;
            const burstSpeed = particleSpeed * (0.65 + Math.random() * 0.7);
            p.vx = Math.cos(angle) * burstSpeed;
            p.vy = Math.sin(angle) * burstSpeed;
            p.size = minSize + Math.random() * Math.max(1, maxSize - minSize);
            p.radius = Math.max(0.5, p.size / 2);
            p.color = color;
            p.alpha = 1;
            p.life = Math.max(0.1, particleLife || 1);
            p.maxLife = Math.max(0.1, particleLife || 1);
            p.isRGB = isRGB || particleRgb;
            p.shape = particleShape;
            spawned++;
            if (spawned >= burstCount) break;
        }
    }

    drawDebugGuides();
}

function animate(currentTime) {
    if (!isAnimating) return;

    requestAnimationFrame(animate);

    if (!currentTime) currentTime = performance.now();
    const elapsed = currentTime - lastFrameTime;
    lastFrameTime = currentTime;

    const deltaFactor = elapsed / 16.66;
    const currentSpeed = speed * deltaFactor;

    ctx.clearRect(0, 0, canvasWidth, canvasHeight);

    let hasActiveElements = false;

    for (let i = activeBlocks.length - 1; i >= 0; i--) {
        const b = activeBlocks[i];
        const { anchorX: trailCenterX, anchorY: trailBaseY } = getTrailAnchorForIndex(b.key);

        if (b.holding) {
            b.h = Math.min(trailMaxHeight, b.h + currentSpeed);
            b.y = trailBaseY - b.h;
            hasActiveElements = true;
        } else {
            b.y -= currentSpeed;
        }

        if (b.y + b.h < 0) {
            activeBlocks.splice(i, 1);
            continue;
        }

        hasActiveElements = true;

        if (showTrails) {
            const tWidth = trailWidths[b.key] || (keyWidths[b.key] - 10) || 50;
            const centerX = trailCenterX;

            let fillStyle = columnGradients[b.key] || '#fff';
            if (rgbEnabledKeys[b.key]) {
                const grad = ctx.createLinearGradient(0, canvasHeight, 0, 0);
                const hue = (currentTime * rgbSpeed * 0.1) % 360;
                grad.addColorStop(0, `hsla(${hue}, 100%, 50%, ${trailOpacity})`);
                grad.addColorStop(1, `hsla(${hue}, 100%, 50%, ${trailFade})`);
                fillStyle = grad;
            }
            drawTrailBlock(
                centerX,
                b.y + b.h,
                tWidth,
                b.h,
                trailScales[b.key] || 1,
                trailRotations[b.key] || 0,
                trailShapes[b.key] || 'rounded',
                fillStyle,
            );
        }
    }

    if (showParticles) {
        for (let i = 0; i < particlesPool.length; i++) {
            const p = particlesPool[i];
            if (p.active) {
                p.x += p.vx * deltaFactor;
                p.y += p.vy * deltaFactor;
                p.vy += particleGravity * deltaFactor;
                p.life -= 0.016 * deltaFactor;
                p.alpha = Math.max(0, p.life / (p.maxLife || 1));

                if (p.life <= 0) {
                    p.active = false;
                    continue;
                }

                hasActiveElements = true;
                if (p.isRGB) {
                    const hue = (currentTime * rgbSpeed * 0.1 + p.x) % 360;
                    ctx.fillStyle = `hsla(${hue}, 100%, 50%, ${p.alpha})`;
                } else {
                    ctx.fillStyle = `rgba(${p.color[0]}, ${p.color[1]}, ${p.color[2]}, ${p.alpha})`;
                }
                const size = p.radius * 2;
                if ((p.shape || particleShape) === 'circle') {
                    ctx.beginPath();
                    ctx.arc(p.x, p.y, p.radius, 0, Math.PI * 2);
                    ctx.fill();
                } else {
                    ctx.fillRect(p.x - p.radius, p.y - p.radius, size, size);
                }
            }
        }
    }

    if (keyStates.some(state => state)) {
        hasActiveElements = true;
    }

    if (!hasActiveElements) {
        isAnimating = false;
    ctx.clearRect(0, 0, canvasWidth, canvasHeight);
    }
}

connect();
