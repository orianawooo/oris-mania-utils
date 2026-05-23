const SKILL_KEYS = ['stream', 'jumpstream', 'handstream', 'stamina', 'jackspeed', 'chordjack', 'technical'];
const SKILL_LABELS = ['Stream', 'JS', 'HS', 'Stam', 'Jack', 'CJ', 'Tech'];

const SKILL_COLORS = {
    stream:     '#ff8ab3',
    jumpstream: '#8d6ad6',
    handstream: '#ffb2d8',
    stamina:    '#ffc8a8',
    jackspeed:  '#d96d98',
    chordjack:  '#ff9fc1',
    technical:  '#f3b7e4',
};

let chartCanvas = null;
let chartCtx = null;
let chartW = 0;
let chartH = 0;
let resizeTimeout = null;
let lastRatings = null;

export function initChart(canvasId) {
    chartCanvas = document.getElementById(canvasId);
    if (chartCanvas) {
        chartCtx = chartCanvas.getContext('2d');
        const dpr = window.devicePixelRatio || 1;
        const fixedSize = 200;
        chartW = fixedSize;
        chartH = fixedSize;
        
        chartCanvas.width = fixedSize * dpr;
        chartCanvas.height = fixedSize * dpr;
        chartCtx.scale(dpr, dpr);
    }
}

export function drawRadar(r) {
    if (!r) return;
    lastRatings = r;
    if (!chartCanvas || !chartCtx || chartW === 0 || chartH === 0) return;

    const vals = SKILL_KEYS.map(k => r[k] || 0);
    const maxVal = Math.max(...vals, 20);
    const sides = SKILL_KEYS.length;
    const cx = chartW / 2;
    const cy = chartH / 2;
    const radius = Math.min(cx, cy) - 22;

    chartCtx.clearRect(0, 0, chartW, chartH);

    const setupScreen = document.getElementById('setup-screen');
    const isDarkMode = !setupScreen || setupScreen.classList.contains('dark-mode');

    const gridColor = isDarkMode ? 'rgba(232, 232, 255, 0.1)' : 'rgba(166, 124, 82, 0.15)';
    const webColor = isDarkMode ? 'rgba(232, 232, 255, 0.08)' : 'rgba(166, 124, 82, 0.1)';
    const fillColor = isDarkMode ? 'rgba(232, 232, 255, 0.12)' : 'rgba(166, 124, 82, 0.15)';
    const strokeColor = isDarkMode ? 'rgba(232, 232, 255, 0.5)' : 'rgba(166, 124, 82, 0.6)';
    const textColor = isDarkMode ? '#e8e8ff' : '#5d4037';

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
        chartCtx.strokeStyle = gridColor;
        chartCtx.lineWidth = 1;
        chartCtx.stroke();
    }

    for (let i = 0; i < sides; i++) {
        const angle = (Math.PI * 2 * i / sides) - Math.PI / 2;
        chartCtx.beginPath();
        chartCtx.moveTo(cx, cy);
        chartCtx.lineTo(cx + radius * Math.cos(angle), cy + radius * Math.sin(angle));
        chartCtx.strokeStyle = webColor;
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
    chartCtx.fillStyle = fillColor;
    chartCtx.fill();
    chartCtx.strokeStyle = strokeColor;
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
        chartCtx.fillStyle = textColor;
        chartCtx.fillText(SKILL_LABELS[i], x, y);
    }
}

export function clearRadar() {
    if (!chartCanvas || !chartCtx) return;
    chartCtx.clearRect(0, 0, chartW, chartH);
}
