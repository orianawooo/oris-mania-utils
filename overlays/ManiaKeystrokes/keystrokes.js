const canvas = document.getElementById('keystrokes-canvas');
const ctx = canvas.getContext('2d');

let canvasWidth = 300;
let height = 800;
const numKeys = 4;

let keyStates = [false, false, false, false];
let keyPressTimes = [0, 0, 0, 0];
let activeBlocks = [];
const MAX_PARTICLES = 150;
const particlesPool = [];
for (let i = 0; i < MAX_PARTICLES; i++) {
    particlesPool.push({
        active: false,
        x: 0,
        y: 0,
        vx: 0,
        vy: 0,
        radius: 0,
        color: null,
        alpha: 0,
        life: 0,
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
let speed = 6;
let rgbSpeed = 1.0;
let rgbEnabledKeys = [false, false, false, false];
let keyLabels = ["D", "F", "J", "K"];
let keyOffsetsX = [0, 0, 0, 0];
let keyOffsetsY = [0, 0, 0, 0];

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

function updateGradients() {
    columnGradients = [];
    for (let i = 0; i < numKeys; i++) {
        const grad = ctx.createLinearGradient(0, height, 0, 0);
        if (colors[i]) {
            grad.addColorStop(0, `rgba(${colors[i][0]}, ${colors[i][1]}, ${colors[i][2]}, ${trailOpacity})`);
            grad.addColorStop(1, `rgba(${colors[i][0]}, ${colors[i][1]}, ${colors[i][2]}, ${trailFade})`);
        }
        columnGradients.push(grad);
    }
}

function resizeCanvas() {
    const container = document.getElementById('keystrokes-container');
    if (container) {
        canvasWidth = container.offsetWidth;
        canvas.width = canvasWidth;
    } else {
        canvas.width = canvasWidth;
    }
    canvas.height = height;
    
    if (container) {
        const containerRect = container.getBoundingClientRect();
        for (let i = 0; i < numKeys; i++) {
            const keyEl = document.getElementById(`key-${i}`);
            if (keyEl) {
                const keyRect = keyEl.getBoundingClientRect();
                keyXPositions[i] = (keyRect.left - containerRect.left) + keyRect.width / 2;
                keyWidths[i] = keyRect.width;
            } else {
                keyXPositions[i] = i * (canvasWidth / numKeys) + (canvasWidth / numKeys) / 2;
                keyWidths[i] = canvasWidth / numKeys;
            }
        }
    }
    updateGradients();
}

window.addEventListener('resize', resizeCanvas);
resizeCanvas();

const containerEl = document.getElementById('keystrokes-container');
if (containerEl) {
    const ro = new ResizeObserver(() => {
        resizeCanvas();
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
    if (data.event === 'bindings') {
        if (data.key_labels) {
            keyLabels = data.key_labels;
        }
        for (let i = 0; i < data.keys.length; i++) {
            keyBindings[i] = data.keys[i];
            const keyEl = document.getElementById(`key-${i}`);
            if (keyEl) {
                const label = keyEl.querySelector('.key-label');
                if (label) {
                    label.textContent = (keyLabels && keyLabels[i] && keyLabels[i].trim() !== '') ? keyLabels[i] : data.keys[i].replace('Key', '');
                }
            }
        }
        
        if (data.show_particles !== undefined) {
            showParticles = data.show_particles;
        }
        if (data.show_trails !== undefined) {
            showTrails = data.show_trails;
        }
        if (data.trail_opacity !== undefined) {
            trailOpacity = data.trail_opacity;
        }
        if (data.trail_fade !== undefined) {
            trailFade = data.trail_fade;
        }
        if (data.keys_bg_opacity !== undefined) {
            document.documentElement.style.setProperty('--bg-opacity', data.keys_bg_opacity);
        }
        if (data.keys_bg_color !== undefined) {
            document.documentElement.style.setProperty('--bg-color-raw', data.keys_bg_color);
        }
        if (data.keys_font !== undefined) {
            document.body.style.fontFamily = `"${data.keys_font}", sans-serif`;
        }
        if (data.rgb_enabled_keys) {
            rgbEnabledKeys = data.rgb_enabled_keys;
        }
        if (data.rgb_speed !== undefined) {
            rgbSpeed = data.rgb_speed;
        }
        if (data.trail_widths !== undefined) {
            trailWidths = data.trail_widths;
        }
        if (data.lock_trails !== undefined) {
            lockTrails = data.lock_trails;
        }
        if (data.trail_offsets_x !== undefined) {
            trailOffsetsX = data.trail_offsets_x;
        }
        if (data.trail_speed !== undefined) {
            speed = data.trail_speed;
        }
        if (data.trail_height !== undefined && data.trail_height !== height) {
            height = data.trail_height;
        }

        if (data.key_offsets_x && data.key_offsets_y) {
            keyOffsetsX = data.key_offsets_x;
            keyOffsetsY = data.key_offsets_y;
            for (let i = 0; i < 4; i++) {
                const keyEl = document.getElementById(`key-${i}`);
                if (keyEl) {
                    keyEl.style.setProperty('--key-offset-x', `${keyOffsetsX[i]}px`);
                    keyEl.style.setProperty('--key-offset-y', `${keyOffsetsY[i]}px`);
                }
            }
        }

        const hexToRgb = (hex) => {
            const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
            return result ? [
                parseInt(result[1], 16),
                parseInt(result[2], 16),
                parseInt(result[3], 16)
            ] : null;
        };

        if (data.key_color_0) {
            for (let i = 0; i < 4; i++) {
                const keyColor = data[`key_color_${i}`];
                if (keyColor) {
                    document.documentElement.style.setProperty(`--key-color-${i}`, keyColor);
                    const rgb = hexToRgb(keyColor);
                    if (rgb) colors[i] = rgb;
                }
            }
        } else {
            if (data.key_color_outer) {
                document.documentElement.style.setProperty('--key-color-0', data.key_color_outer);
                document.documentElement.style.setProperty('--key-color-3', data.key_color_outer);
                const rgb = hexToRgb(data.key_color_outer);
                if (rgb) {
                    colors[0] = rgb;
                    colors[3] = rgb;
                }
            }
            if (data.key_color_inner) {
                document.documentElement.style.setProperty('--key-color-1', data.key_color_inner);
                document.documentElement.style.setProperty('--key-color-2', data.key_color_inner);
                const rgb = hexToRgb(data.key_color_inner);
                if (rgb) {
                    colors[1] = rgb;
                    colors[2] = rgb;
                }
            }
        }
        if (data.key_size) {
            document.documentElement.style.setProperty('--key-size', `${data.key_size}px`);
        }
        if (data.key_height) {
            document.documentElement.style.setProperty('--key-height', `${data.key_height}px`);
        }
        if (data.key_gap) {
            document.documentElement.style.setProperty('--key-gap', `${data.key_gap}px`);
        }
        
        let ticks = 0;
        const recalibrate = () => {
            resizeCanvas();
            ticks++;
            if (ticks < 10) {
                requestAnimationFrame(recalibrate);
            }
        };
        requestAnimationFrame(recalibrate);
        
        document.getElementById('debug-info').textContent = `Keys: ${keyBindings.map(k => k.replace('Key','')).join(', ')}`;
        wakeUp();
        return;
    }

    if (data.event === 'key-down') {
        document.getElementById('debug-info').textContent = `Received: ${data.key}`;
    }

    let receivedKey = data.key.replace('Key', '').toLowerCase();
    if (receivedKey === 'backquote') receivedKey = 'semicolon';

    const index = keyBindings.findIndex(k => k.replace('Key', '').toLowerCase() === receivedKey);
    if (index === -1) return;

    if (data.event === 'key-down') {
        handleKeyPress(index);
    } else if (data.event === 'key-up') {
        handleKeyRelease(index);
    }
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
        y: height,
        h: 0,
        color: colors[i],
        holding: true
    });

    if (showParticles) {
        createParticles(keyXPositions[i], height, colors[i], rgbEnabledKeys[i]);
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
    let spawned = 0;
    for (let i = 0; i < particlesPool.length; i++) {
        const p = particlesPool[i];
        if (!p.active) {
            p.active = true;
            p.x = x;
            p.y = y;
            p.vx = (Math.random() - 0.5) * 6;
            p.vy = -Math.random() * 8 - 2;
            p.radius = Math.random() * 3 + 1;
            p.color = color;
            p.alpha = 1;
            p.life = 1;
            p.isRGB = isRGB;
            spawned++;
            if (spawned >= 8) break;
        }
    }
}

function animate(currentTime) {
    if (!isAnimating) return;
    
    requestAnimationFrame(animate);

    if (!currentTime) currentTime = performance.now();
    const elapsed = currentTime - lastFrameTime;
    lastFrameTime = currentTime;
    
    const deltaFactor = elapsed / 16.66;
    const currentSpeed = speed * deltaFactor;
    
    ctx.clearRect(0, 0, canvasWidth, height);
    
    let hasActiveElements = false;

    for (let i = activeBlocks.length - 1; i >= 0; i--) {
        const b = activeBlocks[i];
        
        if (b.holding) {
            b.h += currentSpeed;
            b.y = height - b.h;
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
            let xPos = keyXPositions[b.key] - (tWidth / 2);
            
            if (!lockTrails) {
                const colW = canvasWidth / numKeys;
                xPos = (b.key * colW) + (colW / 2) - (tWidth / 2) + (trailOffsetsX[b.key] || 0);
            }
            
            if (rgbEnabledKeys[b.key]) {
                const grad = ctx.createLinearGradient(0, height, 0, 0);
                const hue = (currentTime * rgbSpeed * 0.1) % 360;
                grad.addColorStop(0, `hsla(${hue}, 100%, 50%, ${trailOpacity})`);
                grad.addColorStop(1, `hsla(${hue}, 100%, 50%, ${trailFade})`);
                ctx.fillStyle = grad;
            } else {
                ctx.fillStyle = columnGradients[b.key] || '#fff';
            }
            ctx.fillRect(xPos, b.y, tWidth, b.h);
        }
    }

    if (showParticles) {
        for (let i = 0; i < particlesPool.length; i++) {
            const p = particlesPool[i];
            if (p.active) {
                p.x += p.vx * deltaFactor;
                p.y += p.vy * deltaFactor;
                p.vy += 0.2 * deltaFactor;
                p.life -= 0.02 * deltaFactor;
                p.alpha = p.life;
                
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
                ctx.fillRect(p.x - p.radius, p.y - p.radius, size, size);
            }
        }
    }
    
    if (keyStates.some(state => state)) {
        hasActiveElements = true;
    }

    if (!hasActiveElements) {
        isAnimating = false;
        ctx.clearRect(0, 0, canvasWidth, height);
    }
}

connect();
