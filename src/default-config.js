export const CONFIG_VERSION = 2;

function clone(value) {
    return JSON.parse(JSON.stringify(value));
}

export function createDefaultConfig() {
    return {
        config_version: CONFIG_VERSION,
        osu_songs_path: '',
        tosu_root_path: '',
        tosu_port: 24050,
        bg_opacity: 0.85,
        accent_color: '#ff8ab3',
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
            technical: true,
        },
        keys: ['KeyD', 'KeyF', 'KeyJ', 'KeyK'],
        key_color_outer: '#00d2ff',
        key_color_inner: '#ff007f',
        key_size: 60,
        key_height: 60,
        key_gap: 10,
        key_colors: ['#00d2ff', '#ff007f', '#ff007f', '#00d2ff'],
        show_trails: true,
        trail_opacity: 0.6,
        trail_fade: 0.0,
        trail_speed: 6.0,
        trail_height: 800,
        trail_widths: [50, 50, 50, 50],
        key_labels: ['D', 'F', 'J', 'K'],
        rgb_enabled_keys: [false, false, false, false],
        rgb_speed: 1.0,
        keys_bg_color: '#2b1730',
        keys_bg_opacity: 0.7,
        keys_bg_enabled: true,
        keys_bg_offset_x: 0,
        keys_bg_offset_y: 0,
        keys_bg_width: 0,
        keys_bg_height: 0,
        keys_bg_radius: 16,
        keys_bg_padding: 15,
        keys_bg_scale: 1.0,
        keys_bg_rotation: 0,
        keys_bg_shape: 'rounded',
        bg_layer: 6,
        trail_layer: 8,
        key_layer: 10,
        particle_count: 8,
        particle_min_size: 1,
        particle_max_size: 4,
        particle_spread: 6,
        particle_speed: 8.0,
        particle_life: 1.0,
        particle_gravity: 0.2,
        particle_rgb: false,
        particle_shape: 'square',
        key_scales: [1, 1, 1, 1],
        key_rotations: [0, 0, 0, 0],
        key_shapes: ['rounded', 'rounded', 'rounded', 'rounded'],
        key_offsets_x: [0, 0, 0, 0],
        key_offsets_y: [0, 0, 0, 0],
        lock_trails: true,
        trail_scales: [1, 1, 1, 1],
        trail_rotations: [0, 0, 0, 0],
        trail_shapes: ['rounded', 'rounded', 'rounded', 'rounded'],
        trail_offsets_x: [0, 0, 0, 0],
        trail_offsets_y: [0, 0, 0, 0],
        hitcounter_opacity: 0.82,
        hitcounter_scale: 1.0,
        hitcounter_bg_color: '#0f1320',
        hitcounter_border_style: 'solid',
        hitcounter_border_color: '#262f47',
        hitcounter_orientation: 'vertical',
        hitcounter_text_color: '#8b8ba8',
        hitcounter_font: 'Inter',
        hitcounter_position_x: 0,
        hitcounter_position_y: 0,
        hitcounter_padding: 10,
        hitcounter_gap: 8,
        hitcounter_item_width: 118,
        hitcounter_item_height: 62,
        hitcounter_item_radius: 10,
        hitcounter_label_size: 10,
        hitcounter_value_size: 22,
        hitcounter_dot_size: 6,
        hitcounter_glow_strength: 0.12,
        hitcounter_labels: ['MAX', 'PERF', 'GREAT', 'GOOD', 'BAD', 'MISS'],
        hitcounter_colors: ['#ffffff', '#fbc531', '#4cd137', '#00a8ff', '#e84118', '#7f8c8d'],
        hitcounter_item_scales: [1, 1, 1, 1, 1, 1],
        hitcounter_item_offsets_x: [0, 0, 0, 0, 0, 0],
        hitcounter_item_offsets_y: [0, 70, 140, 210, 280, 350],
        hitcounter_label_offsets_x: [0, 0, 0, 0, 0, 0],
        hitcounter_label_offsets_y: [0, 0, 0, 0, 0, 0],
        hitcounter_value_offsets_x: [0, 0, 0, 0, 0, 0],
        hitcounter_value_offsets_y: [0, 0, 0, 0, 0, 0],
        hitcounter_dot_offsets_x: [0, 0, 0, 0, 0, 0],
        hitcounter_dot_offsets_y: [0, 0, 0, 0, 0, 0],
        has_run_before: false,
        debug_overlay_guides: false,
    };
}

