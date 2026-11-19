    export const HIT_KEYS = ['max', 'perf', 'great', 'good', 'bad', 'miss'];
export const HIT_DEFAULT_LABELS = ['max', '300', '200', '100', '50', 'miss'];
export const HIT_DEFAULT_COLORS = ['#ededed', '#e5b567', '#4cc38a', '#6cb6ff', '#d291e4', '#e5484d'];

export const DEFAULT_HIT_STYLE = {
    style: 'radar-twin',
    showRatio: true,
    opacity: 0.92,
    scale: 1.0,
    bg: '#141415',
    itemBg: '#19191b',
    text: '#a1a1a6',
    borderStyle: 'solid',
    borderColor: '#28282b',
    itemBorderColor: '#28282b',
    borderWidth: 1.0,
    showAccentBar: false,
    showItemBorder: false,
    autoBgTheme: true,
    font: 'Inter',
    orientation: 'vertical',
    padding: 10,
    gap: 2,
    itemWidth: 140,
    itemHeight: 20,
    itemRadius: 8,
    labelSize: 11.5,
    valueSize: 12.0,
    dotSize: 0,
    glow: 0,
};

export function hexToRgba(hex, alpha = 1) {
    const sanitized = String(hex || '').replace('#', '');
    if (sanitized.length !== 6) return `rgba(20, 20, 21, ${alpha})`;
    const r = parseInt(sanitized.slice(0, 2), 16);
    const g = parseInt(sanitized.slice(2, 4), 16);
    const b = parseInt(sanitized.slice(4, 6), 16);
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

export function ensureHitCounterConfigState(config) {
    if (!Array.isArray(config.hitcounter_labels) || config.hitcounter_labels.length < 6) {
        config.hitcounter_labels = [...HIT_DEFAULT_LABELS];
    }
    if (!Array.isArray(config.hitcounter_colors) || config.hitcounter_colors.length < 6) {
        config.hitcounter_colors = [...HIT_DEFAULT_COLORS];
    }
}

function normalizeStyle(raw) {
    if (['radar-twin', 'grid-2x3', 'dock', 'dock-ratio', 'hud'].includes(raw)) return raw;
    if (raw === 'telemetry' || raw === 'cards') return 'radar-twin';
    return 'radar-twin';
}

export function applyHitCounterLayout(root, config) {
    if (!root || !config) return;

    if (config.show_hits_overlay === false) {
        root.classList.add('hidden');
        return;
    }
    root.classList.remove('hidden');

    ensureHitCounterConfigState(config);

    const style = normalizeStyle(config.hitcounter_style);
    const isHud = style === 'hud';
    const isDock = style === 'dock' || style === 'dock-ratio';
    const isGrid = style === 'grid-2x3';
    const isHorizontal = isDock || (isHud && config.hitcounter_orientation === 'horizontal');

    const padding = isHud ? 4 : isDock ? 0 : Number(config.hitcounter_padding ?? DEFAULT_HIT_STYLE.padding);
    const gap = isDock ? 0 : Number(config.hitcounter_gap ?? DEFAULT_HIT_STYLE.gap);
    const itemWidth = Number(config.hitcounter_item_width ?? (isHorizontal ? 48 : DEFAULT_HIT_STYLE.itemWidth));
    const itemHeight = Number(config.hitcounter_item_height ?? (isHorizontal ? 42 : DEFAULT_HIT_STYLE.itemHeight));
    const radius = Number(config.hitcounter_item_radius ?? DEFAULT_HIT_STYLE.itemRadius);
    const labelSize = Number(config.hitcounter_label_size ?? DEFAULT_HIT_STYLE.labelSize);
    const valueSize = Number(config.hitcounter_value_size ?? DEFAULT_HIT_STYLE.valueSize);
    const dotSize = Number(config.hitcounter_dot_size ?? 0);
    const scale = Number(config.hitcounter_scale || 1.0);

    const adp = config.hitcounter_auto_bg_theme && config._adaptive?.accent ? config._adaptive : null;
    const bgColor = adp?.bg || config.hitcounter_bg_color || '#141415';
    const frameBorderCol = adp?.border || config.hitcounter_border_color || 'rgba(237, 237, 237, 0.08)';
    const accentColor = adp?.accent || config.accent_color || '#e588a6';
    const borderW = Math.max(0, Number(config.hitcounter_border_width ?? DEFAULT_HIT_STYLE.borderWidth));

    root.dataset.style = style;
    root.style.setProperty('--card-border', frameBorderCol);
    root.style.setProperty('--accent-color', accentColor);
    root.style.setProperty('--text-color', config.hitcounter_text_color || DEFAULT_HIT_STYLE.text);
    root.style.padding = `${padding}px`;
    root.style.background = isHud ? 'transparent' : hexToRgba(bgColor, config.hitcounter_opacity ?? 0.92);
    root.style.borderStyle = isHud ? 'none' : (config.hitcounter_border_style || 'solid');
    root.style.borderColor = (isHud || config.hitcounter_border_style === 'none' || borderW <= 0.05) ? 'transparent' : frameBorderCol;
    root.style.borderWidth = (isHud || config.hitcounter_border_style === 'none' || borderW <= 0.05) ? '0px' : `${borderW}px`;
    root.style.borderRadius = `${Math.max(4, Math.min(12, radius))}px`;
    root.style.transform = `scale(${scale})`;

    const itemsWrap = root.querySelector('#hit-items') || root;
    if (!isGrid && !isDock) {
        itemsWrap.style.display = 'flex';
        itemsWrap.style.flexDirection = isHorizontal ? 'row' : 'column';
        itemsWrap.style.gap = `${gap}px`;
    } else {
        itemsWrap.style.display = '';
        itemsWrap.style.flexDirection = '';
        itemsWrap.style.gap = '';
    }

    const ratioRow = root.querySelector('#hit-ratio-row');
    if (ratioRow) {
        const showTopRatio = !isDock && !isHorizontal && config.hitcounter_show_ratio !== false;
        ratioRow.classList.toggle('hidden', !showTopRatio);
        ratioRow.style.borderBottomColor = frameBorderCol;
    }

    const ratioCell = root.querySelector('#hit-ratio-cell');
    if (ratioCell) {
        const showDockRatio = style === 'dock-ratio';
        ratioCell.classList.toggle('hidden', !showDockRatio);
        ratioCell.style.borderLeftColor = frameBorderCol;
    }

    const items = Array.from(root.querySelectorAll('[data-hit-index]'));
    items.forEach((item, index) => {
        const accent = config.hitcounter_colors[index] || HIT_DEFAULT_COLORS[index];
        const label = config.hitcounter_labels[index] || HIT_DEFAULT_LABELS[index];

        item.classList.toggle('horizontal-item', isHorizontal);
        if (!isDock && !isGrid) {
            item.style.width = `${Math.max(96, itemWidth)}px`;
            item.style.height = `${itemHeight}px`;
        } else {
            item.style.width = '';
            item.style.height = isDock ? `${Math.max(36, itemHeight)}px` : '';
        }

        if (isDock) {
            item.style.borderLeftColor = index > 0 ? frameBorderCol : '';
            item.style.flexDirection = 'column';
            item.style.alignItems = 'center';
            item.style.justifyContent = 'center';
        } else {
            item.style.flexDirection = isHorizontal ? 'column' : 'row';
            item.style.alignItems = isHorizontal ? 'center' : 'baseline';
            item.style.justifyContent = isHorizontal ? 'center' : 'space-between';
        }

        const labelEl = item.querySelector('.j-label');
        const valueEl = item.querySelector('.j-val');

        if (labelEl) {
            labelEl.textContent = '';
            if (dotSize > 0 && !isHorizontal && !isDock) {
                const dot = document.createElement('span');
                dot.className = 'j-dot';
                dot.style.width = `${dotSize}px`;
                dot.style.height = `${dotSize}px`;
                dot.style.background = accent;
                labelEl.appendChild(dot);
            }
            labelEl.appendChild(document.createTextNode(label));
            if (!isDock) {
                labelEl.style.fontSize = `${labelSize}px`;
            }
            labelEl.style.color = config.hitcounter_text_color || DEFAULT_HIT_STYLE.text;
        }
        if (valueEl) {
            valueEl.style.fontSize = `${valueSize}px`;
            valueEl.style.color = accent;
        }
    });
}
