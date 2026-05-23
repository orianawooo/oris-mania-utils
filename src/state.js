import { createDefaultConfig } from './default-config.js';

export const state = {
    socket: null,
    reconnectTimer: null,
    currentMapKey: '',
    lastCalculatedKey: '',
    calcDebounce: null,
    calcId: 0,
    waitingForCalcSince: 0,
    config: createDefaultConfig(),
    currentFolder: '',
    currentFile: '',
    isMapValid: true,
};
