# Changelog - ori's mania utils

All notable changes to this project will be documented in this file.

## [0.3.0] - 2026-05-23 - Standalone Manager, Update Flow & Stability Pass
### Added
- Added a real standalone manager flow: the app can bootstrap from outside `tosu/static/msdconverter`, detect TOSU, and install selected overlays directly into `tosu/static/`.
- Added GitHub release checks plus one-click app/overlay update actions inside the manager.
- Added shared geometry/layout modules for `ManiaKeystrokes` and `HitCounter` so preview and runtime stop drifting apart.
- Added release tooling and smoke checks for the MSI + portable ZIP flow.

### Changed
- Consolidated defaults, persisted config handling, and config versioning so the manager, editors, and runtime overlays stop fighting each other.
- Reworked the release path around the embedded overlay resources and a single `build -> smoke -> zip` flow.
- Tightened overlay install/update status reporting in the manager UI so missing or outdated overlays are obvious before editing.

### Fixed
- Fixed the old requirement of launching the app from inside the `msdconverter` folder just to make config and overlay actions work.
- Fixed live update wiring so `msdconverter`, `HitCounter`, and `ManiaKeystrokes` all react to config changes through the same runtime channel again.
- Fixed preview/runtime mismatch in keystrokes and hit counter by sharing the same layout math instead of duplicating it in multiple places.
- Fixed a bunch of wasted work in config saves, overlay redraws, and Songs reindexing that was adding unnecessary overhead during normal use.

## [0.2.1] - 2026-05-21 - Emergency Editor Hotfixes
### Fixed
- Fixed ReferenceError crash by defining isTauri and invoke in editor.js.
- Implemented getTrailAtPoint to restore clicking and dragging unlocked trails.
- Populated current key configs (labels, binds) when opening context menus to prevent state erasure.

## [0.2.0] - 2026-05-21 - Visual Editor, Overlay Revamp & Latency Polish
### Added
- Redesigned visual editor using a dark-themed HTML5 Canvas matching the in-game overlay look.
- Support for individual key color pickers, trail offsets, and drag-and-drop key/trail custom positions.
- Guide lines and selection bounding boxes in the visual editor for unlocked trails.
- Dynamic color rainbow cycle preview on keys and trails when RGB mode is enabled.
- "Reset Offset" button in context menu to easily snap keys/trails back to default.
- Support for exporting/importing complete layouts including key bindings.
- Lock-free, zero-allocation gameplay keyhook using atomic static variables (`AtomicU64`) — bypassing all mutex read locks and lookup overhead.
- Raw index-based key messages (`{"event":"key-down","index":X}`) — removing string allocations, JSON serialization, and javascript search loops in hot key event pathways.
- Support for Spanish and special keyboard layouts (like Ñ, commas, brackets) using Rust hook keybind listener fallback.

### Fixed
- Fixed window drag bug: dragging only triggers from the header, preventing workspace interaction from dragging the Tauri window.
- Fixed trail alignment: trails now start dynamically from the key's height offset in-game, avoiding static background spawning.
- Fixed key deform/shrink in narrow overlays by setting `flex-shrink: 0` in CSS.
- Added full dark mode support to the context menu interface and input controls.
- Fixed WebSocket memory leak: old sockets are closed explicitly during reconnect loops.
- Fixed disappearing radar chart: automatically re-calibrates canvas transform matrices and redraws from ratings cache on window resize.
- Fixed trail height growth: capped active trail height and added block queue limits to prevent memory growth.
- Fixed editor trail updates: dragging trails now updates the context menu coordinate inputs immediately.
- Swapped duplicate key binds automatically inside the editor layout manager.

---

## [0.1.5] - 2026-05-20 - Deep Performance Optimization Pass
### Fixed
- Wrapped MSD calculation in `tokio::task::spawn_blocking` — CPU-heavy `.osu` parsing no longer blocks the async WebSocket event loop.
- Added O(1) Songs directory index (`HashMap`) — eliminates full-disk scans on folder name cache misses.
- Cached app config in-memory (`RwLock`) — removed hot-path disk reads from every map calculation and WebSocket client connection.
- Pre-built `HashSet<rdev::Key>` for the global keyboard hook — gameplay key filtering now exits before any string allocation or lock acquisition for non-gameplay keys.
- Cached DOM element references in ManiaKeystrokes — eliminates repeated `getElementById`/`querySelector` calls during high-speed streams and jacks.
- Cached canvas dimensions in radar chart — removed `getBoundingClientRect()` from the chart draw loop to prevent layout reflows on every redraw.

---

## [0.1.4] - 2026-05-20 - Performance Optimizations
### Fixed
- Fixed in-game performance drops and micro-stutters by caching MSD calculations using beatmap MD5 hashes.
- Stopped redundant calculation loops when remaining static on a map or while playing.
- Reduced key-hook server traffic by filtering events to gameplay keys only, eliminating IPC/WebSocket overhead.
- Implemented a self-healing trail height cap in ManiaKeystrokes to prevent memory/CPU growth on missed keyup events.
- Throttled Tosu WebSocket proxy data emissions to 200ms and paused processing when minimized.

---

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
