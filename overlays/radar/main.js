const SKILL_KEYS = ['stream', 'jumpstream', 'handstream', 'stamina', 'jackspeed', 'chordjack', 'technical'];
const urlParams = new URLSearchParams(window.location.search);

const config = {
    show_radar_overlay: true,
    tosu_host: '127.0.0.1',
    tosu_port: 24050,
    companion_port: 24051,
    active_companion_port: 24051,
    bg_opacity: 0.92,
    scale: 1.0,
    radar_pos_x: 0,
    radar_pos_y: 0,
    radar_radius: 12,
    radar_width: 312,
    radar_height: 320,
    radar_auto_height: true,
    accent_color: '#e588a6',
    radar_bg_color: '#141415',
    radar_border_color: '#28282b',
    radar_text_color: '#a1a1a6',
    show_radar: true,
    radar_show_header: true,
    radar_show_badge: true,
    radar_show_dots: true,
    radar_fill_opacity: 0.22,
    radar_max_value: 38.0,
    radar_show_dan: true,
    radar_dan_engine: 'combined',
    radar_dan_color_mode: 'tier',
    radar_dan_color: '#f472b6',
    radar_dan_badge_bg: '#1d1d21',
    radar_dan_size: 16,
    radar_dan_show_numeric: true,
    radar_dan_show_breakdown: true,
    radar_show_map_bg: true,
    radar_map_bg_opacity: 0.28,
    radar_auto_bg_theme: true,
    radar_show_strain_graph: true,
    radar_graph_height: 34,
    radar_graph_fill_opacity: 0.24,
    radar_graph_color_mode: 'accent',
    radar_graph_color: '#e588a6',
    radar_graph_show_cursor: true,
    visible_skills: {
        stream: true,
        jumpstream: true,
        handstream: true,
        stamina: true,
        jackspeed: true,
        chordjack: true,
        technical: true,
    },
};

let currentRatings = null;
let lastMapKey = '';
let currentBeatmapKey = '';
let currentRate = 1.0;
let bgSeq = 0;
let calcTimer = null;
let tosuSocket = null;
let companionSocket = null;
let activeTosuAddr = '';
let activeCompanionPort = 0;

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

const overlayEl = document.getElementById('overlay');
const headerEl = document.getElementById('header');
const titleEl = document.getElementById('song-title');
const diffEl = document.getElementById('song-difficulty');
const artistEl = document.getElementById('song-artist');
const badgeEl = document.getElementById('map-type-badge');
const tosuOfflineBadgeEl = document.getElementById('tosu-offline-badge');
const tosuOfflineBarEl = document.getElementById('tosu-offline-bar');
let tosuConnected = false;
const chartWrapEl = document.getElementById('chart-wrap');
const mapBgEl = document.getElementById('map-bg');
const danSectionEl = document.getElementById('dan-section');
const danSymbolEl = document.getElementById('dan-symbol');
const danTitleEl = document.getElementById('dan-title');
const danSubEl = document.getElementById('dan-sub');
const danNumericEl = document.getElementById('dan-numeric');
const danProgressEl = document.getElementById('dan-progress');
const graphSectionEl = document.getElementById('graph-section');
const graphCanvas = document.getElementById('strain-graph');
const graphCtx = graphCanvas ? graphCanvas.getContext('2d') : null;
const canvas = document.getElementById('radar-chart');
const ctx = canvas ? canvas.getContext('2d') : null;
let liveSongTimeMs = 0;

const DAN_TIER_COLORS = {
    1: '#7dd3fc', 2: '#7dd3fc', 3: '#7dd3fc',
    4: '#93c5fd', 5: '#93c5fd', 6: '#93c5fd',
    7: '#a5b4fc', 8: '#a5b4fc',
    9: '#e2e8f0', 10: '#e2e8f0',
    11: '#ff5a5a', 12: '#ffd84d', 13: '#00ffd5', 14: '#ff7b00',
    15: '#ff7a9e', 16: '#d7f7ff', 17: '#ff2b2b', 18: '#cc00ff',
    19: '#f8fafc', 20: '#ffffff',
};

