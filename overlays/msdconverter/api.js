import { state } from './state.js';

export function setupTosuConnection(callbacks) {
    if (typeof window.__TAURI__ !== 'undefined') {
        if (!window._eventListenersAdded) {
            window._eventListenersAdded = true;
            window.__TAURI__.event.listen('tosu-data', (event) => {
                const data = JSON.parse(event.payload);
                callbacks.onData(data);
            });
            window.__TAURI__.event.listen('tosu-status', (event) => {
                callbacks.onStatus(event.payload);
            });
        }
        
        window.__TAURI__.core.invoke('get_tosu_status').then(isConnected => {
            if (isConnected) callbacks.onStatus('connected');
            else callbacks.onStatus('disconnected');
        });
    } else {
        const ws = new WebSocket(`ws://127.0.0.1:${state.config.tosu_port || 24050}/ws`);
        ws.onopen = () => { callbacks.onStatus('connected'); };
        ws.onmessage = (e) => {
            try { callbacks.onData(JSON.parse(e.data)); } catch (err) {}
        };
        ws.onclose = () => {
            callbacks.onStatus('disconnected');
            setTimeout(() => setupTosuConnection(callbacks), 5000);
        };
        ws.onerror = () => { callbacks.onStatus('disconnected'); };
    }
}
