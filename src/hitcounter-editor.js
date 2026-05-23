import { state } from './state.js';
import { applyHitCounterDefaults } from './default-config.js';
import { saveConfig } from './config-store.js';
import { recordPerf } from './perf.js';
import {
    ETTERNA_HIT_STYLE,
    HIT_DEFAULT_COLORS,
    HIT_DEFAULT_LABELS,
    HIT_FONTS,
    HIT_KEYS,
    HIT_PREVIEW_VALUES,
    ITEM_SNAP,
    PART_SNAP,
    applyHitCounterLayout,
    clamp,
    ensureHitCounterConfigState,
    getDefaultHitPositions,
    normalizeHexColor,
    normalizeNumber,
    normalizeScale,
    snapValue,
} from './hitcounter-layout.js';

let initialized = false;
let selectedHitIndex = 0;
let dragState = null;
let previewFitScale = 1;
let pendingLayoutRefreshFrame = 0;

export function ensureHitCounterConfig() {
    return ensureHitCounterConfigState(state.config);
}

function getPreviewRoot() {
    return document.getElementById('hit-editor-preview');
}

function getPreviewWorkspace() {
    return document.getElementById('hit-editor-workspace');
}

function getLiveRoots() {
    const roots = [];
    const managerRoot = document.querySelector('.judgments-grid');
    if (managerRoot) roots.push(managerRoot);
    return roots;
}

function getPreviewItemNodes() {
    return Array.from(getPreviewRoot()?.querySelectorAll('[data-hit-index]') || []);
}

function setStatusText(element, label) {
    if (element) element.textContent = label;
}

function applyLayoutToRoot(root, { preview = false } = {}) {
    const result = applyHitCounterLayout(root, state.config, {
        preview,
        selectedIndex: preview ? selectedHitIndex : -1,
        workspace: preview ? getPreviewWorkspace() : null,
        previewValues: HIT_PREVIEW_VALUES,
    });
    previewFitScale = preview ? result.previewFitScale : 1;
}

export function applyHitCounterToDocument() {
    const startedAt = performance.now();
    ensureHitCounterConfig();
    const previewRoot = getPreviewRoot();
    if (previewRoot) applyLayoutToRoot(previewRoot, { preview: true });
    getLiveRoots().forEach(root => applyLayoutToRoot(root, { preview: false }));
    recordPerf('hitcounter_preview', performance.now() - startedAt);
}

function scheduleHitCounterLayoutRefresh(frameCount = 2) {
    if (pendingLayoutRefreshFrame) {
        cancelAnimationFrame(pendingLayoutRefreshFrame);
        pendingLayoutRefreshFrame = 0;
    }

    const run = (remaining) => {
        pendingLayoutRefreshFrame = requestAnimationFrame(() => {
            if (remaining > 1) {
                run(remaining - 1);
                return;
            }
            pendingLayoutRefreshFrame = 0;
            applyHitCounterToDocument();
        });
    };

    run(Math.max(1, frameCount));
}

function syncControlsFromState() {
    ensureHitCounterConfig();
    const setValue = (id, value) => {
        const el = document.getElementById(id);
        if (el) el.value = value;
    };
    const setText = (id, value) => {
        const el = document.getElementById(id);
        if (el) el.textContent = value;
    };

    setValue('setup-hit-opacity', state.config.hitcounter_opacity);
    setValue('setup-hit-scale', state.config.hitcounter_scale);
    setValue('setup-hit-bg-color', normalizeHexColor(state.config.hitcounter_bg_color, ETTERNA_HIT_STYLE.bg));
    setValue('setup-hit-text-color', normalizeHexColor(state.config.hitcounter_text_color, ETTERNA_HIT_STYLE.text));
    setValue('setup-hit-border-style', state.config.hitcounter_border_style);
    setValue('setup-hit-border-color', normalizeHexColor(state.config.hitcounter_border_color, ETTERNA_HIT_STYLE.borderColor));
    setValue('setup-hit-font', state.config.hitcounter_font);
    setValue('setup-hit-orientation', state.config.hitcounter_orientation);
    setValue('setup-hit-pos-x', state.config.hitcounter_position_x);
    setValue('setup-hit-pos-y', state.config.hitcounter_position_y);
    setValue('setup-hit-padding', state.config.hitcounter_padding);
    setValue('setup-hit-gap', state.config.hitcounter_gap);
    setValue('setup-hit-item-width', state.config.hitcounter_item_width);
    setValue('setup-hit-item-height', state.config.hitcounter_item_height);
    setValue('setup-hit-radius', state.config.hitcounter_item_radius);
    setValue('setup-hit-dot-size', state.config.hitcounter_dot_size);
    setValue('setup-hit-label-size', state.config.hitcounter_label_size);
    setValue('setup-hit-value-size', state.config.hitcounter_value_size);
    setValue('setup-hit-glow', state.config.hitcounter_glow_strength);

    setText('val-hit-padding', state.config.hitcounter_padding);
    setText('val-hit-gap', state.config.hitcounter_gap);
    setText('val-hit-item-width', state.config.hitcounter_item_width);
    setText('val-hit-item-height', state.config.hitcounter_item_height);
    setText('val-hit-radius', state.config.hitcounter_item_radius);
    setText('val-hit-dot-size', state.config.hitcounter_dot_size);
    setText('val-hit-label-size', state.config.hitcounter_label_size);
    setText('val-hit-value-size', state.config.hitcounter_value_size);
    setText('val-hit-glow', Number(state.config.hitcounter_glow_strength).toFixed(2));
    setText('hit-grid-size-readout', `${ITEM_SNAP}px snap`);

    syncSelectedControls();
}

