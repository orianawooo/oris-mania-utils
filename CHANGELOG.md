# changelog

## 1.0.0 (2026-11-18)

First public release of ori's mania utils for Windows and Linux. Everything runs inside one Rust desktop window now, bundling MinaCalc 515, xdc1, Companella (ONNX), and tosu together.

### general
- Live in-app previews match the browser overlays and native Linux windows 1:1
- Floating always-on-top windows on Windows (click-through when locked) and native Wayland (`wlr-layer-shell`) / X11 overlays on Linux with no background browsers
- 10 color presets, map-adaptive colors pulled from beatmap covers, and JSON export/import for themes and config backups (older configs migrate cleanly)

### radar & dan
- MinaCalc 515 skills (`stream`, `jumpstream`, `handstream`, `stamina`, `jackspeed`, `chordjack`, `technical`) auto-sort from highest to lowest for each map
- Dan grading from `1st dan` to `κ` in `combined`, `xdc1`, or Companella's embedded ONNX mode without needing Python
- Fixed-height difficulty curve with a live song cursor, beatmap background, offline `.osu` file picker, and a `tosu! offline` badge when disconnected

### keystrokes
- Four presets (`dock tint`, `dock solid`, `separate`, `flat`) with hold times and updated trail defaults (`0.35` fade, `10.0` speed)
- Click any keycap in the preview to bind both the physical keycode and the right label for ANSI, ISO, ES, AZERTY, QWERTZ, and JIS keyboards
- Optional per-key colors, key shapes, hit particles, and RGB waves

### hits & setup
- Five layouts (`vertical`, `compact grid`, `horizontal`, `bar + ratio`, `minimal hud`) with optional MA ratio and stale-frame rollback protection
- Adjustable refresh interval (`8ms` to `250ms`, default `16ms`) synced to tosu's `PRECISE_DATA_POLL_RATE`
- Auto-detects `osu! Songs` (including custom `BeatmapDirectory` in `osu!.*.cfg`) plus `SERVER_IP`, `SERVER_PORT`, and `STATIC_FOLDER_PATH` in `tosu.env`
- Editable `tosu` host, port, and companion WebSocket port in `setup`, automatic port fallback, and unified `/radar/`, `/keys/`, `/hits/` browser URLs
