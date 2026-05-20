# ori's mania utils

A cozy suite of overlays for **osu!mania 4K**, built on top of [tosu](https://github.com/tosuapp/tosu).

- **MSD Calculator** — real-time Etterna skill ratings (Stream, Jumpstream, Stamina...) with radar chart.
- **Keystroke Overlay** — 4K key visualizer with custom trails, RGB mode, and per-key colors.
- **Hit Counter** — live judgment counter (MAX / PERF / GREAT / GOOD / BAD / MISS).

All overlays inject directly into osu! via tosu's Chromium layer — works in fullscreen exclusive mode.

---

## Features

- **MSD Calculation**: Calculates 8 skill ratings using the Etterna algorithm (`etterna-rs` under the hood) on map change.
- **Tosu Integration**: Automatically detects map changes and captures play data (hits, keys) in real time.
- **In-game Injection**: Works natively in fullscreen exclusive mode through Tosu.
- **Customization**: Configure accent colors, scale, key bindings, and trails directly in the Manager app.

---

## Architecture

```
oris-mania-utils.exe  (Tauri desktop app — Manager)
├── Reads .osu files from your osu! Songs folder
├── Computes MSD ratings via etterna-rs (Rust)
├── Writes results to msd.json
└── Proxies tosu WebSocket data to trigger recalculation on map change

Tosu overlays  (static HTML/CSS/JS — served by tosu at port 24050)
├── msdconverter/   → skill ratings + radar chart
├── ManiaKeystrokes/  → 4K key press visualization with trails
└── HitCounter/     → real-time judgment breakdown
```

---

## Skill ratings

| Skill | What it measures |
|---|---|
| Overall | Composite difficulty |
| Stream | Density and speed of single-note streams |
| Jumpstream | Alternating two-hand stream patterns |
| Handstream | Sustained three-finger stream density |
| Stamina | Long-term endurance under pressure |
| JackSpeed | Same-column rapid repetition |
| Chordjack | Chord-to-chord jack patterns |
| Technical | Complex mixed and off-rhythm patterns |

---

## Requirements

- **[Tosu](https://github.com/tosuapp/tosu/releases/latest) (v4.20.0+)** — running in the background.
- **osu!mania 4K** game mode.

---

## Setup & Game Integration

### 1. Extract to Tosu
Download the latest release and extract the three folders directly inside tosu's `static/` directory:
```
tosu/
└── static/
    ├── msdconverter/     <-- Contains oris-mania-utils.exe (Manager)
    ├── ManiaKeystrokes/
    └── HitCounter/
```

### 2. Run the Manager
1. Open **`oris-mania-utils.exe`** inside `static/msdconverter/`.
2. Select your **osu! Songs folder** path and click **Save**.
3. Keep it running (you can minimize it). It tracks map changes and writes ratings to `msd.json` automatically.

### 3. Game Integration & Configuration

#### In-game Overlay Setup
1. **Enable In-Game Overlay in Tosu**: Open your browser at [http://127.0.0.1:24050/settings](http://127.0.0.1:24050/settings) and verify that the **In-Game Overlay** toggle is turned **ON**.
2. **Open the menu inside osu!**: Launch the game (make sure Tosu is running and hooked). Press the overlay manager shortcut (default is **Ctrl + Shift + Space** or **Ctrl + Shift + ~**).
3. **Add the overlays**: Right-click anywhere on the screen inside osu! to open Tosu's overlay management menu. Select **Add Overlay** and add the folders one by one:
   - `msdconverter`
   - `ManiaKeystrokes`
   - `HitCounter`
4. **Arrange**: Drag and resize the overlay boxes to fit your UI, then close the menu.

#### OBS Studio & Browser Sources
To stream the overlays or view them in a browser, add them as **Browser Sources** in OBS (do NOT check "Local file"):
* **MSD Radar**: `http://localhost:24050/msdconverter/` (350x220)
* **Keystrokes**: `http://localhost:24050/ManiaKeystrokes/` (270x400)
* **Hit Counter**: `http://localhost:24050/HitCounter/` (240x180)

---

## Building from source

```bash
# Install dependencies
npm install

# Development build
npm run tauri dev

# Production build
npm run tauri build
```

**Requires:** [Rust (stable)](https://rustup.rs) · [Node.js 18+](https://nodejs.org) · [tosu](https://github.com/tosuapp/tosu) running

---

## Stack

| Layer | Technology |
|---|---|
| Desktop app | [Tauri v2](https://tauri.app) (Rust + WebView) |
| MSD calculation | [etterna-rs](https://github.com/kangalioo/etterna-rs) |
| Overlays | Vanilla HTML / CSS / JS (ES Modules) |
| Real-time data | tosu WebSocket API |
| Charts | Canvas API (custom radar renderer) |

---

## Changelog

See [CHANGELOG.md](./CHANGELOG.md).
