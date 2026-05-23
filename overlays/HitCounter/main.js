import {
    ETTERNA_HIT_STYLE,
    HIT_DEFAULT_COLORS,
    HIT_DEFAULT_LABELS,
    applyHitCounterLayout,
    ensureHitCounterConfigState,
} from './hitcounter-layout.js';

let socket = null;

const config = {
    bg_opacity: ETTERNA_HIT_STYLE.opacity,
    scale: ETTERNA_HIT_STYLE.scale,
    tosu_port: 24050,
    hitcounter_opacity: ETTERNA_HIT_STYLE.opacity,
    hitcounter_scale: ETTERNA_HIT_STYLE.scale,
    hitcounter_bg_color: ETTERNA_HIT_STYLE.bg,
    hitcounter_text_color: ETTERNA_HIT_STYLE.text,
    hitcounter_border_style: ETTERNA_HIT_STYLE.borderStyle,
    hitcounter_border_color: ETTERNA_HIT_STYLE.borderColor,
    hitcounter_font: ETTERNA_HIT_STYLE.font,
    hitcounter_position_x: ETTERNA_HIT_STYLE.positionX,
    hitcounter_position_y: ETTERNA_HIT_STYLE.positionY,
    hitcounter_padding: ETTERNA_HIT_STYLE.padding,
    hitcounter_gap: ETTERNA_HIT_STYLE.gap,
    hitcounter_item_width: ETTERNA_HIT_STYLE.itemWidth,
    hitcounter_item_height: ETTERNA_HIT_STYLE.itemHeight,
    hitcounter_item_radius: ETTERNA_HIT_STYLE.itemRadius,
    hitcounter_label_size: ETTERNA_HIT_STYLE.labelSize,
    hitcounter_value_size: ETTERNA_HIT_STYLE.valueSize,
    hitcounter_dot_size: ETTERNA_HIT_STYLE.dotSize,
    hitcounter_glow_strength: ETTERNA_HIT_STYLE.glow,
    hitcounter_orientation: ETTERNA_HIT_STYLE.orientation,
    hitcounter_labels: [...HIT_DEFAULT_LABELS],
    hitcounter_colors: [...HIT_DEFAULT_COLORS],
};

function syncLegacyAliases() {
    if (config.hitcounter_opacity === undefined && config.bg_opacity !== undefined) {
        config.hitcounter_opacity = config.bg_opacity;
    }
    if (config.hitcounter_scale === undefined && config.scale !== undefined) {
        config.hitcounter_scale = config.scale;
    }

    config.bg_opacity = config.hitcounter_opacity;
    config.scale = config.hitcounter_scale;
}

function applyLayout() {
    const root = document.getElementById('hit-counter');
    if (!root) return;

    syncLegacyAliases();
    ensureHitCounterConfigState(config);
    applyHitCounterLayout(root, config, { preview: false });
}

function applyConfig(data) {
    Object.assign(config, data);
    if (data.hitcounter_opacity !== undefined) config.bg_opacity = data.hitcounter_opacity;
    if (data.hitcounter_scale !== undefined) config.scale = data.hitcounter_scale;
    if (data.tosu_port !== undefined) config.tosu_port = data.tosu_port;
    applyLayout();
}

async function loadConfig() {
    try {
        const res = await fetch('../msdconverter/config.json');
        if (res.ok) {
            const data = await res.json();
            applyConfig(data);
        }
    } catch (err) {
        console.error('Error loading config:', err);
    }
}

const lastHits = { geki: -1, '300': -1, katu: -1, '100': -1, '50': -1, '0': -1 };

function connect() {
    socket = new WebSocket(`ws://127.0.0.1:${config.tosu_port}/ws`);

    socket.onmessage = (e) => {
        try {
            const data = JSON.parse(e.data);
            const hits = data.gameplay?.hits || {};

            const nextValues = {
                geki: hits.geki || 0,
                '300': hits['300'] || 0,
                katu: hits.katu || 0,
                '100': hits['100'] || 0,
                '50': hits['50'] || 0,
                '0': hits['0'] || 0,
            };

            if (nextValues.geki !== lastHits.geki) {
                document.getElementById('val-max').textContent = nextValues.geki;
                lastHits.geki = nextValues.geki;
            }
            if (nextValues['300'] !== lastHits['300']) {
                document.getElementById('val-perf').textContent = nextValues['300'];
                lastHits['300'] = nextValues['300'];
            }
            if (nextValues.katu !== lastHits.katu) {
                document.getElementById('val-great').textContent = nextValues.katu;
                lastHits.katu = nextValues.katu;
            }
            if (nextValues['100'] !== lastHits['100']) {
                document.getElementById('val-good').textContent = nextValues['100'];
                lastHits['100'] = nextValues['100'];
            }
            if (nextValues['50'] !== lastHits['50']) {
                document.getElementById('val-bad').textContent = nextValues['50'];
                lastHits['50'] = nextValues['50'];
            }
            if (nextValues['0'] !== lastHits['0']) {
                document.getElementById('val-miss').textContent = nextValues['0'];
                lastHits['0'] = nextValues['0'];
            }
        } catch (err) {
            console.error(err);
        }
    };

    socket.onclose = () => {
        setTimeout(connect, 5000);
    };
}

function connectConfig() {
    const ws = new WebSocket('ws://127.0.0.1:24051');

    ws.onmessage = (e) => {
        try {
            const data = JSON.parse(e.data);
            const scopes = Array.isArray(data.changed_scopes) ? data.changed_scopes : [];
            const isRelevant = data.event === 'config-updated'
                && (scopes.length === 0 || scopes.includes('HitCounter') || scopes.includes('all'));
            if (isRelevant) {
                applyConfig(data);
            }
        } catch (err) {
            console.error(err);
        }
    };

    ws.onclose = () => {
        setTimeout(connectConfig, 5000);
    };
}

loadConfig().then(() => {
    applyLayout();
    connect();
    connectConfig();
});
