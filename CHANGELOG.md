# Changelog - ori's mania utils

All notable changes to this project will be documented in this file.

## [0.1.3] - 2026-05-20 - Cleanups and MSD Title Info
### Added
- Added map difficulty name next to the song title in the MSD converter overlay.

### Fixed
- Removed redundant hit counter grid from the MSD converter overlay to avoid overlap with the dedicated HitCounter panel.
- Fixed Mermaid diagram syntax in README.md to restore native rendering on GitHub.

---

## [0.1.2] - 2026-05-20 - WebSockets and Polling Optimizations
### Fixed
- Fixed Tosu WebSocket proxy connection stability by using unified stream polling (handles ping/pong automatically).
- Synced fallback polling timeouts to 50ms in the manager main application.

---

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
