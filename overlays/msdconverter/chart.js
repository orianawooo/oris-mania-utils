const SKILL_KEYS = ['stream', 'jumpstream', 'handstream', 'stamina', 'jackspeed', 'chordjack', 'technical'];
const SKILL_LABELS = ['Stream', 'JS', 'HS', 'Stam', 'Jack', 'CJ', 'Tech'];

const SKILL_COLORS = {
    stream:     '#e07a5f',
    jumpstream: '#3d405b',
    handstream: '#81b29a',
    stamina:    '#f2cc8f',
    jackspeed:  '#a67c52',
    chordjack:  '#d4a373',
    technical:  '#e9d8a6',
};

let chartCanvas = null;
let chartCtx = null;

export function initChart(canvasId) {
    chartCanvas = document.getElementById(canvasId);
    if (chartCanvas) {
        chartCtx = chartCanvas.getContext('2d');
        const dpr = window.devicePixelRatio || 1;
        const rect = chartCanvas.getBoundingClientRect();
        chartCanvas.width = rect.width * dpr;
        chartCanvas.height = rect.height * dpr;
        chartCtx.scale(dpr, dpr);
    }
}

export function drawRadar(r) {
    if (!chartCanvas || !chartCtx) return;

    const vals = SKILL_KEYS.map(k => r[k] || 0);
    const maxVal = Math.max(...vals, 20);
    const sides = SKILL_KEYS.length;
    const rect = chartCanvas.getBoundingClientRect();
    const W = rect.width;
    const H = rect.height;
    const cx = W / 2;
    const cy = H / 2;
    const radius = Math.min(cx, cy) - 22;

    chartCtx.clearRect(0, 0, W, H);

    for (let level = 1; level <= 4; level++) {
        const r = radius * (level / 4);
        chartCtx.beginPath();
        for (let i = 0; i < sides; i++) {
            const angle = (Math.PI * 2 * i / sides) - Math.PI / 2;
            const x = cx + r * Math.cos(angle);
            const y = cy + r * Math.sin(angle);
            i === 0 ? chartCtx.moveTo(x, y) : chartCtx.lineTo(x, y);
        }
        chartCtx.closePath();
        chartCtx.strokeStyle = 'rgba(166, 124, 82, 0.15)';
        chartCtx.lineWidth = 1;
        chartCtx.stroke();
    }

    for (let i = 0; i < sides; i++) {
        const angle = (Math.PI * 2 * i / sides) - Math.PI / 2;
        chartCtx.beginPath();
        chartCtx.moveTo(cx, cy);
        chartCtx.lineTo(cx + radius * Math.cos(angle), cy + radius * Math.sin(angle));
        chartCtx.strokeStyle = 'rgba(166, 124, 82, 0.1)';
        chartCtx.lineWidth = 1;
        chartCtx.stroke();
    }

    chartCtx.beginPath();
    for (let i = 0; i < sides; i++) {
        const angle = (Math.PI * 2 * i / sides) - Math.PI / 2;
        const dist = radius * Math.min(vals[i] / maxVal, 1);
        const x = cx + dist * Math.cos(angle);
        const y = cy + dist * Math.sin(angle);
        i === 0 ? chartCtx.moveTo(x, y) : chartCtx.lineTo(x, y);
    }
    chartCtx.closePath();
    chartCtx.fillStyle = 'rgba(166, 124, 82, 0.15)';
    chartCtx.fill();
    chartCtx.strokeStyle = 'rgba(166, 124, 82, 0.6)';
    chartCtx.lineWidth = 1.5;
    chartCtx.stroke();

    for (let i = 0; i < sides; i++) {
        const angle = (Math.PI * 2 * i / sides) - Math.PI / 2;
        const dist = radius * Math.min(vals[i] / maxVal, 1);
        const x = cx + dist * Math.cos(angle);
        const y = cy + dist * Math.sin(angle);
        chartCtx.beginPath();
        chartCtx.arc(x, y, 3, 0, Math.PI * 2);
        chartCtx.fillStyle = SKILL_COLORS[SKILL_KEYS[i]] || '#000000';
        chartCtx.fill();
    }

    chartCtx.font = '500 9.5px Inter, sans-serif';
    chartCtx.textAlign = 'center';
    chartCtx.textBaseline = 'middle';
    for (let i = 0; i < sides; i++) {
        const angle = (Math.PI * 2 * i / sides) - Math.PI / 2;
        const labelDist = radius + 14;
        const x = cx + labelDist * Math.cos(angle);
        const y = cy + labelDist * Math.sin(angle);
        chartCtx.fillStyle = '#5d4037';
        chartCtx.fillText(SKILL_LABELS[i], x, y);
    }
}

export function clearRadar() {
    if (!chartCanvas || !chartCtx) return;
    const rect = chartCanvas.getBoundingClientRect();
    chartCtx.clearRect(0, 0, rect.width, rect.height);
}
