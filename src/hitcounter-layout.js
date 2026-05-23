export const HIT_KEYS = ['max', 'perf', 'great', 'good', 'bad', 'miss'];
export const HIT_DEFAULT_LABELS = ['MAX', 'PERF', 'GREAT', 'GOOD', 'BAD', 'MISS'];
export const HIT_DEFAULT_COLORS = ['#ffffff', '#fbc531', '#4cd137', '#00a8ff', '#e84118', '#7f8c8d'];
export const HIT_LEGACY_COLORS = ['#ffffff', '#ffcf6c', '#8ee0a0', '#94d5ff', '#ff8b8b', '#ac97a4'];
export const HIT_PREVIEW_VALUES = ['320', '300', '200', '100', '50', '0'];
export const HIT_FONTS = new Set(['Inter', 'Outfit', 'Nunito', 'Quicksand', 'Urbanist', 'Baloo 2']);
export const ITEM_SNAP = 8;
export const PART_SNAP = 4;

export const LEGACY_HIT_STYLE = {
    bg: '#2b1730',
    text: '#ffffff',
    borderStyle: 'none',
    borderColor: '#e6bfd4',
    font: 'Nunito',
    padding: 16,
    gap: 10,
    itemWidth: 118,
    itemHeight: 72,
    itemRadius: 16,
    dotSize: 8,
    glow: 0.35,
};

export const CLEAN_HIT_STYLE = {
    bg: '#10141f',
    text: '#b7bfd4',
    borderStyle: 'solid',
    borderColor: '#2b3347',
    font: 'Urbanist',
    padding: 12,
    gap: 8,
    itemWidth: 116,
    itemHeight: 70,
    itemRadius: 12,
    dotSize: 6,
    glow: 0.18,
};

export const ETTERNA_HIT_STYLE = {
    opacity: 0.82,
    scale: 1.0,
    bg: '#0f1320',
    text: '#8b8ba8',
    borderStyle: 'solid',
    borderColor: '#262f47',
    font: 'Inter',
    orientation: 'vertical',
    positionX: 0,
    positionY: 0,
    padding: 10,
    gap: 8,
    itemWidth: 118,
    itemHeight: 62,
    itemRadius: 10,
    labelSize: 10,
    valueSize: 22,
    dotSize: 6,
    glow: 0.12,
};

export function clamp(value, min, max) {
    return Math.min(max, Math.max(min, value));
}

export function normalizeNumber(value, fallback = 0) {
    const n = Number(value);
    return Number.isFinite(n) ? n : fallback;
}

export function normalizeScale(value, fallback = 1) {
    const n = Number(value);
    return Number.isFinite(n) && n > 0 ? n : fallback;
}

export function snapValue(value, step = ITEM_SNAP) {
    return Math.round(value / step) * step;
}

export function arraysEqual(left, right) {
    return Array.isArray(left)
        && Array.isArray(right)
        && left.length === right.length
        && left.every((value, index) => value === right[index]);
}

function ensureArray(config, key, length, fallback) {
    const current = Array.isArray(config[key]) ? [...config[key]] : [];
    while (current.length < length) {
        current.push(typeof fallback === 'function' ? fallback(current.length) : fallback);
    }
    if (current.length > length) current.length = length;
    config[key] = current;
    return current;
}

