
import { state } from './state.js';
import { readAndSaveSetupSettings } from './events.js';
import { populateSettingsPanel, saveVisualSettings } from './config.js';

let selectedKeyIndex = -1;
let selectedIsTrail = false;
let isListeningForBind = false;
let history = [];
let redoStack = [];
let isPreviewPlaying = false;
let previewAnimationFrame = null;
let previewTrailHeights = [0, 0, 0, 0];
let previewKeyStates = [false, false, false, false];
let draggedKey = null;
let draggedTrailIndex = -1;

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
            ['setup-trail-speed', 'val-trail-speed'],
            ['setup-trail-opacity', 'val-trail-opacity'],
            ['setup-trail-fade', 'val-trail-fade'],
        ];
        sliderPairs.forEach(([sliderId, labelId]) => {
            const sl = document.getElementById(sliderId);
            const lb = document.getElementById(labelId);
            if (sl && lb) {
                sl.addEventListener('input', () => {
                    lb.textContent = sl.value;
                    renderPreview();
                });
            }
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

        document.getElementById('setup-lock-trails')?.addEventListener('change', (e) => {
            state.config.lock_trails = e.target.checked;
            if (!state.config.lock_trails) {
                state.config.trail_offsets_x = [...(state.config.key_offsets_x || [0,0,0,0])];
                state.config.trail_offsets_y = [...(state.config.key_offsets_y || [0,0,0,0])];
            }
            populateSettingsPanel();
            saveVisualSettings();
            renderPreview();
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
            if (selectedKeyIndex === -1) return;
            saveHistory();
            if (selectedIsTrail) {
                if (!state.config.trail_offsets_x) state.config.trail_offsets_x = [0,0,0,0];
                if (!state.config.trail_offsets_y) state.config.trail_offsets_y = [0,0,0,0];
                state.config.trail_offsets_x[selectedKeyIndex] = 0;
                state.config.trail_offsets_y[selectedKeyIndex] = 0;
                
                document.getElementById('ctx-x').value = 0;
                document.getElementById('ctx-y').value = 0;
            } else {
                if (!state.config.key_offsets_x) state.config.key_offsets_x = [0,0,0,0];
                if (!state.config.key_offsets_y) state.config.key_offsets_y = [0,0,0,0];
                state.config.key_offsets_x[selectedKeyIndex] = 0;
                state.config.key_offsets_y[selectedKeyIndex] = 0;
                
                document.getElementById('ctx-x').value = 0;
                document.getElementById('ctx-y').value = 0;
            }
            populateSettingsPanel();
            saveVisualSettings();
            renderPreview();
        });

        window.addEventListener('resize', () => {
            renderPreview();
        });

        if (isTauri) {
            window.__TAURI__.event.listen('bind-key', (event) => {
                if (!isListeningForBind || selectedKeyIndex === -1) return;
                const keyCode = event.payload;
                
                if (!state.config.keys) state.config.keys = ['KeyD', 'KeyF', 'KeyJ', 'KeyK'];
                
                const existingIndex = state.config.keys.indexOf(keyCode);
                if (existingIndex !== -1 && existingIndex !== selectedKeyIndex) {
                    state.config.keys[existingIndex] = state.config.keys[selectedKeyIndex];
                }
                state.config.keys[selectedKeyIndex] = keyCode;
                
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
        document.addEventListener('keydown', (e) => {
            if (isTauri) return;
            if (!isListeningForBind || selectedKeyIndex === -1) return;
            e.preventDefault();
            const keyCode = e.code;
            if (!state.config.keys) state.config.keys = ['KeyD', 'KeyF', 'KeyJ', 'KeyK'];
            
            const existingIndex = state.config.keys.indexOf(keyCode);
            if (existingIndex !== -1 && existingIndex !== selectedKeyIndex) {
                state.config.keys[existingIndex] = state.config.keys[selectedKeyIndex];
            }
            state.config.keys[selectedKeyIndex] = keyCode;
            const ctxBind = document.getElementById('ctx-bind');
            if (ctxBind) {
                ctxBind.value = keyCode.replace('Key', '');
                ctxBind.style.borderColor = '';
            }
            isListeningForBind = false;
            populateSettingsPanel();
            saveVisualSettings();
        });
        ['ctx-label', 'ctx-x', 'ctx-y', 'ctx-width', 'ctx-rgb'].forEach(id => {
            const el = document.getElementById(id);
            if (el) el.addEventListener('change', () => {
                if (selectedKeyIndex === -1) return;
                applyContextToState(selectedKeyIndex);
                saveVisualSettings();
                renderPreview();
            });
        });
        document.getElementById('ctx-x')?.addEventListener('input', () => {
            if (selectedKeyIndex === -1) return;
            applyContextToState(selectedKeyIndex);
            renderPreview();
        });
        document.getElementById('ctx-y')?.addEventListener('input', () => {
            if (selectedKeyIndex === -1) return;
            applyContextToState(selectedKeyIndex);
            renderPreview();
        });
        syncColorBadges();
        renderPreview();

    } catch (e) {
        console.error('Visual editor init failed:', e);
    }
}

function saveHistory() {
    history.push(JSON.stringify({
        key_offsets_x: [...(state.config.key_offsets_x || [0,0,0,0])],
        key_offsets_y: [...(state.config.key_offsets_y || [0,0,0,0])],
        key_colors: [...(state.config.key_colors || ['#00d2ff','#ff007f','#ff007f','#00d2ff'])],
    }));
    if (history.length > 20) history.shift();
    redoStack = [];
}

function undo() {
    if (!history.length) return;
    const curr = JSON.stringify({
        key_offsets_x: [...(state.config.key_offsets_x || [0,0,0,0])],
        key_offsets_y: [...(state.config.key_offsets_y || [0,0,0,0])],
        key_colors: [...(state.config.key_colors || ['#00d2ff','#ff007f','#ff007f','#00d2ff'])],
    });
    redoStack.push(curr);
    const snap = JSON.parse(history.pop());
    state.config.key_offsets_x = snap.key_offsets_x;
    state.config.key_offsets_y = snap.key_offsets_y;
    state.config.key_colors = snap.key_colors;
    populateSettingsPanel(); saveVisualSettings();
    renderPreview();
}

function redo() {
    if (!redoStack.length) return;
    const curr = JSON.stringify({
        key_offsets_x: [...(state.config.key_offsets_x || [0,0,0,0])],
        key_offsets_y: [...(state.config.key_offsets_y || [0,0,0,0])],
        key_colors: [...(state.config.key_colors || ['#00d2ff','#ff007f','#ff007f','#00d2ff'])],
    });
    history.push(curr);
    const snap = JSON.parse(redoStack.pop());
    state.config.key_offsets_x = snap.key_offsets_x;
    state.config.key_offsets_y = snap.key_offsets_y;
    state.config.key_colors = snap.key_colors;
    populateSettingsPanel(); saveVisualSettings();
    renderPreview();
}

function exportLayout() {
    const layout = {
        key_offsets_x: state.config.key_offsets_x,
        key_offsets_y: state.config.key_offsets_y,
        key_labels: state.config.key_labels,
        key_colors: state.config.key_colors,
        trail_widths: state.config.trail_widths,
        rgb_enabled_keys: state.config.rgb_enabled_keys,
        key_size: state.config.key_size,
        key_gap: state.config.key_gap,
        keys: state.config.keys,
    };
    const a = document.createElement('a');
    a.href = 'data:text/json;charset=utf-8,' + encodeURIComponent(JSON.stringify(layout, null, 2));
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
            if (data.key_offsets_x) state.config.key_offsets_x = data.key_offsets_x;
            if (data.key_offsets_y) state.config.key_offsets_y = data.key_offsets_y;
            if (data.key_labels) state.config.key_labels = data.key_labels;
            if (data.key_colors) state.config.key_colors = data.key_colors;
            if (data.trail_widths) state.config.trail_widths = data.trail_widths;
            if (data.rgb_enabled_keys) state.config.rgb_enabled_keys = data.rgb_enabled_keys;
            if (data.key_size) state.config.key_size = data.key_size;
            if (data.key_gap) state.config.key_gap = data.key_gap;
            if (data.keys) state.config.keys = data.keys;
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
        runPreviewAnimation();
    } else {
        cancelAnimationFrame(previewAnimationFrame);
        previewTrailHeights = [0, 0, 0, 0];
        previewKeyStates = [false, false, false, false];
        renderPreview();
    }
}

function runPreviewAnimation() {
    if (!isPreviewPlaying) return;
    const speed = parseFloat(document.getElementById('setup-trail-speed')?.value || 6) * 2;
    const maxH = parseFloat(document.getElementById('setup-trail-height-num')?.value || 800);
    const time = Date.now();

    for (let i = 0; i < 4; i++) {
        previewKeyStates[i] = Math.sin(time * 0.0045 + i * 1.5) > 0.25;
        
        if (previewKeyStates[i]) {
            previewTrailHeights[i] = Math.min(maxH, previewTrailHeights[i] + speed);
        } else {
            previewTrailHeights[i] = Math.max(0, previewTrailHeights[i] - speed * 0.6);
        }
    }
    renderPreview();
    previewAnimationFrame = requestAnimationFrame(runPreviewAnimation);
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
    const canvas = document.getElementById('editor-canvas');
    const ws = document.getElementById('editor-workspace');
    if (!canvas || !ws) return;

    const W = ws.clientWidth || 400;
    const H = ws.clientHeight || 350;
    canvas.width = W;
    canvas.height = H;
    const ctx = canvas.getContext('2d');
    const bgColor = document.getElementById('setup-bg-color')?.value || '#0a0a12';
    const bgOpacity = parseFloat(document.getElementById('setup-bg-opacity')?.value || 0.85);
    ctx.fillStyle = '#1a1a2e';
    ctx.fillRect(0, 0, W, H);

    const size = parseInt(document.getElementById('setup-key-size')?.value || 60);
    const keyH = parseInt(document.getElementById('setup-key-height')?.value || size);
    const gap = parseInt(document.getElementById('setup-key-gap')?.value || 10);
    const colors = getKeyColors();
    const labels = state.config.key_labels || ['D','F','J','K'];
    const offX = state.config.key_offsets_x || [0,0,0,0];
    const offY = state.config.key_offsets_y || [0,0,0,0];
    const trailWidths = state.config.trail_widths || [50,50,50,50];
    const trailOpacity = parseFloat(document.getElementById('setup-trail-opacity')?.value || 0.6);
    const trailFade = parseFloat(document.getElementById('setup-trail-fade')?.value || 0);
    const showTrails = document.getElementById('setup-trails')?.checked ?? true;

    const totalW = size * 4 + gap * 3;
    const startX = (W - totalW) / 2;
    const baseY = H - keyH - 20;
    if (showTrails && state.config.lock_trails === false) {
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
    if (showTrails) {
        const isLocked = state.config.lock_trails !== false; 
        const tOffX = state.config.trail_offsets_x || [0,0,0,0];
        const tOffY = state.config.trail_offsets_y || [0,0,0,0];

        for (let i = 0; i < 4; i++) {
            const trailH = previewTrailHeights[i] || 0;
            const tw = trailWidths[i] || size;
            let trailX, trailY;

            if (isLocked) {
                const currentKeyX = startX + i * (size + gap) + offX[i];
                trailX = currentKeyX + (size - tw) / 2;
                trailY = baseY + offY[i];
            } else {
                const baseKeyX = startX + i * (size + gap);
                trailX = baseKeyX + (size - tw) / 2 + tOffX[i];
                trailY = baseY + tOffY[i];
            }
            const isSelectedTrail = selectedKeyIndex === i && selectedIsTrail === true;
            if (isSelectedTrail) {
                ctx.save();
                ctx.strokeStyle = colors[i];
                ctx.lineWidth = 1.5;
                ctx.setLineDash([4, 4]);
                ctx.strokeRect(trailX, 0, tw, trailY);
                
                ctx.fillStyle = colors[i];
                ctx.font = '600 10px Inter, sans-serif';
                ctx.textAlign = 'center';
                ctx.fillText('Selected Trail', trailX + tw / 2, Math.max(15, trailY - 15));
                ctx.restore();
            }

            if (trailH >= 2) {
                const rgb = hexToRgb(colors[i]);
                const grad = ctx.createLinearGradient(0, trailY, 0, trailY - trailH);
                grad.addColorStop(0, `rgba(${rgb[0]},${rgb[1]},${rgb[2]},${trailOpacity})`);
                grad.addColorStop(1, `rgba(${rgb[0]},${rgb[1]},${rgb[2]},${trailFade})`);
                ctx.fillStyle = grad;
                ctx.fillRect(trailX, trailY - trailH, tw, trailH);
            }
        }
    }
    const padX = 15, padY = 10;
    const panelX = startX - padX;
    const panelY = baseY - padY;
    const panelW = totalW + padX * 2;
    const panelH = keyH + padY * 2;
    const rgb = hexToRgb(bgColor);
    ctx.save();
    ctx.globalAlpha = bgOpacity;
    ctx.fillStyle = `rgb(${rgb[0]},${rgb[1]},${rgb[2]})`;
    roundRect(ctx, panelX, panelY, panelW, panelH, 16);
    ctx.fill();
    ctx.restore();
    ctx.strokeStyle = 'rgba(255,255,255,0.1)';
    ctx.lineWidth = 1;
    roundRect(ctx, panelX, panelY, panelW, panelH, 16);
    ctx.stroke();
    const time = Date.now();
    for (let i = 0; i < 4; i++) {
        const x = startX + i * (size + gap) + offX[i];
        const y = baseY + offY[i];
        
        let col = colors[i];
        let rgb = hexToRgb(col);
        if (state.config.rgb_enabled_keys?.[i]) {
            const hue = (time * (state.config.rgb_speed || 1.0) * 0.08) % 360;
            rgb = hslToRgb(hue, 100, 50);
            col = rgbToHex(rgb[0], rgb[1], rgb[2]);
        }
        
        const isSelected = selectedKeyIndex === i && selectedIsTrail === false;
        const isActive = isPreviewPlaying && previewKeyStates[i];

        if (isActive) {
            ctx.save();
            ctx.fillStyle = `rgba(${rgb[0]}, ${rgb[1]}, ${rgb[2]}, 0.35)`;
            ctx.strokeStyle = col;
            ctx.lineWidth = 2;
            ctx.shadowColor = col;
            ctx.shadowBlur = 15;
            
            const cx = x + size / 2;
            const cy = y + keyH / 2;
            ctx.translate(cx, cy);
            ctx.scale(0.96, 0.96);
            ctx.translate(-cx, -cy);
            
            roundRect(ctx, x, y, size, keyH, 12);
            ctx.fill();
            ctx.stroke();
            ctx.restore();
        } else {
            ctx.save();
            ctx.fillStyle = 'rgba(255,255,255,0.05)';
            ctx.strokeStyle = isSelected ? col : 'rgba(255,255,255,0.1)';
            ctx.lineWidth = isSelected ? 2 : 1;
            roundRect(ctx, x, y, size, keyH, 12);
            ctx.fill();
            ctx.stroke();
            ctx.restore();
        }
        if (isSelected) {
            ctx.save();
            ctx.shadowColor = col;
            ctx.shadowBlur = 12;
            ctx.strokeStyle = col;
            ctx.lineWidth = 2;
            roundRect(ctx, x, y, size, keyH, 12);
            ctx.stroke();
            ctx.restore();
        }
        ctx.fillStyle = 'rgba(255,255,255,0.6)';
        ctx.font = `700 ${Math.round(size * 0.28)}px Inter, sans-serif`;
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText(labels[i] || ['D','F','J','K'][i], x + size / 2, y + keyH / 2 - 6);
        ctx.beginPath();
        ctx.arc(x + size / 2, y + keyH - 10, 3, 0, Math.PI * 2);
        ctx.fillStyle = col;
        ctx.fill();
        if (isSelected) {
            ctx.fillStyle = col;
            ctx.font = `600 10px Inter, sans-serif`;
            ctx.textAlign = 'center';
            ctx.fillText('Selected', x + size / 2, y - 8);
        }
    }
    if (draggedKey) {
        const i = draggedKey.index;
        const tooltip = document.getElementById('editor-tooltip');
        if (tooltip) {
            tooltip.style.display = 'block';
            tooltip.textContent = `X: ${offX[i] >= 0 ? '+' : ''}${offX[i]}, Y: ${offY[i] >= 0 ? '+' : ''}${offY[i]}`;
        }
    } else {
        const tooltip = document.getElementById('editor-tooltip');
        if (tooltip) tooltip.style.display = 'none';
    }
}

function roundRect(ctx, x, y, w, h, r) {
    ctx.beginPath();
    ctx.moveTo(x + r, y);
    ctx.lineTo(x + w - r, y);
    ctx.quadraticCurveTo(x + w, y, x + w, y + r);
    ctx.lineTo(x + w, y + h - r);
    ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
    ctx.lineTo(x + r, y + h);
    ctx.quadraticCurveTo(x, y + h, x, y + h - r);
    ctx.lineTo(x, y + r);
    ctx.quadraticCurveTo(x, y, x + r, y);
    ctx.closePath();
}

function getKeyAtPoint(canvasX, canvasY) {
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
    const totalW = size * 4 + gap * 3;
    const startX = (W - totalW) / 2;
    const baseY = H - keyH - 20;

    for (let i = 0; i < 4; i++) {
        const x = startX + i * (size + gap) + offX[i];
        const y = baseY + offY[i];
        if (canvasX >= x && canvasX <= x + size && canvasY >= y && canvasY <= y + keyH) {
            return i;
        }
    }
    return -1;
}

function onCanvasMouseDown(e) {
    if (e.button !== 0) return;
    const rect = e.target.getBoundingClientRect();
    const cx = e.clientX - rect.left;
    const cy = e.clientY - rect.top;
    
    const idx = getKeyAtPoint(cx, cy);
    if (idx !== -1) {
        selectedKeyIndex = idx;
        selectedIsTrail = false;
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
    
    if (state.config.lock_trails === false) {
        const trailIdx = getTrailAtPoint(cx, cy);
        if (trailIdx !== -1) {
            selectedKeyIndex = trailIdx;
            selectedIsTrail = true;
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
            renderPreview();
            return;
        }
    }
    
    selectedKeyIndex = -1;
    selectedIsTrail = false;
    document.getElementById('context-menu').style.display = 'none';
    renderPreview();
}

function onCanvasRightClick(e) {
    e.preventDefault();
    const rect = e.target.getBoundingClientRect();
    const cx = e.clientX - rect.left;
    const cy = e.clientY - rect.top;
    
    const idx = getKeyAtPoint(cx, cy);
    if (idx !== -1) {
        selectedKeyIndex = idx;
        selectedIsTrail = false;
        renderPreview();
        openContextMenu(e.clientX, e.clientY, idx, false);
        return;
    }
    
    const trailIdx = getTrailAtPoint(cx, cy);
    if (trailIdx !== -1) {
        selectedKeyIndex = trailIdx;
        selectedIsTrail = true;
        renderPreview();
        openContextMenu(e.clientX, e.clientY, trailIdx, true);
        return;
    }
}

function onMouseMove(e) {
    if (!draggedKey) return;
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
    if (draggedKey) {
        populateSettingsPanel();
        saveVisualSettings();
        draggedKey = null;
        renderPreview();
    }
}

function openContextMenu(x, y, i, isTrail = false) {
    const menu = document.getElementById('context-menu');
    menu.style.display = 'block';
    menu.style.left = Math.min(x, window.innerWidth - 240) + 'px';
    menu.style.top = Math.min(y, window.innerHeight - 280) + 'px';

    const header = menu.querySelector('.context-header');
    if (header) {
        header.innerHTML = isTrail 
            ? `Trail <span id="ctx-index">${i + 1}</span> Settings` 
            : `Key <span id="ctx-index">${i + 1}</span> Settings`;
    }

    const bindRow = document.getElementById('ctx-bind')?.closest('.ctx-row');
    const labelRow = document.getElementById('ctx-label')?.closest('.ctx-row');
    if (bindRow) bindRow.style.display = isTrail ? 'none' : 'flex';
    if (labelRow) labelRow.style.display = isTrail ? 'none' : 'flex';

    if (isTrail) {
        document.getElementById('ctx-x').value = state.config.trail_offsets_x?.[i] || 0;
        document.getElementById('ctx-y').value = state.config.trail_offsets_y?.[i] || 0;
    } else {
        document.getElementById('ctx-x').value = state.config.key_offsets_x?.[i] || 0;
        document.getElementById('ctx-y').value = state.config.key_offsets_y?.[i] || 0;
    }

    document.getElementById('ctx-width').value = state.config.trail_widths?.[i] || 50;
    document.getElementById('ctx-rgb').checked = state.config.rgb_enabled_keys?.[i] || false;

    isListeningForBind = false;
}

function applyContextToState(i) {
    if (!state.config.key_labels) state.config.key_labels = ['D','F','J','K'];
    if (!state.config.key_offsets_x) state.config.key_offsets_x = [0,0,0,0];
    if (!state.config.key_offsets_y) state.config.key_offsets_y = [0,0,0,0];
    if (!state.config.trail_offsets_x) state.config.trail_offsets_x = [0,0,0,0];
    if (!state.config.trail_offsets_y) state.config.trail_offsets_y = [0,0,0,0];
    if (!state.config.trail_widths) state.config.trail_widths = [50,50,50,50];
    if (!state.config.rgb_enabled_keys) state.config.rgb_enabled_keys = [false,false,false,false];

    if (selectedIsTrail) {
        state.config.trail_offsets_x[i] = parseInt(document.getElementById('ctx-x')?.value) || 0;
        state.config.trail_offsets_y[i] = parseInt(document.getElementById('ctx-y')?.value) || 0;
    } else {
        state.config.key_labels[i] = document.getElementById('ctx-label')?.value || '';
        state.config.key_offsets_x[i] = parseInt(document.getElementById('ctx-x')?.value) || 0;
        state.config.key_offsets_y[i] = parseInt(document.getElementById('ctx-y')?.value) || 0;
    }
    state.config.trail_widths[i] = parseInt(document.getElementById('ctx-width')?.value) || 50;
    state.config.rgb_enabled_keys[i] = document.getElementById('ctx-rgb')?.checked || false;
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
    if (badge) badge.style.backgroundColor = col;
    if (item) item.style.borderColor = col;
    if (!state.config.key_colors) state.config.key_colors = ['#00d2ff','#ff007f','#ff007f','#00d2ff'];
    state.config.key_colors[i] = col;
}