function syncSelectedControls() {
    ensureHitCounterConfig();
    const index = clamp(selectedHitIndex, 0, HIT_KEYS.length - 1);
    const setValue = (id, value) => {
        const el = document.getElementById(id);
        if (el) el.value = value;
    };
    setStatusText(document.getElementById('hit-selected-name'), state.config.hitcounter_labels[index] || HIT_DEFAULT_LABELS[index]);
    setValue('setup-hit-label-text', state.config.hitcounter_labels[index]);
    setValue('setup-hit-item-color', state.config.hitcounter_colors[index]);
    setValue('setup-hit-item-scale', Number(state.config.hitcounter_item_scales[index]).toFixed(2));
    setValue('setup-hit-item-x', state.config.hitcounter_item_offsets_x[index]);
    setValue('setup-hit-item-y', state.config.hitcounter_item_offsets_y[index]);
    setValue('setup-hit-label-x', state.config.hitcounter_label_offsets_x[index]);
    setValue('setup-hit-label-y', state.config.hitcounter_label_offsets_y[index]);
    setValue('setup-hit-value-x', state.config.hitcounter_value_offsets_x[index]);
    setValue('setup-hit-value-y', state.config.hitcounter_value_offsets_y[index]);
    setValue('setup-hit-dot-x', state.config.hitcounter_dot_offsets_x[index]);
    setValue('setup-hit-dot-y', state.config.hitcounter_dot_offsets_y[index]);
}

function persistConfig() {
    document.dispatchEvent(new CustomEvent('config-saved', { detail: { config: state.config } }));
    return saveConfig({ reason: 'hitcounter', debounceMs: 100 }).catch((err) => {
        console.error('Failed to save hit counter config:', err);
    });
}

function applyLayoutDefaults() {
    state.config.hitcounter_orientation = ETTERNA_HIT_STYLE.orientation;
    state.config.hitcounter_opacity = ETTERNA_HIT_STYLE.opacity;
    state.config.hitcounter_scale = ETTERNA_HIT_STYLE.scale;
    state.config.hitcounter_bg_color = ETTERNA_HIT_STYLE.bg;
    state.config.hitcounter_text_color = ETTERNA_HIT_STYLE.text;
    state.config.hitcounter_font = ETTERNA_HIT_STYLE.font;
    state.config.hitcounter_position_x = ETTERNA_HIT_STYLE.positionX;
    state.config.hitcounter_position_y = ETTERNA_HIT_STYLE.positionY;
}

function applyCardStyleDefaults() {
    state.config.hitcounter_padding = ETTERNA_HIT_STYLE.padding;
    state.config.hitcounter_gap = ETTERNA_HIT_STYLE.gap;
    state.config.hitcounter_item_width = ETTERNA_HIT_STYLE.itemWidth;
    state.config.hitcounter_item_height = ETTERNA_HIT_STYLE.itemHeight;
    state.config.hitcounter_item_radius = ETTERNA_HIT_STYLE.itemRadius;
    state.config.hitcounter_border_style = ETTERNA_HIT_STYLE.borderStyle;
    state.config.hitcounter_border_color = ETTERNA_HIT_STYLE.borderColor;
    state.config.hitcounter_dot_size = ETTERNA_HIT_STYLE.dotSize;
    state.config.hitcounter_label_size = ETTERNA_HIT_STYLE.labelSize;
    state.config.hitcounter_value_size = ETTERNA_HIT_STYLE.valueSize;
    state.config.hitcounter_glow_strength = ETTERNA_HIT_STYLE.glow;
}