const DAN_TIER_META = [
    null,
    ['I', '1st dan'], ['II', '2nd dan'], ['III', '3rd dan'],
    ['IV', '4th dan'], ['V', '5th dan'], ['VI', '6th dan'],
    ['VII', '7th dan'], ['VIII', '8th dan'], ['IX', '9th dan'], ['X', '10th dan'],
    ['α', 'alpha'], ['β', 'beta'], ['γ', 'gamma'], ['δ', 'delta'],
    ['ε', 'epsilon'], ['ζ', 'zeta'], ['η', 'eta'], ['θ', 'theta'],
    ['ι', 'iota'], ['κ', 'kappa'],
];

function formatDanTierJs(level) {
    const num = Number(level || 0);
    if (num <= 0.05) return { tier: 0, symbol: '-', name: 'unrated', variant: '', sub: '' };
    const clamped = Math.min(20.99, Math.max(1.0, num));
    const tier = Math.min(20, Math.max(1, Math.floor(clamped)));
    const frac = clamped - tier;
    const variant = frac < 0.20 ? '--' : frac < 0.40 ? '-' : frac < 0.60 ? '' : frac < 0.80 ? '+' : '++';
    const sub = frac < 0.333 ? 'low' : frac < 0.667 ? 'mid' : 'high';
    const [symbol, name] = DAN_TIER_META[tier] || ['κ', 'kappa'];
    return { tier, symbol, name, variant, sub, level: clamped };
}

