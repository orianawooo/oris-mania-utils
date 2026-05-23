import { state } from './state.js';
import { createDefaultConfig, mergeConfigWithDefaults } from './default-config.js';
import { recordPerf } from './perf.js';

const isTauri = typeof window.__TAURI__ !== 'undefined';
const invoke = isTauri ? window.__TAURI__.core.invoke : async () => {};

let lastSavedSerialized = '';
let saveTimer = null;
let pendingSaveResolvers = [];
let pendingReason = 'ui';

function serializeConfig(config) {
    return JSON.stringify(config);
}

function resolvePendingSave(error = null) {
    const resolvers = [...pendingSaveResolvers];
    pendingSaveResolvers = [];
    for (const { resolve, reject } of resolvers) {
        if (error) reject(error);
        else resolve();
    }
}

export function applyConfigSnapshot(rawConfig) {
    state.config = mergeConfigWithDefaults(rawConfig);
    return state.config;
}

export function setPersistedConfigBaseline(rawConfig) {
    const normalized = mergeConfigWithDefaults(rawConfig);
    lastSavedSerialized = serializeConfig(normalized);
    state.config = normalized;
    return normalized;
}

export async function loadDefaultConfig() {
    if (isTauri) {
        const config = await invoke('get_default_config');
        return mergeConfigWithDefaults(config);
    }
    return createDefaultConfig();
}

async function persistNow(reason = 'ui') {
    if (!isTauri) return;

    const normalized = mergeConfigWithDefaults(state.config);
    const serialized = serializeConfig(normalized);
    if (serialized === lastSavedSerialized) {
        return;
    }

    const startedAt = performance.now();
    await invoke('save_config', { config: normalized });
    lastSavedSerialized = serialized;
    state.config = normalized;
    recordPerf('save_config', performance.now() - startedAt, { reason });
}

export function saveConfig({ immediate = false, debounceMs = 120, reason = 'ui' } = {}) {
    pendingReason = reason;

    if (saveTimer) {
        clearTimeout(saveTimer);
        saveTimer = null;
    }

    const promise = new Promise((resolve, reject) => {
        pendingSaveResolvers.push({ resolve, reject });
    });

    const run = async () => {
        try {
            await persistNow(pendingReason);
            resolvePendingSave();
        } catch (error) {
            resolvePendingSave(error);
        }
    };

    if (immediate || !isTauri) {
        void run();
        return promise;
    }

    saveTimer = window.setTimeout(() => {
        saveTimer = null;
        void run();
    }, debounceMs);

    return promise;
}