function applySelectedDefaults(index) {
    const { xs, ys } = getDefaultHitPositions(state.config, state.config.hitcounter_orientation);
    state.config.hitcounter_labels[index] = HIT_DEFAULT_LABELS[index];
    state.config.hitcounter_colors[index] = HIT_DEFAULT_COLORS[index];
    state.config.hitcounter_item_scales[index] = 1;
    state.config.hitcounter_item_offsets_x[index] = xs[index];
    state.config.hitcounter_item_offsets_y[index] = ys[index];
    state.config.hitcounter_label_offsets_x[index] = 0;
    state.config.hitcounter_label_offsets_y[index] = 0;
    state.config.hitcounter_value_offsets_x[index] = 0;
    state.config.hitcounter_value_offsets_y[index] = 0;
    state.config.hitcounter_dot_offsets_x[index] = 0;
    state.config.hitcounter_dot_offsets_y[index] = 0;
}

function renderAndPersist() {
    applyHitCounterToDocument();
    syncControlsFromState();
    persistConfig();
}

function autoArrangeAndRender(save = true) {
    const { xs, ys } = getDefaultHitPositions(state.config, state.config.hitcounter_orientation);
    state.config.hitcounter_item_offsets_x = xs;
    state.config.hitcounter_item_offsets_y = ys;
    applyHitCounterToDocument();
    syncControlsFromState();
    if (save) persistConfig();
}

function resetSelectedHit() {
    applySelectedDefaults(selectedHitIndex);
    renderAndPersist();
}

function resetLayoutSection() {
    applyLayoutDefaults();
    autoArrangeAndRender(false);
    renderAndPersist();
}

function resetCardStyleSection() {
    applyCardStyleDefaults();
    autoArrangeAndRender(false);
    renderAndPersist();
}

function resetAllHitCounter() {
    applyHitCounterDefaults(state.config);
    selectedHitIndex = 0;
    autoArrangeAndRender(false);
    renderAndPersist();
}

