let socket = null;
let config = {
    bg_opacity: 0.85,
    scale: 1.0,
    tosu_port: 24050
};

function applyConfig(data) {
    if (data.hitcounter_opacity !== undefined) {
        config.bg_opacity = data.hitcounter_opacity;
        document.documentElement.style.setProperty('--bg-opacity', config.bg_opacity);
    }
    if (data.hitcounter_scale !== undefined) {
        config.scale = data.hitcounter_scale;
        document.documentElement.style.setProperty('--ui-scale', config.scale);
    }
    if (data.hitcounter_bg_color !== undefined) {
        document.documentElement.style.setProperty('--bg-color', data.hitcounter_bg_color);
    }
    if (data.hitcounter_text_color !== undefined) {
        document.documentElement.style.setProperty('--text-color', data.hitcounter_text_color);
    }
    if (data.hitcounter_border_style !== undefined) {
        const el = document.querySelector('.hit-counter');
        if (el) el.style.borderStyle = data.hitcounter_border_style;
    }
    if (data.hitcounter_orientation !== undefined) {
        const el = document.querySelector('.hit-counter');
        if (el) {
            if (data.hitcounter_orientation === 'horizontal') {
                el.style.flexDirection = 'row';
                el.style.width = 'auto';
            } else {
                el.style.flexDirection = 'column';
                el.style.width = '100px';
            }
        }
    }
    if (data.hitcounter_font !== undefined) {
        document.body.style.fontFamily = `"${data.hitcounter_font}", sans-serif`;
    }
}

async function loadConfig() {
    try {
        const res = await fetch('../msdconverter/config.json');
        if (res.ok) {
            const data = await res.json();
            applyConfig(data);
            if (data.tosu_port !== undefined) {
                config.tosu_port = data.tosu_port;
            }
        }
    } catch (err) {
        console.error("Error loading config:", err);
    }
}

const lastHits = { geki: -1, '300': -1, katu: -1, '100': -1, '50': -1, '0': -1 };

function connect() {
    socket = new WebSocket(`ws://127.0.0.1:${config.tosu_port}/ws`);
    
    socket.onmessage = (e) => {
        try {
            const data = JSON.parse(e.data);
            const hits = data.gameplay?.hits || {};
            
            const geki = hits.geki || 0;
            const h300 = hits['300'] || 0;
            const katu = hits.katu || 0;
            const h100 = hits['100'] || 0;
            const h50 = hits['50'] || 0;
            const h0 = hits['0'] || 0;
            
            if (geki !== lastHits.geki) {
                document.getElementById('val-max').textContent = geki;
                lastHits.geki = geki;
            }
            if (h300 !== lastHits['300']) {
                document.getElementById('val-perf').textContent = h300;
                lastHits['300'] = h300;
            }
            if (katu !== lastHits.katu) {
                document.getElementById('val-great').textContent = katu;
                lastHits.katu = katu;
            }
            if (h100 !== lastHits['100']) {
                document.getElementById('val-good').textContent = h100;
                lastHits['100'] = h100;
            }
            if (h50 !== lastHits['50']) {
                document.getElementById('val-bad').textContent = h50;
                lastHits['50'] = h50;
            }
            if (h0 !== lastHits['0']) {
                document.getElementById('val-miss').textContent = h0;
                lastHits['0'] = h0;
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
            if (data.event === "bindings") {
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
    connect();
    connectConfig();
});
