# Changelog - ori's mania utils

All notable changes to this project will be documented in this file.

## [0.1.1] - 2026-05-20 - Bugfixes and Cleanups
### Fixed
- Changed radar chart label colors to adjust dynamically to dark mode in both the manager app and the in-game overlay.
- Prevented WebSocket socket hanging by draining client read frames in the key Hook WebSocket server.
- Standardized the default accent color to #a67c52 across the backend config and frontend store.
- Deleted unused DOM element references and cleaned up duplicate stylesheets (`overlays/ManiaKeystrokes/styles.css`).

---

## [0.1.0] - 2026-05-20

### Added
- **MSD Calculator Overlay**: Real-time map parser and skill rating calculator (8 dimensions) with a custom radar chart.
- **ManiaKeystrokes Overlay**: 4K keystroke visualizer supporting custom colors, layout gaps, trails, and RGB speed.
- **HitCounter Overlay**: Real-time judgment counter for MAX/PERF/GREAT/GOOD/BAD/MISS.
- **Manager Application**: Tauri desktop utility to configure songs path, overlay scales, custom keybinds, and visual settings.
- **In-game Injection**: Complete support for displaying overlays inside the game client under fullscreen exclusive mode via Tosu.
