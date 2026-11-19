import {
    DEFAULT_HIT_STYLE,
    HIT_DEFAULT_COLORS,
    HIT_DEFAULT_LABELS,
    applyHitCounterLayout,
    ensureHitCounterConfigState,
} from './hitcounter-layout.js';

let socket = null;
let configSocket = null;
let activeTosuAddr = '';
let activeCompanionPort = 0;
const urlParams = new URLSearchParams(window.location.search);

const config = {
    show_hits_overlay: true,
    tosu_host: '127.0.0.1',
    tosu_port: 24050,
    companion_port: 24051,
    active_companion_port: 24051,
    accent_color: '#e588a6',
    hitcounter_style: DEFAULT_HIT_STYLE.style,
    hitcounter_show_ratio: DEFAULT_HIT_STYLE.showRatio,
    hitcounter_opacity: DEFAULT_HIT_STYLE.opacity,
    hitcounter_scale: DEFAULT_HIT_STYLE.scale,
    hitcounter_bg_color: DEFAULT_HIT_STYLE.bg,
    hitcounter_item_bg_color: DEFAULT_HIT_STYLE.itemBg,
    hitcounter_text_color: DEFAULT_HIT_STYLE.text,
    hitcounter_border_style: DEFAULT_HIT_STYLE.borderStyle,
    hitcounter_border_color: DEFAULT_HIT_STYLE.borderColor,
    hitcounter_border_width: DEFAULT_HIT_STYLE.borderWidth,
    hitcounter_auto_bg_theme: DEFAULT_HIT_STYLE.autoBgTheme,
    hitcounter_font: DEFAULT_HIT_STYLE.font,
    hitcounter_padding: DEFAULT_HIT_STYLE.padding,
    hitcounter_gap: DEFAULT_HIT_STYLE.gap,
    hitcounter_item_width: DEFAULT_HIT_STYLE.itemWidth,
    hitcounter_item_height: DEFAULT_HIT_STYLE.itemHeight,
    hitcounter_item_radius: DEFAULT_HIT_STYLE.itemRadius,
    hitcounter_label_size: DEFAULT_HIT_STYLE.labelSize,
    hitcounter_value_size: DEFAULT_HIT_STYLE.valueSize,
    hitcounter_dot_size: DEFAULT_HIT_STYLE.dotSize,
    hitcounter_orientation: DEFAULT_HIT_STYLE.orientation,
    hitcounter_labels: [...HIT_DEFAULT_LABELS],
    hitcounter_colors: [...HIT_DEFAULT_COLORS],
};

function resolveTosuHost(cfg) {
    const qHost = urlParams.get('tosuHost');
    if (qHost && qHost.trim()) return qHost.trim();
    if (cfg?.tosu_host && String(cfg.tosu_host).trim()) return String(cfg.tosu_host).trim();
    const isHttp = window.location.protocol === 'http:' || window.location.protocol === 'https:';
    if (isHttp && window.location.hostname) return window.location.hostname;
    return '127.0.0.1';
}

function resolveTosuPort(cfg) {
    const qPort = Number(urlParams.get('tosuPort'));
    if (Number.isInteger(qPort) && qPort >= 1024 && qPort <= 65535) return qPort;
    const cfgPort = Number(cfg?.tosu_port);
    if (Number.isInteger(cfgPort) && cfgPort >= 1024 && cfgPort <= 65535) return cfgPort;
    const isHttp = window.location.protocol === 'http:' || window.location.protocol === 'https:';
    const locPort = Number(window.location.port);
    if (isHttp && Number.isInteger(locPort) && locPort >= 1024 && locPort <= 65535) return locPort;
    return 24050;
}

function resolveCompanionPort(cfg) {
    const qPort = Number(urlParams.get('companionPort'));
    if (Number.isInteger(qPort) && qPort >= 1024 && qPort <= 65535) return qPort;
    const activePort = Number(cfg?.active_companion_port);
    if (Number.isInteger(activePort) && activePort >= 1024 && activePort <= 65535) return activePort;
    const cfgPort = Number(cfg?.companion_port);
    if (Number.isInteger(cfgPort) && cfgPort >= 1024 && cfgPort <= 65535) return cfgPort;
    return 24051;
}

function applyLayout() {
    const root = document.getElementById('hit-counter');
    if (!root) return;
    ensureHitCounterConfigState(config);
    applyHitCounterLayout(root, config);
}

function applyConfig(data) {
    Object.assign(config, data);
    applyLayout();
    const nextTosuAddr = `${resolveTosuHost(config)}:${resolveTosuPort(config)}`;
    if (activeTosuAddr && nextTosuAddr !== activeTosuAddr && socket) {
        const old = socket;
        socket = null;
        old.close();
        connect();
    }
    const nextCompPort = resolveCompanionPort(config);
    if (activeCompanionPort && nextCompPort !== activeCompanionPort && configSocket) {
        const old = configSocket;
        configSocket = null;
        old.close();
        connectConfig();
    }
}

async function loadConfig() {
    try {
        const res = await fetch(`../radar/config.json?t=${Date.now()}`);
        if (res.ok) {
            applyConfig(await res.json());
        }
    } catch (_) {}
}