function updateStateFromControl(id) {
    ensureHitCounterConfig();
    const value = document.getElementById(id)?.value;
    switch (id) {
        case 'setup-hit-opacity':
            state.config.hitcounter_opacity = clamp(normalizeNumber(value, ETTERNA_HIT_STYLE.opacity), 0, 1);
            break;
        case 'setup-hit-scale':
            state.config.hitcounter_scale = normalizeScale(value, 1);
            break;
        case 'setup-hit-bg-color':
            state.config.hitcounter_bg_color = value || ETTERNA_HIT_STYLE.bg;
            break;
        case 'setup-hit-text-color':
            state.config.hitcounter_text_color = value || ETTERNA_HIT_STYLE.text;
            break;
        case 'setup-hit-border-style':
            state.config.hitcounter_border_style = value || ETTERNA_HIT_STYLE.borderStyle;
            break;
        case 'setup-hit-border-color':
            state.config.hitcounter_border_color = value || ETTERNA_HIT_STYLE.borderColor;
            break;
        case 'setup-hit-font':
            state.config.hitcounter_font = HIT_FONTS.has(value) ? value : ETTERNA_HIT_STYLE.font;
            break;
        case 'setup-hit-pos-x':
            state.config.hitcounter_position_x = normalizeNumber(value, 0);
            break;
        case 'setup-hit-pos-y':
            state.config.hitcounter_position_y = normalizeNumber(value, 0);
            break;
        case 'setup-hit-padding':
            state.config.hitcounter_padding = clamp(normalizeNumber(value, ETTERNA_HIT_STYLE.padding), 0, 64);
            break;
        case 'setup-hit-gap':
            state.config.hitcounter_gap = clamp(normalizeNumber(value, ETTERNA_HIT_STYLE.gap), 0, 48);
            break;
        case 'setup-hit-item-width':
            state.config.hitcounter_item_width = clamp(normalizeNumber(value, ETTERNA_HIT_STYLE.itemWidth), 72, 220);
            break;
        case 'setup-hit-item-height':
            state.config.hitcounter_item_height = clamp(normalizeNumber(value, ETTERNA_HIT_STYLE.itemHeight), 52, 180);
            break;
        case 'setup-hit-radius':
            state.config.hitcounter_item_radius = clamp(normalizeNumber(value, ETTERNA_HIT_STYLE.itemRadius), 0, 48);
            break;
        case 'setup-hit-dot-size':
            state.config.hitcounter_dot_size = clamp(normalizeNumber(value, ETTERNA_HIT_STYLE.dotSize), 4, 40);
            break;
        case 'setup-hit-label-size':
            state.config.hitcounter_label_size = clamp(normalizeNumber(value, ETTERNA_HIT_STYLE.labelSize), 8, 40);
            break;
        case 'setup-hit-value-size':
            state.config.hitcounter_value_size = clamp(normalizeNumber(value, ETTERNA_HIT_STYLE.valueSize), 10, 56);
            break;
        case 'setup-hit-glow':
            state.config.hitcounter_glow_strength = clamp(normalizeNumber(value, ETTERNA_HIT_STYLE.glow), 0, 1);
            break;
        case 'setup-hit-label-text':
            state.config.hitcounter_labels[selectedHitIndex] = String(value || '').trim() || HIT_DEFAULT_LABELS[selectedHitIndex];
            break;
        case 'setup-hit-item-color':
            state.config.hitcounter_colors[selectedHitIndex] = value || HIT_DEFAULT_COLORS[selectedHitIndex];
            break;
        case 'setup-hit-item-scale':
            state.config.hitcounter_item_scales[selectedHitIndex] = normalizeScale(value, 1);
            break;
        case 'setup-hit-item-x':
            state.config.hitcounter_item_offsets_x[selectedHitIndex] = normalizeNumber(value, 0);
            break;
        case 'setup-hit-item-y':
            state.config.hitcounter_item_offsets_y[selectedHitIndex] = normalizeNumber(value, 0);
            break;
        case 'setup-hit-label-x':
            state.config.hitcounter_label_offsets_x[selectedHitIndex] = normalizeNumber(value, 0);
            break;
        case 'setup-hit-label-y':
            state.config.hitcounter_label_offsets_y[selectedHitIndex] = normalizeNumber(value, 0);
            break;
        case 'setup-hit-value-x':
            state.config.hitcounter_value_offsets_x[selectedHitIndex] = normalizeNumber(value, 0);
            break;
        case 'setup-hit-value-y':
            state.config.hitcounter_value_offsets_y[selectedHitIndex] = normalizeNumber(value, 0);
            break;
        case 'setup-hit-dot-x':
            state.config.hitcounter_dot_offsets_x[selectedHitIndex] = normalizeNumber(value, 0);
            break;
        case 'setup-hit-dot-y':
            state.config.hitcounter_dot_offsets_y[selectedHitIndex] = normalizeNumber(value, 0);
            break;
        case 'setup-hit-orientation':
            state.config.hitcounter_orientation = value || 'vertical';
            autoArrangeAndRender(true);
            return;
        default:
            return;
    }

    applyHitCounterToDocument();
    syncControlsFromState();
    persistConfig();
}

function setDragTooltip(text, x, y) {
    const tooltip = document.getElementById('hit-editor-tooltip');
    if (!tooltip) return;
    tooltip.style.display = text ? 'block' : 'none';
    tooltip.textContent = text;
    if (typeof x === 'number') tooltip.style.left = `${x + 16}px`;
    if (typeof y === 'number') tooltip.style.top = `${y + 16}px`;
}

function startDrag(event) {
    const target = event.target.closest('[data-hit-index], [data-hit-part]');
    if (!target) return;
    event.preventDefault();

    const item = event.target.closest('[data-hit-index]');
    if (!item) return;
    const index = Number(item.dataset.hitIndex);
    if (!Number.isInteger(index)) return;

    selectedHitIndex = index;
    syncSelectedControls();
    applyHitCounterToDocument();

    const part = event.target.dataset.hitPart || 'item';
    let xKey = 'hitcounter_item_offsets_x';
    let yKey = 'hitcounter_item_offsets_y';
    let mode = 'item';

    if (part === 'label') {
        xKey = 'hitcounter_label_offsets_x';
        yKey = 'hitcounter_label_offsets_y';
        mode = 'label';
    } else if (part === 'value') {
        xKey = 'hitcounter_value_offsets_x';
        yKey = 'hitcounter_value_offsets_y';
        mode = 'value';
    } else if (part === 'dot') {
        xKey = 'hitcounter_dot_offsets_x';
        yKey = 'hitcounter_dot_offsets_y';
        mode = 'dot';
    }

    dragState = {
        index,
        mode,
        xKey,
        yKey,
        startX: event.clientX,
        startY: event.clientY,
        originX: state.config[xKey][index],
        originY: state.config[yKey][index],
        dragScale: Math.max(0.0001, previewFitScale * state.config.hitcounter_scale),
    };
    setDragTooltip(`${mode.toUpperCase()} ${index + 1}`, event.clientX, event.clientY);
}