function hexToRgba(hex, alpha) {
    const clean = String(hex || '#141415').replace('#', '');
    if (clean.length !== 6) return `rgba(20, 20, 21, ${alpha})`;
    const r = parseInt(clean.slice(0, 2), 16);
    const g = parseInt(clean.slice(2, 4), 16);
    const b = parseInt(clean.slice(4, 6), 16);
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

function applyVisualConfig() {
    if (!overlayEl) return;
    if (config.show_radar_overlay === false) {
        overlayEl.classList.add('hidden');
        return;
    }
    overlayEl.classList.remove('hidden');

    const root = document.documentElement;
    const useAdaptive = config.radar_auto_bg_theme && currentRatings?.adaptive_accent;
    const effAccent = useAdaptive ? currentRatings.adaptive_accent : (config.accent_color || '#e588a6');
    const effBg = useAdaptive ? currentRatings.adaptive_bg : (config.radar_bg_color || '#141415');
    const effBorder = useAdaptive ? currentRatings.adaptive_border : (config.radar_border_color || '#28282b');

    const radarW = Math.max(150, Number(config.radar_width ?? 312));
    overlayEl.style.width = `${radarW}px`;
    overlayEl.style.height = config.radar_auto_height === false
        ? `${Math.max(140, Number(config.radar_height ?? 320))}px`
        : 'auto';

    root.style.setProperty('--bg-opacity', config.bg_opacity ?? 0.92);
    root.style.setProperty('--ui-scale', config.scale ?? 1.0);
    root.style.setProperty('--pos-x', `${config.radar_pos_x || 0}px`);
    root.style.setProperty('--pos-y', `${config.radar_pos_y || 0}px`);
    root.style.setProperty('--card-radius', `${config.radar_radius ?? 12}px`);
    root.style.setProperty('--accent-color', effAccent);
    root.style.setProperty('--bg-surface', hexToRgba(effBg, config.bg_opacity ?? 0.92));
    root.style.setProperty('--card-border', effBorder);
    root.style.setProperty('--text-secondary', config.radar_text_color || '#a1a1a6');
    const danBadgeBg = (useAdaptive || !config.radar_dan_badge_bg || config.radar_dan_badge_bg.toLowerCase() === '#1d1d21')
        ? effBg
        : config.radar_dan_badge_bg;
    root.style.setProperty('--dan-badge-bg', danBadgeBg);
    root.style.setProperty('--dan-size', `${config.radar_dan_size ?? 16}px`);
    root.style.setProperty('--map-bg-opacity', config.radar_map_bg_opacity ?? 0.28);
    root.style.setProperty('--graph-height', `${Math.max(16, Number(config.radar_graph_height ?? 34))}px`);

    if (graphSectionEl) {
        const showCurve = config.radar_show_strain_graph !== false;
        graphSectionEl.classList.toggle('hidden', !showCurve);
    }

    if (mapBgEl) {
        mapBgEl.classList.toggle('hidden', config.radar_show_map_bg === false);
    }
    if (danSectionEl) {
        danSectionEl.classList.toggle('hidden', config.radar_show_dan === false);
    }

    if (headerEl) {
        headerEl.classList.toggle('hidden', config.radar_show_header === false);
    }
    const headerVisible = config.radar_show_header !== false;
    if (tosuOfflineBadgeEl) {
        tosuOfflineBadgeEl.classList.toggle('hidden', tosuConnected || !headerVisible);
    }
    if (tosuOfflineBarEl) {
        tosuOfflineBarEl.classList.toggle('hidden', tosuConnected || headerVisible);
    }
    if (badgeEl) {
        badgeEl.classList.toggle('hidden', !tosuConnected || config.radar_show_badge === false || !badgeEl.textContent);
    }
    if (chartWrapEl) {
        chartWrapEl.classList.toggle('hidden', config.show_radar === false);
    }

    const vis = config.visible_skills || {};
    for (const key of SKILL_KEYS) {
        const row = document.querySelector(`.skill-row[data-skill="${key}"]`);
        if (row) {
            row.classList.toggle('hidden', vis[key] === false);
        }
    }

    if (currentRatings) {
        drawRadar(currentRatings);
    } else {
        drawEmptyGrid();
    }
}

function getRadarCanvasBox() {
    const radarW = Math.max(150, Number(config.radar_width ?? 312));
    return Math.max(88, Math.min(220, Math.round((radarW - 28) * 0.48)));
}

function setupCanvas() {
    if (!canvas || !ctx) return 148;
    const dpr = window.devicePixelRatio || 1;
    const size = getRadarCanvasBox();
    if (canvas.width !== size * dpr || canvas.height !== size * dpr) {
        canvas.width = size * dpr;
        canvas.height = size * dpr;
        canvas.style.width = `${size}px`;
        canvas.style.height = `${size}px`;
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    return size;
}

function drawEmptyGrid() {
    if (!ctx) return;
    const size = setupCanvas();
    ctx.clearRect(0, 0, size, size);
    const cx = size * 0.5;
    const cy = size * 0.5;
    const maxR = Math.max(26, size * 0.39);
    const n = SKILL_KEYS.length;

    ctx.strokeStyle = 'rgba(237, 237, 237, 0.08)';
    ctx.lineWidth = 1;

    for (let step = 1; step <= 4; step++) {
        const r = maxR * (step / 4);
        ctx.beginPath();
        for (let i = 0; i < n; i++) {
            const angle = -Math.PI / 2 + i * ((2 * Math.PI) / n);
            const x = cx + r * Math.cos(angle);
            const y = cy + r * Math.sin(angle);
            if (i === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x, y);
        }
        ctx.closePath();
        ctx.stroke();
    }

    for (let i = 0; i < n; i++) {
        const angle = -Math.PI / 2 + i * ((2 * Math.PI) / n);
        ctx.beginPath();
        ctx.moveTo(cx, cy);
        ctx.lineTo(cx + maxR * Math.cos(angle), cy + maxR * Math.sin(angle));
        ctx.stroke();
    }
}

function drawRadar(ratings) {
    if (!ctx) return;
    drawEmptyGrid();

    const size = getRadarCanvasBox();
    const cx = size * 0.5;
    const cy = size * 0.5;
    const maxR = Math.max(26, size * 0.39);
    const activeKeys = SKILL_KEYS.filter(k => (config.visible_skills?.[k] ?? true) !== false);
    const keys = activeKeys.length >= 3 ? activeKeys : SKILL_KEYS;
    const n = keys.length;
    const useAdaptive = config.radar_auto_bg_theme && currentRatings?.adaptive_accent;
    const accent = useAdaptive ? currentRatings.adaptive_accent : (config.accent_color || '#e588a6');

    const maxVal = Math.max(5, Number(config.radar_max_value || 38.0));
    const pts = keys.map((k, i) => {
        const val = Number(ratings[k] || 0);
        const norm = Math.min(1, Math.max(0.08, val / maxVal));
        const r = maxR * norm;
        const angle = -Math.PI / 2 + i * ((2 * Math.PI) / n);
        return [cx + r * Math.cos(angle), cy + r * Math.sin(angle)];
    });

    ctx.beginPath();
    pts.forEach(([x, y], i) => {
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    });
    ctx.closePath();
    ctx.fillStyle = hexToRgba(accent, Math.min(1, Math.max(0, Number(config.radar_fill_opacity ?? 0.22))));
    ctx.fill();
    ctx.strokeStyle = accent;
    ctx.lineWidth = 1.75;
    ctx.stroke();

    if (config.radar_show_dots !== false) {
        ctx.fillStyle = accent;
        pts.forEach(([x, y]) => {
            ctx.beginPath();
            ctx.arc(x, y, 2.2, 0, Math.PI * 2);
            ctx.fill();
        });
    }
}

function reorderSkillRows(ratings) {
    const listEl = document.querySelector('.skill-list');
    if (!listEl) return;
    const rowsBySkill = new Map();
    listEl.querySelectorAll('.skill-row[data-skill]').forEach((row) => {
        if (row.dataset.skill) rowsBySkill.set(row.dataset.skill, row);
    });
    const ordered = ratings
        ? [...SKILL_KEYS].sort((a, b) => {
            const diff = Number(ratings[b] || 0) - Number(ratings[a] || 0);
            return Math.abs(diff) > 1e-4 ? diff : SKILL_KEYS.indexOf(a) - SKILL_KEYS.indexOf(b);
        })
        : SKILL_KEYS;
    for (const skill of ordered) {
        const row = rowsBySkill.get(skill);
        if (row) listEl.appendChild(row);
    }
}

function resetRatingsForNewMap() {
    currentRatings = null;
    reorderSkillRows(null);
    for (const id of ['val-overall', 'val-stream', 'val-jumpstream', 'val-handstream', 'val-stamina', 'val-jackspeed', 'val-chordjack', 'val-technical']) {
        const el = document.getElementById(id);
        if (el) el.textContent = '-';
    }
    if (badgeEl) {
        badgeEl.textContent = '';
        badgeEl.classList.add('hidden');
    }
    if (danSymbolEl) danSymbolEl.textContent = '-';
    if (danTitleEl) danTitleEl.textContent = 'unrated';
    if (danSubEl) danSubEl.textContent = 'xdc1 + companella';
    if (danNumericEl) {
        danNumericEl.textContent = '';
        danNumericEl.classList.add('hidden');
    }
    if (danProgressEl) danProgressEl.style.width = '0%';
    document.documentElement.style.setProperty('--dan-color', config.accent_color || '#e588a6');
    applyVisualConfig();
    drawStrainGraph(null, config.accent_color || '#e588a6', config.accent_color || '#e588a6');
}

function refreshBeatmapBackground(beatmapKey) {
    if (!mapBgEl) return;
    if (config.radar_show_map_bg === false || !beatmapKey) {
        mapBgEl.style.backgroundImage = '';
        return;
    }
    const host = resolveTosuHost(config);
    const port = resolveTosuPort(config);
    const seq = ++bgSeq;
    const isSameHttpOrigin = (window.location.protocol === 'http:' || window.location.protocol === 'https:')
        && Number(window.location.port || 80) === port;
    const bgUrl = isSameHttpOrigin
        ? `/files/beatmap/background?t=${encodeURIComponent(beatmapKey)}_${seq}`
        : `http://${host}:${port}/files/beatmap/background?t=${encodeURIComponent(beatmapKey)}_${seq}`;
    const img = new Image();
    img.onload = () => {
        if (seq === bgSeq && (!currentBeatmapKey || currentBeatmapKey === beatmapKey)) {
            mapBgEl.style.backgroundImage = `url("${bgUrl}")`;
        }
    };
    img.onerror = () => {
        if (seq === bgSeq) {
            mapBgEl.style.backgroundImage = '';
        }
    };
    img.src = bgUrl;
}

function renderRatings(r) {
    if (!r || r.overall === undefined) return;
    currentRatings = r;
    const setVal = (id, v) => {
        const el = document.getElementById(id);
        if (el) el.textContent = Number(v || 0).toFixed(2);
    };
    setVal('val-overall', r.overall);
    setVal('val-stream', r.stream);
    setVal('val-jumpstream', r.jumpstream);
    setVal('val-handstream', r.handstream);
    setVal('val-stamina', r.stamina);
    setVal('val-jackspeed', r.jackspeed);
    setVal('val-chordjack', r.chordjack);
    setVal('val-technical', r.technical);
    reorderSkillRows(r);

    if (badgeEl) {
        if (r.map_type && config.radar_show_badge !== false) {
            badgeEl.textContent = String(r.map_type).toLowerCase().replace(' map', '');
            badgeEl.classList.remove('hidden');
        } else {
            badgeEl.classList.add('hidden');
        }
    }

    const d = r.dan || {};
    const mode = config.radar_dan_engine || 'combined';
    let rawLvl = Number(d.level || 0);
    if (mode === 'xdc1' && Number(d.xdc1_dan) > 0.5) rawLvl = Number(d.xdc1_dan);
    else if (mode === 'companella' && Number(d.companella_dan) > 0.5) rawLvl = Number(d.companella_dan);
    if (rawLvl <= 0.05 && Number(r.overall) > 0.1) rawLvl = Number(r.overall) * 0.52;

    const info = formatDanTierJs(rawLvl);
    const useAdaptive = config.radar_auto_bg_theme && r.adaptive_accent;
    const effAccent = useAdaptive ? r.adaptive_accent : (config.accent_color || '#e588a6');
    let danColor = effAccent;
    if (config.radar_dan_color_mode === 'tier') danColor = DAN_TIER_COLORS[info.tier] || effAccent;
    else if (config.radar_dan_color_mode === 'custom') danColor = config.radar_dan_color || '#f472b6';
    document.documentElement.style.setProperty('--dan-color', danColor);

    if (danSymbolEl) danSymbolEl.textContent = `${info.symbol}${info.variant}`;
    if (danTitleEl) danTitleEl.textContent = info.sub ? `${info.name} · ${info.sub}` : info.name;
    if (danSubEl) {
        danSubEl.classList.toggle('hidden', config.radar_dan_show_breakdown === false);
        if (Number(d.xdc1_dan) > 0.1) {
            danSubEl.textContent = `xdc1 ${Number(d.xdc1_dan).toFixed(2)} · comp ${Number(d.companella_dan || 0).toFixed(2)}`;
        } else if (Number(d.companella_dan) > 0.1) {
            danSubEl.textContent = `comp ${Number(d.companella_dan).toFixed(2)}`;
        } else {
            danSubEl.textContent = 'xdc1 + companella';
        }
    }
    if (danNumericEl) {
        danNumericEl.classList.toggle('hidden', config.radar_dan_show_numeric === false || rawLvl <= 0.05);
        danNumericEl.textContent = rawLvl > 0.05 ? rawLvl.toFixed(2) : '';
    }
    if (danProgressEl) {
        const frac = rawLvl > 0.05 ? Math.min(1, Math.max(0.04, rawLvl - Math.floor(rawLvl))) : 0;
        danProgressEl.style.width = `${Math.round(frac * 92)}%`;
    }
    if (mapBgEl && config.radar_show_map_bg !== false && (!mapBgEl.style.backgroundImage || !currentBeatmapKey)) {
        refreshBeatmapBackground(currentBeatmapKey || lastMapKey || String(Date.now()));
    }

    applyVisualConfig();
    drawStrainGraph(r, effAccent, danColor);
}

function drawStrainGraph(r, effAccent, danColor) {
    if (!graphCanvas || !graphCtx) return;
    if (config.radar_show_strain_graph === false) return;
    const curve = Array.isArray(r?.dan?.strain_curve) ? r.dan.strain_curve : [];
    const gh = Math.max(16, Number(config.radar_graph_height ?? 34));
    const gw = graphCanvas.clientWidth || 284;
    const dpr = window.devicePixelRatio || 1;
    if (graphCanvas.width !== gw * dpr || graphCanvas.height !== gh * dpr) {
        graphCanvas.width = gw * dpr;
        graphCanvas.height = gh * dpr;
    }
    graphCtx.setTransform(dpr, 0, 0, dpr, 0, 0);
    graphCtx.clearRect(0, 0, gw, gh);

    let gcol = effAccent;
    if (config.radar_graph_color_mode === 'dan') gcol = danColor;
    else if (config.radar_graph_color_mode === 'custom') gcol = config.radar_graph_color || effAccent;

    if (curve.length < 2) {
        const midY = gh - 6;
        graphCtx.beginPath();
        graphCtx.moveTo(6, midY);
        graphCtx.lineTo(gw - 6, midY);
        graphCtx.strokeStyle = hexToRgba(gcol, 0.28);
        graphCtx.lineWidth = 1;
        graphCtx.stroke();
        return;
    }

    const padX = 3;
    const innerW = Math.max(10, gw - 6);
    const innerTop = 4;
    const innerBot = gh - 3;
    const innerH = Math.max(4, innerBot - innerTop);

    graphCtx.beginPath();
    curve.forEach((v, i) => {
        const px = padX + (i / (curve.length - 1)) * innerW;
        const py = innerBot - Math.min(1, Math.max(0, Number(v || 0))) * innerH;
        if (i === 0) graphCtx.moveTo(px, py);
        else graphCtx.lineTo(px, py);
    });
    graphCtx.strokeStyle = gcol;
    graphCtx.lineWidth = 1.4;
    graphCtx.stroke();

    graphCtx.lineTo(padX + innerW, innerBot);
    graphCtx.lineTo(padX, innerBot);
    graphCtx.closePath();
    graphCtx.fillStyle = hexToRgba(gcol, Math.min(1, Math.max(0, Number(config.radar_graph_fill_opacity ?? 0.24))));
    graphCtx.fill();

    const dur = Number(r?.dan?.duration_ms || 0);
    if (config.radar_graph_show_cursor !== false && liveSongTimeMs > 0 && dur > 0) {
        const prog = Math.min(1, Math.max(0, liveSongTimeMs / dur));
        const cx = padX + prog * innerW;
        graphCtx.beginPath();
        graphCtx.moveTo(cx, 2);
        graphCtx.lineTo(cx, gh - 2);
        graphCtx.strokeStyle = 'rgba(237, 237, 237, 0.85)';
        graphCtx.lineWidth = 1.2;
        graphCtx.stroke();
    }
}

function payloadMatchesCurrentMap(data) {
    if (!currentBeatmapKey) return true;
    if (data.beatmap_key) {
        if (data.beatmap_key !== currentBeatmapKey) return false;
        if (typeof data.rate === 'number' && Math.abs(data.rate - currentRate) > 0.02) return false;
        return true;
    }
    if (data.map_key && lastMapKey) {
        return data.map_key === lastMapKey || String(data.map_key).startsWith(`${currentBeatmapKey}||`);
    }
    return true;
}

async function pollMsdFile(expectedBeatmapKey, expectedRate, attemptsLeft = 30) {
    if (currentBeatmapKey && expectedBeatmapKey && expectedBeatmapKey !== currentBeatmapKey) {
        return;
    }
    try {
        const res = await fetch(`msd.json?t=${Date.now()}`);
        if (res.ok) {
            const data = await res.json();
            const keyOk = !expectedBeatmapKey
                || (data.beatmap_key ? data.beatmap_key === expectedBeatmapKey : String(data.map_key || '').startsWith(`${expectedBeatmapKey}||`));
            const rateOk = typeof data.rate !== 'number' || !expectedRate || Math.abs(data.rate - expectedRate) <= 0.02;
            if (data.ratings && data.ratings.overall !== undefined && keyOk && rateOk) {
                renderRatings(data.ratings);
                return;
            }
        }
    } catch (_) {}

    if (attemptsLeft > 0) {
        clearTimeout(calcTimer);
        calcTimer = setTimeout(() => pollMsdFile(expectedBeatmapKey, expectedRate, attemptsLeft - 1), 150);
    }
}

function handleTosuMessage(data) {
    let title = '';
    let artist = '';
    let difficulty = '';
    let id = '';
    let md5 = '';
    let modsStr = '';
    let rate = 1.0;

    if (data.beatmap) {
        title = data.beatmap.title || data.beatmap.titleUnicode || '';
        artist = data.beatmap.artist || data.beatmap.artistUnicode || '';
        difficulty = data.beatmap.difficulty || data.beatmap.version || '';
        id = String(data.beatmap.id || '');
        md5 = data.beatmap.checksum || data.beatmap.md5 || '';
        modsStr = data.play?.mods?.name || data.beatmap.mods || data.menu?.mods?.str || '';
        const r = Number(data.play?.mods?.rate || 0);
        if (r > 0.1 && Math.abs(r - 1.0) > 0.01) {
            rate = r;
        }
    } else if (data.menu?.bm) {
        title = data.menu.bm.metadata?.title || data.menu.bm.metadata?.titleUnicode || '';
        artist = data.menu.bm.metadata?.artist || data.menu.bm.metadata?.artistUnicode || '';
        difficulty = data.menu.bm.metadata?.difficulty || '';
        id = String(data.menu.bm.id || '');
        md5 = data.menu.bm.md5 || '';
        modsStr = data.menu.mods?.str || '';
    }
    if (rate === 1.0 && modsStr) {
        const u = modsStr.toUpperCase();
        if (u.includes('DT') || u.includes('NC')) rate = 1.5;
        else if (u.includes('HT')) rate = 0.75;
    }

    if (title && titleEl) titleEl.textContent = title;
    if (diffEl) diffEl.textContent = difficulty ? `[${difficulty}]` : '';
    if (artist && artistEl) artistEl.textContent = artist;

    const nextTime = Number(data.beatmap?.time?.live ?? data.menu?.bm?.time?.current ?? 0);
    if (nextTime !== liveSongTimeMs) {
        liveSongTimeMs = nextTime;
        if (currentRatings && config.radar_show_strain_graph !== false && config.radar_graph_show_cursor !== false) {
            const useAdaptive = config.radar_auto_bg_theme && currentRatings?.adaptive_accent;
            const effAccent = useAdaptive ? currentRatings.adaptive_accent : (config.accent_color || '#e588a6');
            drawStrainGraph(currentRatings, effAccent, effAccent);
        }
    }

    const useKey = md5 || id;
    if (useKey && useKey !== '0') {
        const mapKey = `${useKey}||${rate.toFixed(2)}`;
        if (mapKey !== lastMapKey) {
            const beatmapChanged = useKey !== currentBeatmapKey;
            lastMapKey = mapKey;
            currentBeatmapKey = useKey;
            currentRate = rate;
            resetRatingsForNewMap();
            if (beatmapChanged) {
                refreshBeatmapBackground(useKey);
            }
            clearTimeout(calcTimer);
            pollMsdFile(useKey, rate, 35);
        }
    }
}

function connectTosu(endpoint = '/websocket/v2') {
    let opened = false;
    const host = resolveTosuHost(config);
    const port = resolveTosuPort(config);
    activeTosuAddr = `${host}:${port}`;
    const ws = new WebSocket(`ws://${activeTosuAddr}${endpoint}`);
    tosuSocket = ws;
    ws.onopen = () => {
        opened = true;
        if (!tosuConnected) {
            tosuConnected = true;
            applyVisualConfig();
        }
    };
    ws.onmessage = (e) => {
        try {
            handleTosuMessage(JSON.parse(e.data));
        } catch (_) {}
    };
    ws.onerror = () => ws.close();
    ws.onclose = () => {
        if (tosuSocket !== ws) return;
        if (tosuConnected) {
            tosuConnected = false;
            applyVisualConfig();
        }
        const next = (!opened && endpoint === '/websocket/v2') ? '/ws' : '/websocket/v2';
        setTimeout(() => connectTosu(next), 1500);
    };
}

function connectCompanion() {
    const host = resolveTosuHost(config);
    const port = resolveCompanionPort(config);
    activeCompanionPort = port;
    const ws = new WebSocket(`ws://${host}:${port}`);
    companionSocket = ws;
    ws.onmessage = (e) => {
        try {
            const data = JSON.parse(e.data);
            if (data.event === 'config-updated') {
                Object.assign(config, data);
                applyVisualConfig();
                const nextTosuAddr = `${resolveTosuHost(config)}:${resolveTosuPort(config)}`;
                if (activeTosuAddr && nextTosuAddr !== activeTosuAddr && tosuSocket) {
                    const old = tosuSocket;
                    tosuSocket = null;
                    old.close();
                    connectTosu();
                }
                const nextCompPort = resolveCompanionPort(config);
                if (activeCompanionPort && nextCompPort !== activeCompanionPort && companionSocket) {
                    const old = companionSocket;
                    companionSocket = null;
                    old.close();
                    connectCompanion();
                }
            } else if (data.event === 'msd-updated' && data.ratings) {
                if (!payloadMatchesCurrentMap(data)) return;
                clearTimeout(calcTimer);
                if (data.song_title && titleEl) titleEl.textContent = data.song_title;
                if (data.song_difficulty !== undefined && diffEl) {
                    diffEl.textContent = data.song_difficulty ? `[${data.song_difficulty}]` : '';
                }
                if (data.song_artist && artistEl) artistEl.textContent = data.song_artist;
                renderRatings(data.ratings);
            }
        } catch (_) {}
    };
    ws.onclose = () => {
        if (companionSocket !== ws) return;
        setTimeout(connectCompanion, 3000);
    };
}

async function init() {
    drawEmptyGrid();
    try {
        const res = await fetch(`config.json?t=${Date.now()}`);
        if (res.ok) {
            Object.assign(config, await res.json());
        }
    } catch (_) {}
    applyVisualConfig();
    pollMsdFile('', 0, 2);
    connectTosu();
    connectCompanion();
}

init();