const lastHits = { geki: -1, '300': -1, katu: -1, '100': -1, '50': -1, '0': -1 };
let lastFastPushAt = 0;

function sumHitsObj(h) {
    return (h.geki || 0) + (h['300'] || 0) + (h.katu || 0) + (h['100'] || 0) + (h['50'] || 0) + (h['0'] || 0);
}

function setVal(id, num) {
    const el = document.getElementById(id);
    if (!el) return;
    el.textContent = num;
    el.classList.toggle('is-zero', Number(num) === 0);
}

function updateHitValues(nextValues) {
    if (nextValues.geki !== lastHits.geki) {
        setVal('val-max', nextValues.geki);
        lastHits.geki = nextValues.geki;
    }
    if (nextValues['300'] !== lastHits['300']) {
        setVal('val-perf', nextValues['300']);
        lastHits['300'] = nextValues['300'];
    }
    if (nextValues.katu !== lastHits.katu) {
        setVal('val-great', nextValues.katu);
        lastHits.katu = nextValues.katu;
    }
    if (nextValues['100'] !== lastHits['100']) {
        setVal('val-good', nextValues['100']);
        lastHits['100'] = nextValues['100'];
    }
    if (nextValues['50'] !== lastHits['50']) {
        setVal('val-bad', nextValues['50']);
        lastHits['50'] = nextValues['50'];
    }
    if (nextValues['0'] !== lastHits['0']) {
        setVal('val-miss', nextValues['0']);
        lastHits['0'] = nextValues['0'];
    }

    const g = nextValues.geki || 0;
    const p = nextValues['300'] || 0;
    const ratioStr = p > 0 ? (g / p).toFixed(2) : (g > 0 ? `${g}` : '0.00');
    const ratioEl = document.getElementById('val-ratio');
    const ratioCellEl = document.getElementById('val-ratio-cell');
    if (ratioEl) ratioEl.textContent = ratioStr;
    if (ratioCellEl) ratioCellEl.textContent = ratioStr;
}

function setTosuOffline(offline) {
    const badge = document.getElementById('tosu-offline-badge');
    if (badge) {
        badge.classList.toggle('hidden', !offline);
    }
}

function connect(endpoint = '/websocket/v2') {
    let opened = false;
    const host = resolveTosuHost(config);
    const port = resolveTosuPort(config);
    activeTosuAddr = `${host}:${port}`;
    const ws = new WebSocket(`ws://${activeTosuAddr}${endpoint}`);
    socket = ws;
    ws.onopen = () => {
        opened = true;
        setTosuOffline(false);
    };

    ws.onmessage = (e) => {
        try {
            const data = JSON.parse(e.data);
            const stateNum = data.state?.number ?? data.menu?.state;
            const inPlay = stateNum === undefined || stateNum === 2 || stateNum === 7;
            const hits = inPlay ? (data.play?.hits || data.gameplay?.hits || {}) : {};

            const nextObj = {
                geki: hits.geki ?? 0,
                '300': hits['300'] ?? 0,
                katu: hits.katu ?? 0,
                '100': hits['100'] ?? 0,
                '50': hits['50'] ?? 0,
                '0': hits['0'] ?? 0,
            };
            const curTotal = sumHitsObj(lastHits);
            const nextTotal = sumHitsObj(nextObj);
            if (stateNum === 2 && nextTotal > 0 && nextTotal < curTotal && (performance.now() - lastFastPushAt) < 220) {
                return;
            }
            updateHitValues(nextObj);
        } catch (_) {}
    };

    ws.onerror = () => ws.close();
    ws.onclose = () => {
        if (socket !== ws) return;
        setTosuOffline(true);
        const next = (!opened && endpoint === '/websocket/v2') ? '/ws' : '/websocket/v2';
        setTimeout(() => connect(next), 1500);
    };
}

function connectConfig() {
    const host = resolveTosuHost(config);
    const port = resolveCompanionPort(config);
    activeCompanionPort = port;
    const ws = new WebSocket(`ws://${host}:${port}`);
    configSocket = ws;

    ws.onmessage = (e) => {
        try {
            const data = JSON.parse(e.data);
            if (data.event === 'config-updated') {
                applyConfig(data);
            } else if (data.event === 'hits-updated' && Array.isArray(data.hits)) {
                lastFastPushAt = performance.now();
                updateHitValues({
                    geki: data.hits[0] ?? 0,
                    '300': data.hits[1] ?? 0,
                    katu: data.hits[2] ?? 0,
                    '100': data.hits[3] ?? 0,
                    '50': data.hits[4] ?? 0,
                    '0': data.hits[5] ?? 0,
                });
            } else if (data.event === 'msd-updated' && data.ratings) {
                config._adaptive = data.ratings.adaptive_accent
                    ? {
                        accent: data.ratings.adaptive_accent || '',
                        bg: data.ratings.adaptive_bg || '',
                        item_bg: data.ratings.adaptive_item_bg || '',
                        border: data.ratings.adaptive_border || '',
                    }
                    : null;
                if (config.hitcounter_auto_bg_theme) {
                    applyLayout();
                }
            }
        } catch (_) {}
    };

    ws.onclose = () => {
        if (configSocket !== ws) return;
        setTimeout(connectConfig, 1500);
    };
}

applyLayout();
connect();
connectConfig();
loadConfig();