export function normalizeHexColor(color, fallback) {
    const hexMatch = String(color || '').match(/^#([0-9a-f]{6})$/i);
    return hexMatch ? hexMatch[0] : fallback;
}

export function hexToRgba(hex, alpha = 1) {
    const sanitized = String(hex || '').replace('#', '');
    if (sanitized.length !== 6) return `rgba(255,255,255,${alpha})`;
    const r = parseInt(sanitized.slice(0, 2), 16);
    const g = parseInt(sanitized.slice(2, 4), 16);
    const b = parseInt(sanitized.slice(4, 6), 16);
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

export function getDefaultHitPositions(config, orientation = config.hitcounter_orientation) {
    const width = normalizeNumber(config.hitcounter_item_width, ETTERNA_HIT_STYLE.itemWidth);
    const height = normalizeNumber(config.hitcounter_item_height, ETTERNA_HIT_STYLE.itemHeight);
    const gap = normalizeNumber(config.hitcounter_gap, ETTERNA_HIT_STYLE.gap);
    const xs = [];
    const ys = [];

    for (let index = 0; index < HIT_KEYS.length; index++) {
        if (orientation === 'horizontal') {
            xs.push(index * (width + gap));
            ys.push(0);
        } else {
            xs.push(0);
            ys.push(index * (height + gap));
        }
    }

    return { xs, ys };
}

export function migrateLegacyHitCounterStyle(config) {
    const matchesLegacyStyle =
        config.hitcounter_bg_color === LEGACY_HIT_STYLE.bg
        && config.hitcounter_text_color === LEGACY_HIT_STYLE.text
        && config.hitcounter_border_style === LEGACY_HIT_STYLE.borderStyle
        && config.hitcounter_border_color === LEGACY_HIT_STYLE.borderColor
        && config.hitcounter_font === LEGACY_HIT_STYLE.font
        && normalizeNumber(config.hitcounter_padding, LEGACY_HIT_STYLE.padding) === LEGACY_HIT_STYLE.padding
        && normalizeNumber(config.hitcounter_gap, LEGACY_HIT_STYLE.gap) === LEGACY_HIT_STYLE.gap
        && normalizeNumber(config.hitcounter_item_width, LEGACY_HIT_STYLE.itemWidth) === LEGACY_HIT_STYLE.itemWidth
        && normalizeNumber(config.hitcounter_item_height, LEGACY_HIT_STYLE.itemHeight) === LEGACY_HIT_STYLE.itemHeight
        && normalizeNumber(config.hitcounter_item_radius, LEGACY_HIT_STYLE.itemRadius) === LEGACY_HIT_STYLE.itemRadius
        && normalizeNumber(config.hitcounter_dot_size, LEGACY_HIT_STYLE.dotSize) === LEGACY_HIT_STYLE.dotSize
        && Math.abs(normalizeNumber(config.hitcounter_glow_strength, LEGACY_HIT_STYLE.glow) - LEGACY_HIT_STYLE.glow) < 0.0001;
    const matchesCleanStyle =
        config.hitcounter_bg_color === CLEAN_HIT_STYLE.bg
        && config.hitcounter_text_color === CLEAN_HIT_STYLE.text
        && config.hitcounter_border_style === CLEAN_HIT_STYLE.borderStyle
        && config.hitcounter_border_color === CLEAN_HIT_STYLE.borderColor
        && config.hitcounter_font === CLEAN_HIT_STYLE.font
        && normalizeNumber(config.hitcounter_padding, CLEAN_HIT_STYLE.padding) === CLEAN_HIT_STYLE.padding
        && normalizeNumber(config.hitcounter_gap, CLEAN_HIT_STYLE.gap) === CLEAN_HIT_STYLE.gap
        && normalizeNumber(config.hitcounter_item_width, CLEAN_HIT_STYLE.itemWidth) === CLEAN_HIT_STYLE.itemWidth
        && normalizeNumber(config.hitcounter_item_height, CLEAN_HIT_STYLE.itemHeight) === CLEAN_HIT_STYLE.itemHeight
        && normalizeNumber(config.hitcounter_item_radius, CLEAN_HIT_STYLE.itemRadius) === CLEAN_HIT_STYLE.itemRadius
        && normalizeNumber(config.hitcounter_dot_size, CLEAN_HIT_STYLE.dotSize) === CLEAN_HIT_STYLE.dotSize
        && Math.abs(normalizeNumber(config.hitcounter_glow_strength, CLEAN_HIT_STYLE.glow) - CLEAN_HIT_STYLE.glow) < 0.0001;

    if (!matchesLegacyStyle && !matchesCleanStyle) return;

    const legacyXs = [0, 128, 0, 128, 0, 128];
    const legacyYs = [0, 0, 82, 82, 164, 164];
    const hadLegacyPositions = arraysEqual(config.hitcounter_item_offsets_x, legacyXs)
        && arraysEqual(config.hitcounter_item_offsets_y, legacyYs);
    const cleanXs = [0, 0, 0, 0, 0, 0];
    const cleanYs = [0, 78, 156, 234, 312, 390];
    const hadCleanPositions = arraysEqual(config.hitcounter_item_offsets_x, cleanXs)
        && arraysEqual(config.hitcounter_item_offsets_y, cleanYs);

    config.hitcounter_opacity = ETTERNA_HIT_STYLE.opacity;
    config.hitcounter_scale = ETTERNA_HIT_STYLE.scale;
    config.hitcounter_bg_color = ETTERNA_HIT_STYLE.bg;
    config.hitcounter_text_color = ETTERNA_HIT_STYLE.text;
    config.hitcounter_border_style = ETTERNA_HIT_STYLE.borderStyle;
    config.hitcounter_border_color = ETTERNA_HIT_STYLE.borderColor;
    config.hitcounter_font = ETTERNA_HIT_STYLE.font;
    config.hitcounter_padding = ETTERNA_HIT_STYLE.padding;
    config.hitcounter_gap = ETTERNA_HIT_STYLE.gap;
    config.hitcounter_item_width = ETTERNA_HIT_STYLE.itemWidth;
    config.hitcounter_item_height = ETTERNA_HIT_STYLE.itemHeight;
    config.hitcounter_item_radius = ETTERNA_HIT_STYLE.itemRadius;
    config.hitcounter_label_size = ETTERNA_HIT_STYLE.labelSize;
    config.hitcounter_value_size = ETTERNA_HIT_STYLE.valueSize;
    config.hitcounter_dot_size = ETTERNA_HIT_STYLE.dotSize;
    config.hitcounter_glow_strength = ETTERNA_HIT_STYLE.glow;
    config.hitcounter_position_x = ETTERNA_HIT_STYLE.positionX;
    config.hitcounter_position_y = ETTERNA_HIT_STYLE.positionY;
    config.hitcounter_orientation = ETTERNA_HIT_STYLE.orientation;

    if (arraysEqual(config.hitcounter_colors, HIT_LEGACY_COLORS)) {
        config.hitcounter_colors = [...HIT_DEFAULT_COLORS];
    }

    if (hadLegacyPositions || hadCleanPositions) {
        const { xs, ys } = getDefaultHitPositions(config, config.hitcounter_orientation || 'vertical');
        config.hitcounter_item_offsets_x = xs;
        config.hitcounter_item_offsets_y = ys;
    }
}

export function ensureHitCounterConfigState(config) {
    migrateLegacyHitCounterStyle(config);
    config.hitcounter_opacity = clamp(normalizeNumber(config.hitcounter_opacity, ETTERNA_HIT_STYLE.opacity), 0, 1);
    config.hitcounter_scale = normalizeScale(config.hitcounter_scale, ETTERNA_HIT_STYLE.scale);
    config.hitcounter_bg_color = config.hitcounter_bg_color || ETTERNA_HIT_STYLE.bg;
    config.hitcounter_text_color = config.hitcounter_text_color || ETTERNA_HIT_STYLE.text;
    config.hitcounter_border_color = config.hitcounter_border_color || ETTERNA_HIT_STYLE.borderColor;
    config.hitcounter_border_style = config.hitcounter_border_style || ETTERNA_HIT_STYLE.borderStyle;
    config.hitcounter_orientation = config.hitcounter_orientation || ETTERNA_HIT_STYLE.orientation;
    config.hitcounter_font = HIT_FONTS.has(config.hitcounter_font) ? config.hitcounter_font : ETTERNA_HIT_STYLE.font;
    config.hitcounter_position_x = normalizeNumber(config.hitcounter_position_x, 0);
    config.hitcounter_position_y = normalizeNumber(config.hitcounter_position_y, 0);
    config.hitcounter_padding = clamp(normalizeNumber(config.hitcounter_padding, ETTERNA_HIT_STYLE.padding), 0, 64);
    config.hitcounter_gap = clamp(normalizeNumber(config.hitcounter_gap, ETTERNA_HIT_STYLE.gap), 0, 48);
    config.hitcounter_item_width = clamp(normalizeNumber(config.hitcounter_item_width, ETTERNA_HIT_STYLE.itemWidth), 72, 220);
    config.hitcounter_item_height = clamp(normalizeNumber(config.hitcounter_item_height, ETTERNA_HIT_STYLE.itemHeight), 52, 180);
    config.hitcounter_item_radius = clamp(normalizeNumber(config.hitcounter_item_radius, ETTERNA_HIT_STYLE.itemRadius), 0, 48);
    config.hitcounter_label_size = clamp(normalizeNumber(config.hitcounter_label_size, ETTERNA_HIT_STYLE.labelSize), 8, 40);
    config.hitcounter_value_size = clamp(normalizeNumber(config.hitcounter_value_size, ETTERNA_HIT_STYLE.valueSize), 10, 56);
    config.hitcounter_dot_size = clamp(normalizeNumber(config.hitcounter_dot_size, ETTERNA_HIT_STYLE.dotSize), 4, 40);
    config.hitcounter_glow_strength = clamp(normalizeNumber(config.hitcounter_glow_strength, ETTERNA_HIT_STYLE.glow), 0, 1);

    const labels = ensureArray(config, 'hitcounter_labels', HIT_KEYS.length, idx => HIT_DEFAULT_LABELS[idx]);
    config.hitcounter_labels = labels.map((label, idx) => String(label || '').trim() || HIT_DEFAULT_LABELS[idx]);

    const colors = ensureArray(config, 'hitcounter_colors', HIT_KEYS.length, idx => HIT_DEFAULT_COLORS[idx]);
    config.hitcounter_colors = colors.map((color, idx) => String(color || '').trim() || HIT_DEFAULT_COLORS[idx]);

    const defaults = getDefaultHitPositions(config, config.hitcounter_orientation);
    ensureArray(config, 'hitcounter_item_offsets_x', HIT_KEYS.length, idx => defaults.xs[idx]);
    ensureArray(config, 'hitcounter_item_offsets_y', HIT_KEYS.length, idx => defaults.ys[idx]);

    const itemScales = ensureArray(config, 'hitcounter_item_scales', HIT_KEYS.length, 1);
    config.hitcounter_item_scales = itemScales.map(value => normalizeScale(value, 1));

    [
        'hitcounter_label_offsets_x',
        'hitcounter_label_offsets_y',
        'hitcounter_value_offsets_x',
        'hitcounter_value_offsets_y',
        'hitcounter_dot_offsets_x',
        'hitcounter_dot_offsets_y',
    ].forEach((key) => {
        const values = ensureArray(config, key, HIT_KEYS.length, 0);
        config[key] = values.map(value => normalizeNumber(value, 0));
    });

    return config;
}

export function applyHitCounterLayout(
    root,
    config,
    {
        preview = false,
        selectedIndex = -1,
        workspace = null,
        previewValues = HIT_PREVIEW_VALUES,
    } = {},
) {
    if (!root || !config) {
        return { layoutWidth: 0, layoutHeight: 0, previewFitScale: 1 };
    }

    ensureHitCounterConfigState(config);

    const padding = config.hitcounter_padding;
    const itemWidth = config.hitcounter_item_width;
    const itemHeight = config.hitcounter_item_height;
    const radius = config.hitcounter_item_radius;
    const dotSize = config.hitcounter_dot_size;
    const labelSize = config.hitcounter_label_size;
    const valueSize = config.hitcounter_value_size;
    const glowStrength = config.hitcounter_glow_strength;
    const itemXs = config.hitcounter_item_offsets_x;
    const itemYs = config.hitcounter_item_offsets_y;
    const itemScales = config.hitcounter_item_scales;
    const labelXs = config.hitcounter_label_offsets_x;
    const labelYs = config.hitcounter_label_offsets_y;
    const valueXs = config.hitcounter_value_offsets_x;
    const valueYs = config.hitcounter_value_offsets_y;
    const dotXs = config.hitcounter_dot_offsets_x;
    const dotYs = config.hitcounter_dot_offsets_y;

    root.classList.add('hit-layout-root', 'hit-counter');
    if (preview) {
        root.style.setProperty('--hit-grid-size', `${ITEM_SNAP}px`);
    } else {
        root.style.removeProperty('--hit-grid-size');
    }
    root.style.position = preview ? 'absolute' : 'relative';
    root.style.display = 'block';
    root.style.minWidth = '0px';
    root.style.minHeight = '0px';
    root.style.padding = `${padding}px`;
    root.style.background = hexToRgba(config.hitcounter_bg_color, config.hitcounter_opacity);
    root.style.borderStyle = config.hitcounter_border_style;
    root.style.borderColor = config.hitcounter_border_style === 'none' ? 'transparent' : config.hitcounter_border_color;
    root.style.borderWidth = config.hitcounter_border_style === 'none' ? '0px' : '1px';
    root.style.borderRadius = `${radius + 6}px`;
    root.style.left = preview ? '0px' : '';
    root.style.top = preview ? '0px' : '';
    root.style.transformOrigin = 'top left';
    root.style.backdropFilter = 'blur(12px)';
    root.style.webkitBackdropFilter = 'blur(12px)';
    root.style.boxShadow = `0 14px 34px rgba(4, 8, 18, ${0.3 + glowStrength * 0.18}), inset 0 1px 0 rgba(255,255,255,0.04), inset 0 0 0 1px rgba(18, 25, 43, 0.42)`;

    let maxX = itemWidth;
    let maxY = itemHeight;
    const items = Array.from(root.querySelectorAll('[data-hit-index]'));

    items.forEach((item, index) => {
        const itemScale = normalizeScale(itemScales[index], 1);
        const itemX = padding + normalizeNumber(itemXs[index], 0);
        const itemY = padding + normalizeNumber(itemYs[index], 0);
        const accent = config.hitcounter_colors[index] || HIT_DEFAULT_COLORS[index];
        const label = config.hitcounter_labels[index] || HIT_DEFAULT_LABELS[index];

        item.style.position = 'absolute';
        item.style.left = `${itemX}px`;
        item.style.top = `${itemY}px`;
        item.style.width = `${itemWidth}px`;
        item.style.height = `${itemHeight}px`;
        item.style.padding = '0';
        item.style.margin = '0';
        item.style.display = 'block';
        item.style.borderRadius = `${radius}px`;
        item.style.background = preview
            ? 'linear-gradient(180deg, rgba(255,255,255,0.045), rgba(255,255,255,0.018))'
            : 'linear-gradient(180deg, rgba(255,255,255,0.035), rgba(255,255,255,0.012))';
        item.style.border = `1px solid rgba(255,255,255,${preview ? '0.06' : '0.045'})`;
        item.style.boxShadow = preview
            ? '0 10px 22px rgba(2, 6, 14, 0.22), inset 0 1px 0 rgba(255,255,255,0.04), inset 0 0 0 1px rgba(19, 28, 46, 0.46)'
            : '0 8px 18px rgba(2, 6, 14, 0.24), inset 0 1px 0 rgba(255,255,255,0.025), inset 0 0 0 1px rgba(19, 28, 46, 0.34)';
        item.style.transform = `scale(${itemScale})`;
        item.style.transformOrigin = 'top left';
        item.classList.toggle('selected-hit-item', preview && index === selectedIndex);

        const header = item.querySelector('.j-header');
        if (header) {
            header.style.position = 'absolute';
            header.style.inset = '0';
            header.style.display = 'block';
            header.style.pointerEvents = 'none';
        }

        const dot = item.querySelector('.j-dot');
        const labelEl = item.querySelector('.j-label');
        const valueEl = item.querySelector('.j-val');
        const previewValue = preview ? previewValues[index] : valueEl?.textContent || '0';

        if (dot) {
            dot.style.position = 'absolute';
            dot.style.left = `${14 + normalizeNumber(dotXs[index], 0)}px`;
            dot.style.top = `${14 + normalizeNumber(dotYs[index], 0)}px`;
            dot.style.width = `${dotSize}px`;
            dot.style.height = `${dotSize}px`;
            dot.style.borderRadius = '999px';
            dot.style.background = accent;
            dot.style.boxShadow = `0 0 ${Math.round(6 + glowStrength * 12)}px ${hexToRgba(accent, 0.55)}`;
            dot.style.pointerEvents = preview ? 'auto' : 'none';
        }

        if (labelEl) {
            labelEl.textContent = label;
            labelEl.style.position = 'absolute';
            labelEl.style.left = `${28 + normalizeNumber(labelXs[index], 0)}px`;
            labelEl.style.top = `${8 + normalizeNumber(labelYs[index], 0)}px`;
            labelEl.style.fontSize = `${labelSize}px`;
            labelEl.style.color = config.hitcounter_text_color;
            labelEl.style.fontFamily = `"${config.hitcounter_font}", sans-serif`;
            labelEl.style.fontWeight = '700';
            labelEl.style.letterSpacing = '0.08em';
            labelEl.style.textTransform = 'uppercase';
            labelEl.style.opacity = '0.94';
            labelEl.style.pointerEvents = preview ? 'auto' : 'none';
        }

        if (valueEl) {
            if (preview) valueEl.textContent = previewValue;
            valueEl.style.position = 'absolute';
            valueEl.style.left = `${(itemWidth / 2) + normalizeNumber(valueXs[index], 0)}px`;
            valueEl.style.top = `${(itemHeight / 2) + normalizeNumber(valueYs[index], 0)}px`;
            valueEl.style.transform = 'translate(-50%, -10%)';
            valueEl.style.fontSize = `${valueSize}px`;
            valueEl.style.fontFamily = `"${config.hitcounter_font}", sans-serif`;
            valueEl.style.fontWeight = '800';
            valueEl.style.color = accent;
            valueEl.style.textShadow = `0 0 ${Math.round(8 + glowStrength * 14)}px ${hexToRgba(accent, 0.32)}`;
            valueEl.style.pointerEvents = preview ? 'auto' : 'none';
        }

        maxX = Math.max(maxX, itemX + itemWidth * itemScale);
        maxY = Math.max(maxY, itemY + itemHeight * itemScale);
    });

    const layoutWidth = Math.ceil(maxX + padding);
    const layoutHeight = Math.ceil(maxY + padding);
    root.style.width = `${layoutWidth}px`;
    root.style.height = `${layoutHeight}px`;

    let previewFitScale = 1;
    if (preview) {
        const scaledWidth = layoutWidth * config.hitcounter_scale;
        const scaledHeight = layoutHeight * config.hitcounter_scale;
        const boundsWidth = scaledWidth + Math.max(0, config.hitcounter_position_x);
        const boundsHeight = scaledHeight + Math.max(0, config.hitcounter_position_y);
        const availableWidth = Math.max(120, (workspace?.clientWidth || layoutWidth) - 28);
        const availableHeight = Math.max(120, (workspace?.clientHeight || layoutHeight) - 28);
        previewFitScale = Math.min(1, availableWidth / Math.max(1, boundsWidth), availableHeight / Math.max(1, boundsHeight));
        const contentWidth = boundsWidth * previewFitScale;
        const contentHeight = boundsHeight * previewFitScale;
        const offsetX = Math.max(0, Math.floor((availableWidth - contentWidth) / 2));
        const offsetY = Math.max(0, Math.floor((availableHeight - contentHeight) / 2));
        root.style.left = `${offsetX}px`;
        root.style.top = `${offsetY}px`;
        root.style.transform = `translate(${config.hitcounter_position_x * previewFitScale}px, ${config.hitcounter_position_y * previewFitScale}px) scale(${previewFitScale * config.hitcounter_scale})`;
    } else {
        root.style.transform = `translate(${config.hitcounter_position_x}px, ${config.hitcounter_position_y}px) scale(${config.hitcounter_scale})`;
    }

    return { layoutWidth, layoutHeight, previewFitScale };
}