function onDragMove(event) {
    if (!dragState) return;
    const deltaX = (event.clientX - dragState.startX) / dragState.dragScale;
    const deltaY = (event.clientY - dragState.startY) / dragState.dragScale;
    const snap = dragState.mode === 'item' ? ITEM_SNAP : PART_SNAP;
    state.config[dragState.xKey][dragState.index] = snapValue(dragState.originX + deltaX, snap);
    state.config[dragState.yKey][dragState.index] = snapValue(dragState.originY + deltaY, snap);
    applyHitCounterToDocument();
    syncSelectedControls();
    setDragTooltip(
        `${dragState.mode.toUpperCase()} ${dragState.index + 1}: ${state.config[dragState.xKey][dragState.index]}, ${state.config[dragState.yKey][dragState.index]}`,
        event.clientX,
        event.clientY,
    );
}

function onDragEnd() {
    if (!dragState) return;
    dragState = null;
    setDragTooltip('');
    persistConfig();
}

export function initHitCounterEditor() {
    ensureHitCounterConfig();
    scheduleHitCounterLayoutRefresh();
    syncControlsFromState();

    if (initialized) return;
    initialized = true;

    const previewRoot = getPreviewRoot();
    if (previewRoot) {
        previewRoot.addEventListener('mousedown', startDrag);
    }
    document.addEventListener('mousemove', onDragMove);
    document.addEventListener('mouseup', onDragEnd);

    [
        'setup-hit-opacity',
        'setup-hit-scale',
        'setup-hit-bg-color',
        'setup-hit-text-color',
        'setup-hit-border-style',
        'setup-hit-border-color',
        'setup-hit-font',
        'setup-hit-pos-x',
        'setup-hit-pos-y',
        'setup-hit-padding',
        'setup-hit-gap',
        'setup-hit-item-width',
        'setup-hit-item-height',
        'setup-hit-radius',
        'setup-hit-dot-size',
        'setup-hit-label-size',
        'setup-hit-value-size',
        'setup-hit-glow',
        'setup-hit-label-text',
        'setup-hit-item-color',
        'setup-hit-item-scale',
        'setup-hit-item-x',
        'setup-hit-item-y',
        'setup-hit-label-x',
        'setup-hit-label-y',
        'setup-hit-value-x',
        'setup-hit-value-y',
        'setup-hit-dot-x',
        'setup-hit-dot-y',
        'setup-hit-orientation',
    ].forEach((id) => {
        const element = document.getElementById(id);
        if (!element) return;
        const eventName = element.tagName === 'SELECT' ? 'change' : 'input';
        element.addEventListener(eventName, () => updateStateFromControl(id));
    });

    document.getElementById('hit-auto-arrange-btn')?.addEventListener('click', () => autoArrangeAndRender(true));
    document.getElementById('hit-stack-vertical-btn')?.addEventListener('click', () => {
        state.config.hitcounter_orientation = 'vertical';
        const select = document.getElementById('setup-hit-orientation');
        if (select) select.value = 'vertical';
        autoArrangeAndRender(true);
    });
    document.getElementById('hit-stack-horizontal-btn')?.addEventListener('click', () => {
        state.config.hitcounter_orientation = 'horizontal';
        const select = document.getElementById('setup-hit-orientation');
        if (select) select.value = 'horizontal';
        autoArrangeAndRender(true);
    });
    document.getElementById('hit-reset-all-btn')?.addEventListener('click', resetAllHitCounter);
    document.getElementById('hit-reset-layout-btn')?.addEventListener('click', resetLayoutSection);
    document.getElementById('hit-reset-style-btn')?.addEventListener('click', resetCardStyleSection);
    document.getElementById('hit-reset-selected-btn')?.addEventListener('click', resetSelectedHit);
    document.getElementById('hit-reset-selected-inline-btn')?.addEventListener('click', resetSelectedHit);

    document.addEventListener('config-saved', () => {
        ensureHitCounterConfig();
        scheduleHitCounterLayoutRefresh();
        syncControlsFromState();
    });

    window.addEventListener('resize', () => {
        scheduleHitCounterLayoutRefresh();
    });

    document.addEventListener('ori:tab-activated', (event) => {
        if (event?.detail?.tabId === 'hits') {
            scheduleHitCounterLayoutRefresh();
        }
    });
}
