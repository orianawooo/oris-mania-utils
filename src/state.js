export const state = {
    socket: null,
    reconnectTimer: null,
    currentMapKey: '',
    lastCalculatedKey: '',
    calcDebounce: null,
    calcId: 0,
    waitingForCalcSince: 0,
    config: { 
        osu_songs_path: '', 
        tosu_port: 24050,
        bg_opacity: 0.85,
        accent_color: '#a67c52',
        scale: 1.0,
        show_radar: true,
        show_particles: true,
        visible_skills: {
            stream: true,
            jumpstream: true,
            handstream: true,
            stamina: true,
            jackspeed: true,
            chordjack: true,
            technical: true
        },
        keys: ["KeyD", "KeyF", "KeyJ", "KeyK"]
    },
    currentFolder: '',
    currentFile: '',
    isMapValid: true
};
