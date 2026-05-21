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
        keys: ["KeyD", "KeyF", "KeyJ", "KeyK"],
        key_size: 60,
        key_height: 60,
        key_gap: 10,
        key_colors: ["#00d2ff", "#ff007f", "#ff007f", "#00d2ff"],
        lock_trails: true,
        trail_offsets_x: [0, 0, 0, 0],
        trail_offsets_y: [0, 0, 0, 0]
    },
    currentFolder: '',
    currentFile: '',
    isMapValid: true
};
