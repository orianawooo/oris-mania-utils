# ori's mania utils

discord: `orianawoo`

Hey, I'm Ori. I made this for osu!mania 4K players on Windows and Linux who want keystrokes, an MSD/Dan radar, and a hit counter in one clean app. It connects to [tosu](https://github.com/tosuapp/tosu) and runs Etterna MinaCalc 515, Daniel (`xdc1`), and Companella locally in Rust.

My main goal was keeping things light. No Python install to manage, and no background browser processes eating memory while you play on Linux. Every change saves automatically too, so updates or quick restarts won't wipe your layout.

## what's inside

### keystrokes

Global 4K input viewer with rising gradient trails, hold times in milliseconds, and optional hit particles. You get four styles (`dock tint`, `dock solid`, `separate`, and `flat`), plus controls for trail opacity, fade, and speed.

Open the advanced section when you want custom hex colors per key, different shapes, or RGB waves. To rebind a key, click any cap in the live preview and press what you use. It captures both your physical keycode and the right label from your OS layout (`ANSI`, `ISO`, `ES`, `AZERTY`, `QWERTZ`, or `JIS`).

### msd radar & dan tiers

Calculates MinaCalc 515 scores as you browse songs (`stream`, `jumpstream`, `handstream`, `stamina`, `jackspeed`, `chordjack`, and `technical`). Skill rows sort themselves from highest to lowest for each beatmap. Don't need all seven? Uncheck whichever ones you want to hide.

Right under the radar is the Dan badge (`1st dan` through `κ`). Switch freely between `combined`, `xdc1`, or `companella` mode, which runs via an embedded ONNX model so nothing extra needs to be installed. Below that is a fixed-height difficulty curve with a live playback cursor over the map background. You can also hit `open .osu file` anytime to inspect charts offline.

### hit counter

Shows live judgments (`max`, `300`, `200`, `100`, `50`, and `miss`) alongside an optional MA ratio. Pick from five layouts (`vertical`, `compact grid`, `horizontal`, `bar + ratio`, and `minimal hud`), all matched 1:1 with the in-app preview.

You can adjust how often counts refresh too. It defaults to `16ms` for real-time updates, with `50ms` and `120ms` presets (or any interval from `8ms` to `250ms`) synced straight to `tosu` for weaker hardware.

### themes & backups

Includes ten palettes and a `link all overlays` toggle when you want everything matching at once. Turn on adaptive theming if you'd rather pull accent and background tones straight from the current song cover. Themes and full config backups export or import as JSON whenever you need them.

## quick start

Grab and launch `oris-mania-utils.exe` on Windows or `oris-mania-utils` on Linux. On first run it looks for your `osu! Songs` folder (including custom `BeatmapDirectory` paths inside `osu!.*.cfg`) and any existing `tosu` install. Still need `tosu`? Open the **setup** tab and click `install tosu` to fetch the latest release automatically.

### windows

Browser hosting turns on automatically and sets `ENABLE_INGAME_OVERLAY` and `ENABLE_KEY_OVERLAY` inside `tosu.env`. Launch `tosu`, open osu!, and hit `Ctrl + Shift + Space` in-game to move or resize things even in exclusive fullscreen.

Play borderless instead? Toggle **float window** on any tab for standalone transparent windows without hooks. Drag them where you want while unlocked, then check **lock overlays** so clicks pass straight through.

### linux

Turn on **float window** per tab (or inside **setup**) to spawn native transparent windows via `zwlr_layer_shell_v1` on Wayland or `override_redirect` on X11. Move them into position, then toggle **lock overlays** for click-through input. Browser hosting stays off by default here to save RAM, though you can flip it on whenever you stream with OBS.

### obs studio & custom ports

Make sure **enable browser overlays** is checked in **setup**, click **sync overlays** if needed, and add three Browser Sources:

- `http://127.0.0.1:24050/radar/`
- `http://127.0.0.1:24050/keys/`
- `http://127.0.0.1:24050/hits/`

Custom `SERVER_IP`, `SERVER_PORT`, and `STATIC_FOLDER_PATH` entries in `tosu.env` work out of the box. Running custom ports? Change `host`, `tosu port`, or `companion ws port` inside **setup** (or pass `?tosuHost=...&tosuPort=...&companionPort=...` in your OBS URL). When the companion WebSocket port is busy, it picks a free port nearby and points the browser sources there automatically.

## building from source

You need [Rust](https://rustup.rs) (2021 edition). Linux builds require X11 dev headers (`libx11-dev` and `libxtst-dev` on Debian or Ubuntu). Cross-compiling the Windows build from Linux uses `mingw-w64` (`x86_64-w64-mingw32-gcc` and `x86_64-w64-mingw32-windres`), with `libstdc++` linked statically in `build.rs` and icon resources handled on both MinGW and MSVC.

```bash
git clone https://github.com/orianawooo/oris-mania-utils.git
cd oris-mania-utils

# lint and run unit tests
cargo clippy --all-targets -- -D warnings
cargo test

# linux release binary
cargo build --release

# windows release binary from linux
cargo build --target x86_64-pc-windows-gnu --release
```

## license

Licensed under the [MIT license](LICENSE).