export function mergeConfigWithDefaults(rawConfig = {}) {
    const defaults = createDefaultConfig();
    const merged = clone(defaults);

    Object.entries(rawConfig || {}).forEach(([key, value]) => {
        if (value !== undefined) {
            merged[key] = Array.isArray(value) ? [...value] : value;
        }
    });

    merged.visible_skills = {
        ...defaults.visible_skills,
        ...(rawConfig.visible_skills || {}),
    };

    for (const [key, value] of Object.entries(defaults)) {
        if (Array.isArray(value) && !Array.isArray(merged[key])) {
            merged[key] = [...value];
        }
    }

    merged.config_version = CONFIG_VERSION;
    return merged;
}

export function applyKeystrokeDefaults(config) {
    const defaults = createDefaultConfig();
    const keys = [
        'key_color_outer',
        'key_color_inner',
        'key_size',
        'key_height',
        'key_gap',
        'show_trails',
        'show_particles',
        'trail_opacity',
        'trail_fade',
        'trail_speed',
        'trail_height',
        'trail_widths',
        'key_labels',
        'rgb_enabled_keys',
        'rgb_speed',
        'keys_bg_color',
        'keys_bg_opacity',
        'keys_bg_enabled',
        'keys_bg_offset_x',
        'keys_bg_offset_y',
        'keys_bg_width',
        'keys_bg_height',
        'keys_bg_radius',
        'keys_bg_padding',
        'keys_bg_scale',
        'keys_bg_rotation',
        'keys_bg_shape',
        'bg_layer',
        'trail_layer',
        'key_layer',
        'particle_count',
        'particle_min_size',
        'particle_max_size',
        'particle_spread',
        'particle_speed',
        'particle_life',
        'particle_gravity',
        'particle_rgb',
        'particle_shape',
        'key_scales',
        'key_rotations',
        'key_shapes',
        'key_offsets_x',
        'key_offsets_y',
        'lock_trails',
        'trail_scales',
        'trail_rotations',
        'trail_shapes',
        'trail_offsets_x',
        'trail_offsets_y',
        'keys',
        'key_colors',
    ];

    for (const key of keys) {
        config[key] = Array.isArray(defaults[key]) ? [...defaults[key]] : defaults[key];
    }
}

export function applyHitCounterDefaults(config) {
    const defaults = createDefaultConfig();
    const keys = [
        'hitcounter_opacity',
        'hitcounter_scale',
        'hitcounter_bg_color',
        'hitcounter_border_style',
        'hitcounter_border_color',
        'hitcounter_orientation',
        'hitcounter_text_color',
        'hitcounter_font',
        'hitcounter_position_x',
        'hitcounter_position_y',
        'hitcounter_padding',
        'hitcounter_gap',
        'hitcounter_item_width',
        'hitcounter_item_height',
        'hitcounter_item_radius',
        'hitcounter_label_size',
        'hitcounter_value_size',
        'hitcounter_dot_size',
        'hitcounter_glow_strength',
        'hitcounter_labels',
        'hitcounter_colors',
        'hitcounter_item_scales',
        'hitcounter_item_offsets_x',
        'hitcounter_item_offsets_y',
        'hitcounter_label_offsets_x',
        'hitcounter_label_offsets_y',
        'hitcounter_value_offsets_x',
        'hitcounter_value_offsets_y',
        'hitcounter_dot_offsets_x',
        'hitcounter_dot_offsets_y',
    ];

    for (const key of keys) {
        config[key] = Array.isArray(defaults[key]) ? [...defaults[key]] : defaults[key];
    }
}
