use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::io::AsFd;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use wayland_client::protocol::{
    wl_buffer, wl_callback, wl_compositor, wl_pointer, wl_region, wl_registry, wl_seat, wl_shm,
    wl_shm_pool, wl_surface,
};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols::wp::relative_pointer::zv1::client::{
    zwp_relative_pointer_manager_v1, zwp_relative_pointer_v1,
};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

use x11rb::connection::Connection as X11Connection;
use x11rb::protocol::shape::{ConnectionExt as ShapeExt, SK, SO};
use x11rb::protocol::xproto::{
    AtomEnum, ClipOrdering, ColormapAlloc, ConfigureWindowAux, ConnectionExt as XProtoExt,
    CreateGCAux, CreateWindowAux, EventMask, ImageFormat, StackMode, VisualClass, WindowClass,
};
use x11rb::protocol::Event as X11Event;

use crate::calc::SkillRatings;
use crate::config::{read_config, CONFIG_REV};
use crate::keys::{LIVE_KEY_HOLD_MS, LIVE_KEY_STATES};
use crate::overlay_draw::{
    hex_rgb, keys_trail_area_h, render_hits_bgra_live, render_keys_bgra_live,
    render_radar_bgra_live, TrailBlock,
};
#[cfg(test)]
use crate::overlay_draw::{render_hits_bgra, render_keys_bgra, render_radar_bgra};

#[derive(Clone)]
pub struct OverlaySharedState {
    pub float_keys: bool,
    pub float_radar: bool,
    pub float_hits: bool,
    pub title: String,
    pub artist: String,
    pub diff: String,
    pub ratings: SkillRatings,
    pub hit_counts: [u32; 6],
    pub dragged_pos: [Option<(i32, i32)>; 3],
}

impl Default for OverlaySharedState {
    fn default() -> Self {
        Self {
            float_keys: false,
            float_radar: false,
            float_hits: false,
            title: "Frontier Explorer".to_string(),
            artist: "kiwi_iwata".to_string(),
            diff: "Another".to_string(),
            ratings: SkillRatings {
                overall: 26.50,
                stream: 28.20,
                jumpstream: 25.10,
                handstream: 23.40,
                stamina: 26.90,
                jackspeed: 22.80,
                chordjack: 21.50,
                technical: 24.30,
                map_type: "Stream Map".to_string(),
                dan: crate::dan::DanResult {
                    level: 13.42,
                    tier: 13,
                    symbol: "γ".to_string(),
                    name: "gamma".to_string(),
                    variant: "".to_string(),
                    sub_label: "mid".to_string(),
                    display: "γ · gamma mid".to_string(),
                    xdc1_dan: 13.55,
                    xdc1_points: 34.20,
                    xdc1_rice_points: 33.80,
                    companella_dan: 13.21,
                    interlude: 7.42,
                    sunny: 8.15,
                    strain_curve: vec![
                        0.25, 0.35, 0.55, 0.48, 0.72, 0.88, 0.64, 0.42, 0.68, 0.94, 0.78, 0.52,
                        0.82, 0.60, 0.34,
                    ],
                    duration_ms: 120000,
                },
                bg_path: String::new(),
                adaptive_accent: String::new(),
                adaptive_accent_secondary: String::new(),
                adaptive_bg: String::new(),
                adaptive_item_bg: String::new(),
                adaptive_border: String::new(),
            },
            hit_counts: [482, 94, 18, 5, 1, 3],
            dragged_pos: [None, None, None],
        }
    }
}

lazy_static::lazy_static! {
    pub static ref OVERLAY_SHARED: Mutex<OverlaySharedState> = Mutex::new(OverlaySharedState::default());
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OverlayKind {
    Keys = 0,
    Radar = 1,
    Hits = 2,
}

pub fn start_linux_overlay_thread() {
    std::thread::spawn(|| {
        if try_wayland_loop().is_err() {
            let _ = try_x11_loop();
        }
    });
}

struct TrailSim {
    blocks: Vec<TrailBlock>,
    prev_pressed: [bool; 4],
    last_tick: Instant,
}

impl TrailSim {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            prev_pressed: [false; 4],
            last_tick: Instant::now(),
        }
    }

    fn step(&mut self, cfg: &crate::config::Config, pressed: &[bool; 4]) -> bool {
        let now = Instant::now();
        let dt = now
            .duration_since(self.last_tick)
            .as_secs_f32()
            .clamp(0.001, 0.05);
        self.last_tick = now;
        let speed = (cfg.trail_speed * 60.0 * dt).max(0.5);
        let outer = hex_rgb(&cfg.key_color_outer, [0xe5, 0x88, 0xa6]);
        let inner = hex_rgb(&cfg.key_color_inner, [0xd4, 0x76, 0x95]);

        let mut active = false;
        for (i, &down) in pressed.iter().enumerate() {
            if down && !self.prev_pressed[i] && cfg.show_trails {
                self.blocks.push(TrailBlock {
                    key: i,
                    y: 0.0,
                    h: 2.0,
                    holding: true,
                    color: if i == 0 || i == 3 { outer } else { inner },
                });
                active = true;
            } else if !down && self.prev_pressed[i] {
                for b in &mut self.blocks {
                    if b.key == i && b.holding {
                        b.holding = false;
                    }
                }
                active = true;
            }
            self.prev_pressed[i] = down;
        }

        let max_h = (cfg.trail_height as f32).clamp(20.0, 3400.0);
        for b in &mut self.blocks {
            if b.holding {
                b.h = (b.h + speed).min(max_h);
            } else {
                b.y += speed;
            }
            active = true;
        }
        self.blocks.retain(|b| b.y < max_h + 24.0);
        active || pressed.iter().any(|&p| p)
    }
}

struct ShmSlot {
    file: File,
    pool: wl_shm_pool::WlShmPool,
    buffer: Option<wl_buffer::WlBuffer>,
    cap: usize,
    w: u32,
    h: u32,
}

fn create_anon_shm_file(size: usize) -> Result<File, String> {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/dev/shm"));
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let path = base.join(format!("oris-shm-{}-{}.bin", pid, nanos));
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    let _ = fs::remove_file(&path);
    file.set_len(size as u64).map_err(|e| e.to_string())?;
    Ok(file)
}

struct DragState {
    kind: OverlayKind,
    start_x: i32,
    start_y: i32,
    accum_dx: f64,
    accum_dy: f64,
    anchor_lx: f64,
    anchor_ly: f64,
    last_apply: Instant,
}

struct WaylandSurfaceEntry {
    kind: OverlayKind,
    wl_surface: wl_surface::WlSurface,
    layer_surface: zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
    configured: bool,
    shm: Option<ShmSlot>,
    last_locked: Option<bool>,
    last_pos: (i32, i32),
    last_size: (u32, u32),
    surface_offset_y: i32,
    drag_wait_until: Option<Instant>,
}

struct WaylandApp {
    globals: HashSet<String>,
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    seat: Option<wl_seat::WlSeat>,
    pointer: Option<wl_pointer::WlPointer>,
    rel_mgr: Option<zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1>,
    rel_pointer: Option<zwp_relative_pointer_v1::ZwpRelativePointerV1>,
    surfaces: Vec<WaylandSurfaceEntry>,
    hovered_kind: Option<OverlayKind>,
    dragging: Option<DragState>,
    last_pointer_xy: (f64, f64),
}

impl WaylandApp {
    fn new() -> Self {
        Self {
            globals: HashSet::new(),
            compositor: None,
            shm: None,
            layer_shell: None,
            seat: None,
            pointer: None,
            rel_mgr: None,
            rel_pointer: None,
            surfaces: Vec::new(),
            hovered_kind: None,
            dragging: None,
            last_pointer_xy: (0.0, 0.0),
        }
    }

    fn ensure_relative_pointer(&mut self, qh: &QueueHandle<Self>) {
        if self.rel_pointer.is_none() {
            if let (Some(mgr), Some(ptr)) = (&self.rel_mgr, &self.pointer) {
                self.rel_pointer = Some(mgr.get_relative_pointer(ptr, qh, ()));
            }
        }
    }
}

fn apply_wl_input_region(
    comp: &wl_compositor::WlCompositor,
    qh: &QueueHandle<WaylandApp>,
    entry: &WaylandSurfaceEntry,
    locked: bool,
) {
    if locked {
        let region = comp.create_region(qh, ());
        entry.wl_surface.set_input_region(Some(&region));
        region.destroy();
    } else if entry.kind == OverlayKind::Keys && entry.surface_offset_y > 0 {
        let region = comp.create_region(qh, ());
        let fh = (entry.last_size.1 as i32 - entry.surface_offset_y).max(24);
        region.add(0, entry.surface_offset_y, entry.last_size.0 as i32, fh);
        entry.wl_surface.set_input_region(Some(&region));
        region.destroy();
    } else {
        entry.wl_surface.set_input_region(None);
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandApp {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            state.globals.insert(interface.clone());
            match interface.as_str() {
                "wl_compositor" => {
                    state.compositor = Some(registry.bind(name, version.min(4), qh, ()));
                }
                "wl_shm" => {
                    state.shm = Some(registry.bind(name, version.min(1), qh, ()));
                }
                "zwlr_layer_shell_v1" => {
                    state.layer_shell = Some(registry.bind(name, version.min(4), qh, ()));
                }
                "wl_seat" => {
                    state.seat = Some(registry.bind(name, version.min(5), qh, ()));
                }
                "zwp_relative_pointer_manager_v1" => {
                    state.rel_mgr = Some(registry.bind(name, version.min(1), qh, ()));
                    state.ensure_relative_pointer(qh);
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for WaylandApp {
    fn event(
        _: &mut Self,
        _: &wl_compositor::WlCompositor,
        _: wl_compositor::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_shm::WlShm, ()> for WaylandApp {
    fn event(
        _: &mut Self,
        _: &wl_shm::WlShm,
        _: wl_shm::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_shm_pool::WlShmPool, ()> for WaylandApp {
    fn event(
        _: &mut Self,
        _: &wl_shm_pool::WlShmPool,
        _: wl_shm_pool::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_buffer::WlBuffer, ()> for WaylandApp {
    fn event(
        _: &mut Self,
        _: &wl_buffer::WlBuffer,
        _: wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_region::WlRegion, ()> for WaylandApp {
    fn event(
        _: &mut Self,
        _: &wl_region::WlRegion,
        _: wl_region::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for WaylandApp {
    fn event(
        _: &mut Self,
        _: &wl_surface::WlSurface,
        _: wl_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_callback::WlCallback, OverlayKind> for WaylandApp {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        kind: &OverlayKind,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            if let Some(entry) = state.surfaces.iter_mut().find(|s| s.kind == *kind) {
                entry.drag_wait_until = None;
            }
        }
    }
}

impl Dispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, ()> for WaylandApp {
    fn event(
        _: &mut Self,
        _: &zwlr_layer_shell_v1::ZwlrLayerShellV1,
        _: zwlr_layer_shell_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, OverlayKind> for WaylandApp {
    fn event(
        state: &mut Self,
        proxy: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        data: &OverlayKind,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure { serial, .. } => {
                proxy.ack_configure(serial);
                if let Some(entry) = state.surfaces.iter_mut().find(|s| s.kind == *data) {
                    entry.configured = true;
                }
            }
            zwlr_layer_surface_v1::Event::Closed => {
                state.surfaces.retain(|s| s.kind != *data);
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for WaylandApp {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(caps),
        } = event
        {
            if caps.contains(wl_seat::Capability::Pointer) && state.pointer.is_none() {
                state.pointer = Some(seat.get_pointer(qh, ()));
                state.ensure_relative_pointer(qh);
            }
        }
    }
}

impl Dispatch<zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1, ()> for WaylandApp {
    fn event(
        _: &mut Self,
        _: &zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1,
        _: zwp_relative_pointer_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwp_relative_pointer_v1::ZwpRelativePointerV1, ()> for WaylandApp {
    fn event(
        state: &mut Self,
        _: &zwp_relative_pointer_v1::ZwpRelativePointerV1,
        event: zwp_relative_pointer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwp_relative_pointer_v1::Event::RelativeMotion { dx, dy, .. } = event {
            if let Some(d) = state.dragging.as_mut() {
                d.accum_dx += dx;
                d.accum_dy += dy;
                let nx = (d.start_x + d.accum_dx.round() as i32).max(-600);
                let ny = (d.start_y + d.accum_dy.round() as i32).max(-600);
                if let Some(entry) = state.surfaces.iter_mut().find(|s| s.kind == d.kind) {
                    if entry.last_pos != (nx, ny) {
                        entry.last_pos = (nx, ny);
                        entry
                            .layer_surface
                            .set_margin(ny - entry.surface_offset_y, 0, 0, nx);
                        entry.wl_surface.commit();
                        if let Ok(mut shared) = OVERLAY_SHARED.lock() {
                            shared.dragged_pos[d.kind as usize] = Some((nx, ny));
                        }
                    }
                }
            }
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for WaylandApp {
    fn event(
        state: &mut Self,
        _: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter {
                surface,
                surface_x,
                surface_y,
                ..
            } => {
                state.last_pointer_xy = (surface_x, surface_y);
                state.hovered_kind = state
                    .surfaces
                    .iter()
                    .find(|s| s.wl_surface == surface)
                    .map(|s| s.kind);
            }
            wl_pointer::Event::Leave { .. } => {
                if state.dragging.is_none() {
                    state.hovered_kind = None;
                }
            }
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                if state.rel_pointer.is_some() {
                    state.last_pointer_xy = (surface_x, surface_y);
                    return;
                }
                if let Some(d) = state.dragging.as_mut() {
                    let dx = (surface_x - d.anchor_lx).round() as i32;
                    let dy = (surface_y - d.anchor_ly).round() as i32;
                    if (dx != 0 || dy != 0) && d.last_apply.elapsed() >= Duration::from_millis(24) {
                        if let Some(entry) = state.surfaces.iter_mut().find(|s| s.kind == d.kind) {
                            let nx = (entry.last_pos.0 + dx).max(-600);
                            let ny = (entry.last_pos.1 + dy).max(-600);
                            entry.last_pos = (nx, ny);
                            d.last_apply = Instant::now();
                            entry
                                .layer_surface
                                .set_margin(ny - entry.surface_offset_y, 0, 0, nx);
                            entry.wl_surface.commit();
                            if let Ok(mut shared) = OVERLAY_SHARED.lock() {
                                shared.dragged_pos[d.kind as usize] = Some((nx, ny));
                            }
                        }
                    }
                } else {
                    state.last_pointer_xy = (surface_x, surface_y);
                }
            }
            wl_pointer::Event::Button {
                button: 0x110,
                state: WEnum::Value(btn_state),
                ..
            } => match btn_state {
                wl_pointer::ButtonState::Pressed => {
                    if let Some(kind) = state.hovered_kind {
                        if let Some(entry) = state.surfaces.iter().find(|s| s.kind == kind) {
                            state.dragging = Some(DragState {
                                kind,
                                start_x: entry.last_pos.0,
                                start_y: entry.last_pos.1,
                                accum_dx: 0.0,
                                accum_dy: 0.0,
                                anchor_lx: state.last_pointer_xy.0,
                                anchor_ly: state.last_pointer_xy.1,
                                last_apply: Instant::now() - Duration::from_secs(1),
                            });
                        }
                    }
                }
                wl_pointer::ButtonState::Released => {
                    state.dragging = None;
                }
                _ => {}
            },
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_wl_surface(
    app: &mut WaylandApp,
    qh: &QueueHandle<WaylandApp>,
    kind: OverlayKind,
    x: i32,
    y: i32,
    surface_offset_y: i32,
    w: u32,
    h: u32,
    locked: bool,
) -> Option<()> {
    let comp = app.compositor.as_ref()?;
    let shell = app.layer_shell.as_ref()?;
    let wl_surface = comp.create_surface(qh, ());
    let ns = match kind {
        OverlayKind::Keys => "oris-keys",
        OverlayKind::Radar => "oris-radar",
        OverlayKind::Hits => "oris-hits",
    };
    let layer_surface = shell.get_layer_surface(
        &wl_surface,
        None,
        zwlr_layer_shell_v1::Layer::Overlay,
        ns.to_string(),
        qh,
        kind,
    );

    layer_surface
        .set_anchor(zwlr_layer_surface_v1::Anchor::Top | zwlr_layer_surface_v1::Anchor::Left);
    layer_surface.set_margin(y - surface_offset_y, 0, 0, x);
    layer_surface.set_exclusive_zone(-1);
    layer_surface.set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::None);
    layer_surface.set_size(w, h);

    let entry = WaylandSurfaceEntry {
        kind,
        wl_surface,
        layer_surface,
        configured: false,
        shm: None,
        last_locked: Some(locked),
        last_pos: (x, y),
        last_size: (w, h),
        surface_offset_y,
        drag_wait_until: None,
    };
    apply_wl_input_region(comp, qh, &entry, locked);
    entry.wl_surface.commit();

    app.surfaces.push(entry);
    Some(())
}

fn update_wl_buffer(
    entry: &mut WaylandSurfaceEntry,
    shm_global: &wl_shm::WlShm,
    qh: &QueueHandle<WaylandApp>,
    w: u32,
    h: u32,
    bgra: &[u8],
) {
    if !entry.configured || bgra.is_empty() {
        return;
    }
    let needed = (w * h * 4) as usize;
    if entry.last_size != (w, h) {
        entry.last_size = (w, h);
        entry.layer_surface.set_size(w, h);
    }

    let recreate_pool = match &entry.shm {
        Some(slot) => slot.cap < needed,
        None => true,
    };

    if recreate_pool {
        if let Some(old) = entry.shm.take() {
            if let Some(buf) = old.buffer {
                buf.destroy();
            }
            old.pool.destroy();
        }
        let alloc = needed.max(65536);
        if let Ok(file) = create_anon_shm_file(alloc) {
            let pool = shm_global.create_pool(file.as_fd(), alloc as i32, qh, ());
            entry.shm = Some(ShmSlot {
                file,
                pool,
                buffer: None,
                cap: alloc,
                w: 0,
                h: 0,
            });
        }
    }

    if let Some(slot) = &mut entry.shm {
        if slot.file.seek(SeekFrom::Start(0)).is_ok() && slot.file.write_all(bgra).is_ok() {
            let _ = slot.file.flush();
            if slot.buffer.is_none() || slot.w != w || slot.h != h {
                if let Some(old_buf) = slot.buffer.take() {
                    old_buf.destroy();
                }
                let buf = slot.pool.create_buffer(
                    0,
                    w as i32,
                    h as i32,
                    (w * 4) as i32,
                    wl_shm::Format::Argb8888,
                    qh,
                    (),
                );
                slot.buffer = Some(buf);
                slot.w = w;
                slot.h = h;
            }
            if let Some(buf) = &slot.buffer {
                entry.wl_surface.attach(Some(buf), 0, 0);
                entry.wl_surface.damage_buffer(0, 0, w as i32, h as i32);
                entry.wl_surface.commit();
            }
        }
    }
}

fn try_wayland_loop() -> Result<(), String> {
    if std::env::var("WAYLAND_DISPLAY")
        .map(|v| v.is_empty())
        .unwrap_or(true)
    {
        return Err("no wayland display".to_string());
    }
    let conn = Connection::connect_to_env().map_err(|e| e.to_string())?;
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let display = conn.display();
    let _registry = display.get_registry(&qh, ());

    let mut app = WaylandApp::new();
    event_queue.roundtrip(&mut app).map_err(|e| e.to_string())?;

    if app.compositor.is_none() || app.shm.is_none() || app.layer_shell.is_none() {
        return Err("wlr-layer-shell unsupported".to_string());
    }
    let _ = event_queue.roundtrip(&mut app);

    let mut trails = TrailSim::new();
    let mut last_hashes: [u64; 3] = [0; 3];
    let mut radar_cache: Option<(u64, u32, u32, Vec<u8>)> = None;
    let mut hits_cache: Option<(u64, u32, u32, Vec<u8>)> = None;
    let mut keys_cache: Option<(u64, u32, u32, Vec<u8>)> = None;
    let mut last_cfg_rev: u64 = 0;
    let mut cfg = read_config();
    let started = Instant::now();

    loop {
        let _ = event_queue.dispatch_pending(&mut app);
        let _ = conn.flush();

        let (shared, pressed, hold_ms) = snapshot_state(&mut last_cfg_rev, &mut cfg);
        let trail_active = trails.step(&cfg, &pressed);
        let rgb_active = cfg.rgb_enabled_keys.iter().any(|&v| v) && pressed.iter().any(|&v| v);
        let phase_secs = started.elapsed().as_secs_f32();
        let song_time_ms = crate::proxy::LIVE_SONG_TIME_MS.load(Ordering::Relaxed);

        let wanted = wanted_overlays(&shared, &cfg);

        for (kind, show, px, py) in wanted {
            let exists = app.surfaces.iter().any(|s| s.kind == kind);
            if !show && exists {
                if let Some(pos) = app.surfaces.iter().position(|s| s.kind == kind) {
                    let entry = app.surfaces.remove(pos);
                    if let Some(slot) = entry.shm {
                        if let Some(b) = slot.buffer {
                            b.destroy();
                        }
                        slot.pool.destroy();
                    }
                    entry.layer_surface.destroy();
                    entry.wl_surface.destroy();
                }
                last_hashes[kind as usize] = 0;
                continue;
            }
            if !show {
                continue;
            }

            let (w, h, bgra): (u32, u32, &[u8]) = match kind {
                OverlayKind::Keys => {
                    let tosu_connected = crate::proxy::TOSU_CONNECTED.load(Ordering::Relaxed);
                    let in_key =
                        keys_input_key(&cfg, &pressed, &hold_ms, &shared.ratings, tosu_connected);
                    let need_render = trail_active
                        || rgb_active
                        || keys_cache.as_ref().map(|c| c.0 != in_key).unwrap_or(true);
                    if need_render {
                        let (rw, rh, rbuf) = render_keys_bgra_live(
                            &cfg,
                            &pressed,
                            &hold_ms,
                            &trails.blocks,
                            phase_secs,
                            Some(&shared.ratings),
                            tosu_connected,
                        );
                        keys_cache = Some((in_key, rw, rh, rbuf));
                    }
                    let c = keys_cache.as_ref().unwrap();
                    (c.1, c.2, &c.3)
                }
                OverlayKind::Radar => {
                    let tosu_connected = crate::proxy::TOSU_CONNECTED.load(Ordering::Relaxed);
                    let time_bucket = if cfg.radar_show_strain_graph
                        && cfg.radar_graph_show_cursor
                        && song_time_ms > 0
                    {
                        song_time_ms / 250
                    } else {
                        0
                    };
                    let in_key = radar_input_key(&shared, time_bucket, tosu_connected);
                    if radar_cache.as_ref().map(|c| c.0 != in_key).unwrap_or(true) {
                        let (rw, rh, rbuf) = render_radar_bgra_live(
                            &cfg,
                            &shared.title,
                            &shared.artist,
                            &shared.diff,
                            &shared.ratings,
                            song_time_ms,
                            tosu_connected,
                        );
                        radar_cache = Some((in_key, rw, rh, rbuf));
                    }
                    let c = radar_cache.as_ref().unwrap();
                    (c.1, c.2, &c.3)
                }
                OverlayKind::Hits => {
                    let tosu_connected = crate::proxy::TOSU_CONNECTED.load(Ordering::Relaxed);
                    let in_key =
                        hits_input_key(&cfg, &shared.hit_counts, &shared.ratings, tosu_connected);
                    if hits_cache.as_ref().map(|c| c.0 != in_key).unwrap_or(true) {
                        let (hw, hh, hbuf) = render_hits_bgra_live(
                            &cfg,
                            &shared.hit_counts,
                            Some(&shared.ratings),
                            tosu_connected,
                        );
                        hits_cache = Some((in_key, hw, hh, hbuf));
                    }
                    let c = hits_cache.as_ref().unwrap();
                    (c.1, c.2, &c.3)
                }
            };
            let surf_offset_y = if kind == OverlayKind::Keys {
                keys_trail_area_h(&cfg).round() as i32
            } else {
                0
            };

            if !exists {
                spawn_wl_surface(
                    &mut app,
                    &qh,
                    kind,
                    px,
                    py,
                    surf_offset_y,
                    w,
                    h,
                    cfg.lock_overlays,
                );
                let _ = event_queue.roundtrip(&mut app);
            }

            if let Some(entry) = app.surfaces.iter_mut().find(|s| s.kind == kind) {
                let offset_changed = entry.surface_offset_y != surf_offset_y;
                if offset_changed {
                    entry.surface_offset_y = surf_offset_y;
                }
                if entry.last_locked != Some(cfg.lock_overlays)
                    || offset_changed
                    || entry.last_size != (w, h)
                {
                    if let Some(comp) = &app.compositor {
                        apply_wl_input_region(comp, &qh, entry, cfg.lock_overlays);
                        entry.wl_surface.commit();
                    }
                    entry.last_locked = Some(cfg.lock_overlays);
                }

                let is_dragging_this = app.dragging.as_ref().map(|d| d.kind) == Some(kind);
                if (!is_dragging_this && entry.last_pos != (px, py)) || offset_changed {
                    if !is_dragging_this {
                        entry.last_pos = (px, py);
                    }
                    entry.layer_surface.set_margin(
                        entry.last_pos.1 - entry.surface_offset_y,
                        0,
                        0,
                        entry.last_pos.0,
                    );
                    entry.wl_surface.commit();
                }

                let hash = hash_bytes(bgra);
                if entry.configured
                    && (last_hashes[kind as usize] != hash || trail_active || rgb_active)
                {
                    last_hashes[kind as usize] = hash;
                    if let Some(shm) = &app.shm {
                        update_wl_buffer(entry, shm, &qh, w, h, bgra);
                    }
                }
            }
        }

        let _ = conn.flush();
        if let Some(guard) = conn.prepare_read() {
            let _ = guard.read();
        }
        let sleep_ms = if app.surfaces.is_empty() {
            100
        } else if trail_active || rgb_active {
            16
        } else {
            cfg.hitcounter_poll_ms.clamp(8, 33) as u64
        };
        wait_overlay_tick(sleep_ms);
    }
}

struct X11SurfaceEntry {
    kind: OverlayKind,
    win: u32,
    gc: u32,
    w: u32,
    h: u32,
    last_locked: Option<bool>,
    last_pos: (i32, i32),
    surface_offset_y: i32,
}

fn find_argb_visual(screen: &x11rb::protocol::xproto::Screen) -> Option<(u8, u32)> {
    for depth_info in &screen.allowed_depths {
        if depth_info.depth == 32 {
            for vis in &depth_info.visuals {
                if vis.class == VisualClass::TRUE_COLOR {
                    return Some((32, vis.visual_id));
                }
            }
        }
    }
    None
}

fn set_x11_click_through(conn: &impl X11Connection, win: u32, locked: bool) {
    if locked {
        let _ = conn.shape_rectangles(SO::SET, SK::INPUT, ClipOrdering::UNSORTED, win, 0, 0, &[]);
    } else {
        let _ = conn.shape_mask(SO::SET, SK::INPUT, win, 0, 0, x11rb::NONE);
    }
}

fn try_x11_loop() -> Result<(), String> {
    let (conn, screen_num) = x11rb::connect(None).map_err(|e| e.to_string())?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;
    let (depth, visual_id) = find_argb_visual(screen).ok_or("no 32-bit argb visual")?;
    let cmap = conn.generate_id().map_err(|e| e.to_string())?;
    conn.create_colormap(ColormapAlloc::NONE, cmap, root, visual_id)
        .map_err(|e| e.to_string())?;

    let mut entries: Vec<X11SurfaceEntry> = Vec::new();
    let mut trails = TrailSim::new();
    let mut last_hashes: [u64; 3] = [0; 3];
    let mut radar_cache: Option<(u64, u32, u32, Vec<u8>)> = None;
    let mut hits_cache: Option<(u64, u32, u32, Vec<u8>)> = None;
    let mut keys_cache: Option<(u64, u32, u32, Vec<u8>)> = None;
    let mut dragging: Option<(OverlayKind, i16, i16, i32, i32)> = None;
    let mut tick_count: u32 = 0;
    let mut last_cfg_rev: u64 = 0;
    let mut cfg = read_config();
    let started = Instant::now();

    loop {
        while let Ok(Some(ev)) = conn.poll_for_event() {
            match ev {
                X11Event::ButtonPress(bp) if bp.detail == 1 => {
                    if let Some(entry) = entries.iter().find(|e| e.win == bp.event) {
                        dragging = Some((
                            entry.kind,
                            bp.root_x,
                            bp.root_y,
                            entry.last_pos.0,
                            entry.last_pos.1,
                        ));
                    }
                }
                X11Event::MotionNotify(mn) => {
                    if let Some((kind, rx0, ry0, ox, oy)) = dragging {
                        let nx = (ox + (mn.root_x - rx0) as i32).max(-600);
                        let ny = (oy + (mn.root_y - ry0) as i32).max(-600);
                        if let Some(entry) = entries.iter_mut().find(|e| e.kind == kind) {
                            entry.last_pos = (nx, ny);
                            let sy = (ny - entry.surface_offset_y)
                                .clamp(i16::MIN as i32, i16::MAX as i32);
                            let sx = nx.clamp(i16::MIN as i32, i16::MAX as i32);
                            let _ = conn.configure_window(
                                entry.win,
                                &ConfigureWindowAux::new()
                                    .x(sx)
                                    .y(sy)
                                    .stack_mode(StackMode::ABOVE),
                            );
                            if let Ok(mut shared) = OVERLAY_SHARED.lock() {
                                shared.dragged_pos[kind as usize] = Some((nx, ny));
                            }
                        }
                    }
                }
                X11Event::ButtonRelease(br) if br.detail == 1 => {
                    dragging = None;
                }
                _ => {}
            }
        }

        let (shared, pressed, hold_ms) = snapshot_state(&mut last_cfg_rev, &mut cfg);
        let trail_active = trails.step(&cfg, &pressed);
        let rgb_active = cfg.rgb_enabled_keys.iter().any(|&v| v) && pressed.iter().any(|&v| v);
        let phase_secs = started.elapsed().as_secs_f32();
        let song_time_ms = crate::proxy::LIVE_SONG_TIME_MS.load(Ordering::Relaxed);

        let wanted = wanted_overlays(&shared, &cfg);

        for (kind, show, px, py) in wanted {
            let exists = entries.iter().any(|e| e.kind == kind);
            if !show && exists {
                if let Some(pos) = entries.iter().position(|e| e.kind == kind) {
                    let e = entries.remove(pos);
                    let _ = conn.free_gc(e.gc);
                    let _ = conn.destroy_window(e.win);
                }
                last_hashes[kind as usize] = 0;
                continue;
            }
            if !show {
                continue;
            }

            let (w, h, bgra): (u32, u32, &[u8]) = match kind {
                OverlayKind::Keys => {
                    let tosu_connected = crate::proxy::TOSU_CONNECTED.load(Ordering::Relaxed);
                    let in_key =
                        keys_input_key(&cfg, &pressed, &hold_ms, &shared.ratings, tosu_connected);
                    let need_render = trail_active
                        || rgb_active
                        || keys_cache.as_ref().map(|c| c.0 != in_key).unwrap_or(true);
                    if need_render {
                        let (rw, rh, rbuf) = render_keys_bgra_live(
                            &cfg,
                            &pressed,
                            &hold_ms,
                            &trails.blocks,
                            phase_secs,
                            Some(&shared.ratings),
                            tosu_connected,
                        );
                        keys_cache = Some((in_key, rw, rh, rbuf));
                    }
                    let c = keys_cache.as_ref().unwrap();
                    (c.1, c.2, &c.3)
                }
                OverlayKind::Radar => {
                    let tosu_connected = crate::proxy::TOSU_CONNECTED.load(Ordering::Relaxed);
                    let time_bucket = if cfg.radar_show_strain_graph
                        && cfg.radar_graph_show_cursor
                        && song_time_ms > 0
                    {
                        song_time_ms / 250
                    } else {
                        0
                    };
                    let in_key = radar_input_key(&shared, time_bucket, tosu_connected);
                    if radar_cache.as_ref().map(|c| c.0 != in_key).unwrap_or(true) {
                        let (rw, rh, rbuf) = render_radar_bgra_live(
                            &cfg,
                            &shared.title,
                            &shared.artist,
                            &shared.diff,
                            &shared.ratings,
                            song_time_ms,
                            tosu_connected,
                        );
                        radar_cache = Some((in_key, rw, rh, rbuf));
                    }
                    let c = radar_cache.as_ref().unwrap();
                    (c.1, c.2, &c.3)
                }
                OverlayKind::Hits => {
                    let tosu_connected = crate::proxy::TOSU_CONNECTED.load(Ordering::Relaxed);
                    let in_key =
                        hits_input_key(&cfg, &shared.hit_counts, &shared.ratings, tosu_connected);
                    if hits_cache.as_ref().map(|c| c.0 != in_key).unwrap_or(true) {
                        let (hw, hh, hbuf) = render_hits_bgra_live(
                            &cfg,
                            &shared.hit_counts,
                            Some(&shared.ratings),
                            tosu_connected,
                        );
                        hits_cache = Some((in_key, hw, hh, hbuf));
                    }
                    let c = hits_cache.as_ref().unwrap();
                    (c.1, c.2, &c.3)
                }
            };

            let surf_offset_y = if kind == OverlayKind::Keys {
                keys_trail_area_h(&cfg).round() as i32
            } else {
                0
            };

            if !exists {
                if let (Ok(win), Ok(gc)) = (conn.generate_id(), conn.generate_id()) {
                    let aux = CreateWindowAux::new()
                        .background_pixel(0)
                        .border_pixel(0)
                        .override_redirect(1)
                        .colormap(cmap)
                        .event_mask(
                            EventMask::BUTTON_PRESS
                                | EventMask::BUTTON_RELEASE
                                | EventMask::POINTER_MOTION,
                        );
                    let sx = px.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                    let sy = (py - surf_offset_y).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                    if conn
                        .create_window(
                            depth,
                            win,
                            root,
                            sx,
                            sy,
                            w.min(u16::MAX as u32) as u16,
                            h.min(u16::MAX as u32) as u16,
                            0,
                            WindowClass::INPUT_OUTPUT,
                            visual_id,
                            &aux,
                        )
                        .is_ok()
                    {
                        let _ = conn.create_gc(gc, win, &CreateGCAux::new());
                        let _ = conn.change_property(
                            x11rb::protocol::xproto::PropMode::REPLACE,
                            win,
                            AtomEnum::WM_NAME,
                            AtomEnum::STRING,
                            8,
                            5,
                            b"orisu",
                        );
                        set_x11_click_through(&conn, win, cfg.lock_overlays);
                        let _ = conn.map_window(win);
                        let _ = conn.configure_window(
                            win,
                            &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
                        );
                        entries.push(X11SurfaceEntry {
                            kind,
                            win,
                            gc,
                            w,
                            h,
                            last_locked: Some(cfg.lock_overlays),
                            last_pos: (px, py),
                            surface_offset_y: surf_offset_y,
                        });
                    }
                }
            }

            if let Some(entry) = entries.iter_mut().find(|e| e.kind == kind) {
                let offset_changed = entry.surface_offset_y != surf_offset_y;
                if offset_changed {
                    entry.surface_offset_y = surf_offset_y;
                }
                if entry.last_locked != Some(cfg.lock_overlays) {
                    set_x11_click_through(&conn, entry.win, cfg.lock_overlays);
                    entry.last_locked = Some(cfg.lock_overlays);
                }

                let size_changed = entry.w != w || entry.h != h;
                if size_changed {
                    entry.w = w;
                    entry.h = h;
                    let _ = conn.configure_window(
                        entry.win,
                        &ConfigureWindowAux::new()
                            .width(w)
                            .height(h)
                            .stack_mode(StackMode::ABOVE),
                    );
                }

                let is_dragging = dragging.map(|d| d.0) == Some(kind);
                if (!is_dragging && entry.last_pos != (px, py)) || offset_changed {
                    if !is_dragging {
                        entry.last_pos = (px, py);
                    }
                    let sx = entry.last_pos.0;
                    let sy = entry.last_pos.1 - entry.surface_offset_y;
                    let _ = conn.configure_window(
                        entry.win,
                        &ConfigureWindowAux::new()
                            .x(sx)
                            .y(sy)
                            .stack_mode(StackMode::ABOVE),
                    );
                } else if tick_count.is_multiple_of(20) {
                    let _ = conn.configure_window(
                        entry.win,
                        &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
                    );
                }

                let hash = hash_bytes(bgra);
                if last_hashes[kind as usize] != hash || size_changed || trail_active || rgb_active
                {
                    last_hashes[kind as usize] = hash;
                    let _ = conn.put_image(
                        ImageFormat::Z_PIXMAP,
                        entry.win,
                        entry.gc,
                        w as u16,
                        h as u16,
                        0,
                        0,
                        0,
                        depth,
                        bgra,
                    );
                }
            }
        }

        let _ = conn.flush();
        tick_count = tick_count.wrapping_add(1);
        let sleep_ms = if entries.is_empty() {
            100
        } else if trail_active || rgb_active {
            16
        } else {
            cfg.hitcounter_poll_ms.clamp(8, 33) as u64
        };
        wait_overlay_tick(sleep_ms);
    }
}

fn wanted_overlays(
    shared: &OverlaySharedState,
    cfg: &crate::config::Config,
) -> [(OverlayKind, bool, i32, i32); 3] {
    let (kx, ky) = shared.dragged_pos[0].unwrap_or((cfg.keys_pos_x, cfg.keys_pos_y));
    let (rx, ry) = shared.dragged_pos[1].unwrap_or((cfg.radar_pos_x, cfg.radar_pos_y));
    let (hx, hy) =
        shared.dragged_pos[2].unwrap_or((cfg.hitcounter_position_x, cfg.hitcounter_position_y));
    [
        (
            OverlayKind::Keys,
            shared.float_keys && cfg.show_keys_overlay,
            kx,
            ky,
        ),
        (
            OverlayKind::Radar,
            shared.float_radar && cfg.show_radar_overlay,
            rx,
            ry,
        ),
        (
            OverlayKind::Hits,
            shared.float_hits && cfg.show_hits_overlay,
            hx,
            hy,
        ),
    ]
}

fn snapshot_state(
    last_cfg_rev: &mut u64,
    cfg: &mut crate::config::Config,
) -> (OverlaySharedState, [bool; 4], [u64; 4]) {
    let cur_rev = CONFIG_REV.load(Ordering::Acquire);
    if *last_cfg_rev != cur_rev {
        *cfg = read_config();
        *last_cfg_rev = cur_rev;
    }

    let mut shared = OVERLAY_SHARED.lock().map(|g| g.clone()).unwrap_or_default();
    if (shared.float_keys || shared.float_radar || shared.float_hits)
        && crate::proxy::TOSU_CONNECTED.load(Ordering::Relaxed)
    {
        if let Ok(live) = crate::proxy::LIVE_OSU_STATE.read() {
            if !live.song_title.is_empty() {
                shared.title = live.song_title.clone();
                shared.artist = live.song_artist.clone();
                shared.diff = live.song_difficulty.clone();
            }
            if let Some(ratings) = &live.ratings {
                shared.ratings = ratings.clone();
            }
            if !live.bg_path.is_empty() && shared.ratings.bg_path.is_empty() {
                shared.ratings.bg_path = live.bg_path.clone();
            }
            shared.hit_counts = live.hits;
        }
    }
    let pressed = [
        LIVE_KEY_STATES[0].load(Ordering::Relaxed),
        LIVE_KEY_STATES[1].load(Ordering::Relaxed),
        LIVE_KEY_STATES[2].load(Ordering::Relaxed),
        LIVE_KEY_STATES[3].load(Ordering::Relaxed),
    ];
    let hold_ms = [
        LIVE_KEY_HOLD_MS[0].load(Ordering::Relaxed),
        LIVE_KEY_HOLD_MS[1].load(Ordering::Relaxed),
        LIVE_KEY_HOLD_MS[2].load(Ordering::Relaxed),
        LIVE_KEY_HOLD_MS[3].load(Ordering::Relaxed),
    ];
    (shared, pressed, hold_ms)
}

fn hash_bytes(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325 ^ (data.len() as u64);
    for &chunk in data.as_chunks::<4>().0 {
        let v = u32::from_le_bytes(chunk) as u64;
        h ^= v;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn fnv_str(h: &mut u64, s: &str) {
    for &b in s.as_bytes() {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x100000001b3);
    }
    *h ^= 0xff;
}

fn wait_overlay_tick(sleep_ms: u64) {
    let (lock, cvar) = &*crate::proxy::OVERLAY_WAKE;
    if let Ok(guard) = lock.lock() {
        let _ = cvar.wait_timeout(guard, Duration::from_millis(sleep_ms));
    } else {
        std::thread::sleep(Duration::from_millis(sleep_ms));
    }
}

fn fnv_adaptive(h: &mut u64, r: &SkillRatings) {
    fnv_str(h, &r.adaptive_accent);
    fnv_str(h, &r.adaptive_accent_secondary);
    fnv_str(h, &r.adaptive_bg);
    fnv_str(h, &r.adaptive_item_bg);
    fnv_str(h, &r.adaptive_border);
}

fn keys_input_key(
    cfg: &crate::config::Config,
    pressed: &[bool; 4],
    hold_ms: &[u64; 4],
    ratings: &SkillRatings,
    tosu_connected: bool,
) -> u64 {
    let mut h: u64 = 0x811c9dc5 ^ CONFIG_REV.load(Ordering::Relaxed) ^ (tosu_connected as u64);
    for (i, &is_down) in pressed.iter().enumerate() {
        h ^= (is_down as u64) << i;
        h = h.wrapping_mul(0x100000001b3) ^ hold_ms[i];
    }
    if cfg.keys_auto_bg_theme {
        fnv_adaptive(&mut h, ratings);
    }
    h
}

fn radar_input_key(shared: &OverlaySharedState, time_bucket: i64, tosu_connected: bool) -> u64 {
    let mut h: u64 = 0x811c9dc5
        ^ (time_bucket as u64)
        ^ CONFIG_REV.load(Ordering::Relaxed)
        ^ (tosu_connected as u64);
    fnv_str(&mut h, &shared.title);
    fnv_str(&mut h, &shared.artist);
    fnv_str(&mut h, &shared.diff);
    fnv_str(&mut h, &shared.ratings.bg_path);
    fnv_adaptive(&mut h, &shared.ratings);
    h = h.wrapping_mul(0x100000001b3) ^ (shared.ratings.overall.to_bits() as u64);
    h = h.wrapping_mul(0x100000001b3) ^ (shared.ratings.dan.level.to_bits() as u64);
    h = h.wrapping_mul(0x100000001b3) ^ (shared.ratings.dan.strain_curve.len() as u64);
    h
}

fn hits_input_key(
    cfg: &crate::config::Config,
    hit_counts: &[u32; 6],
    ratings: &SkillRatings,
    tosu_connected: bool,
) -> u64 {
    let mut h: u64 = 0x811c9dc5 ^ CONFIG_REV.load(Ordering::Relaxed) ^ (tosu_connected as u64);
    for &c in hit_counts {
        h = h.wrapping_mul(0x100000001b3) ^ (c as u64);
    }
    if cfg.hitcounter_auto_bg_theme {
        fnv_adaptive(&mut h, ratings);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_all_three_overlay_buffers() {
        let cfg = crate::config::Config::default();
        let ratings = OverlaySharedState::default().ratings;
        let (kw, kh, kbuf) =
            render_keys_bgra(&cfg, &[true, false, true, false], &[42, 0, 12, 0], &[], 0.0);
        assert!(kw > 0 && kh > 0);
        assert_eq!(kbuf.len(), (kw * kh * 4) as usize);
        assert!(kbuf.iter().any(|&b| b != 0));

        let (rw, rh, rbuf) = render_radar_bgra(&cfg, "Map Title", "Artist", "Diff", &ratings);
        assert!(rw > 0 && rh > 0);
        assert_eq!(rbuf.len(), (rw * rh * 4) as usize);
        assert!(rbuf.iter().any(|&b| b != 0));

        let mut tall_cfg = cfg.clone();
        tall_cfg.trail_height = 680;
        let (_tkw, tkh, _tkbuf) = render_keys_bgra(
            &tall_cfg,
            &[true, false, false, false],
            &[10, 0, 0, 0],
            &[],
            0.0,
        );
        assert!(tkh > 680);

        let (hw, hh, hbuf) = render_hits_bgra(&cfg, &[482, 94, 18, 5, 1, 3]);
        assert!(hw >= cfg.hitcounter_item_width && hh > 0);
        assert_eq!(hbuf.len(), (hw * hh * 4) as usize);
        assert!(hbuf.iter().any(|&b| b != 0));

        let mut empty_curve_ratings = ratings.clone();
        empty_curve_ratings.dan.strain_curve.clear();
        let (erw, erh, _) = render_radar_bgra_live(
            &cfg,
            "Map Title",
            "Artist",
            "Diff",
            &empty_curve_ratings,
            0,
            false,
        );
        assert_eq!((erw, erh), (rw, rh));

        let mut alt_cfg = cfg.clone();
        for ks in ["dock-tint", "dock", "tiles", "flat"] {
            alt_cfg.keys_style = ks.to_string();
            let (dkw, dkh, dkbuf) = render_keys_bgra_live(
                &alt_cfg,
                &[true, false, false, true],
                &[10; 4],
                &[],
                0.0,
                None,
                false,
            );
            assert!(
                dkw > 0 && dkh > 0 && dkbuf.iter().any(|&b| b != 0),
                "keys style {} failed",
                ks
            );
        }

        for hs in ["radar-twin", "grid-2x3", "dock", "dock-ratio", "hud"] {
            alt_cfg.hitcounter_style = hs.to_string();
            alt_cfg.hitcounter_show_ratio = true;
            let (hhw, hhh, hhbuf) =
                render_hits_bgra_live(&alt_cfg, &[100, 50, 2, 0, 0, 1], None, false);
            assert!(
                hhw > 0 && hhh > 0 && hhbuf.iter().any(|&b| b != 0),
                "hits style {} failed",
                hs
            );
        }
    }

    #[test]
    #[ignore]
    fn verify_live_wayland_and_x11_overlays() {
        if std::env::var("WAYLAND_DISPLAY")
            .map(|v| !v.is_empty())
            .unwrap_or(false)
        {
            let conn = Connection::connect_to_env().expect("wayland connect");
            let mut eq = conn.new_event_queue();
            let qh = eq.handle();
            let _reg = conn.display().get_registry(&qh, ());
            let mut app = WaylandApp::new();
            eq.roundtrip(&mut app).expect("roundtrip 1");
            assert!(app.layer_shell.is_some(), "zwlr_layer_shell_v1 required");
            let cfg = crate::config::Config::default();
            let (rw, rh, rbuf) = render_radar_bgra(
                &cfg,
                "Frontier Explorer",
                "kiwi_iwata",
                "Another",
                &OverlaySharedState::default().ratings,
            );
            spawn_wl_surface(&mut app, &qh, OverlayKind::Radar, 24, 24, 0, rw, rh, true)
                .expect("spawn layer surface");
            let (kw, kh, _kbuf) = render_keys_bgra(&cfg, &[false; 4], &[0; 4], &[], 0.0);
            let k_off = keys_trail_area_h(&cfg).round() as i32;
            spawn_wl_surface(
                &mut app,
                &qh,
                OverlayKind::Keys,
                360,
                0,
                k_off,
                kw,
                kh,
                false,
            )
            .expect("spawn keys layer surface at y=0");
            eq.roundtrip(&mut app).expect("configure roundtrip");
            assert!(
                app.surfaces.iter().all(|e| e.configured),
                "all layer surfaces must configure"
            );
            let entry = app.surfaces.first_mut().expect("surface exists");
            let shm = app.shm.as_ref().expect("wl_shm");
            update_wl_buffer(entry, shm, &qh, rw, rh, &rbuf);
            let _ = conn.flush();
            std::thread::sleep(Duration::from_millis(250));
            println!(
                "wayland Layer::Overlay + rel_ptr({}) verified ({}x{})",
                app.rel_mgr.is_some(),
                rw,
                rh
            );
        } else if std::env::var_os("DISPLAY").is_some() {
            let (conn, screen_num) = x11rb::connect(None).expect("x11 connect");
            let screen = &conn.setup().roots[screen_num];
            let (depth, vis) = find_argb_visual(screen).expect("32-bit argb visual");
            let cmap = conn.generate_id().unwrap();
            conn.create_colormap(ColormapAlloc::NONE, cmap, screen.root, vis)
                .unwrap();
            let cfg = crate::config::Config::default();
            let (rw, rh, _rbuf) = render_radar_bgra(
                &cfg,
                "Frontier Explorer",
                "kiwi_iwata",
                "Another",
                &OverlaySharedState::default().ratings,
            );
            let win = conn.generate_id().unwrap();
            conn.create_window(
                depth,
                win,
                screen.root,
                24,
                24,
                rw as u16,
                rh as u16,
                0,
                WindowClass::INPUT_OUTPUT,
                vis,
                &CreateWindowAux::new()
                    .background_pixel(0)
                    .border_pixel(0)
                    .colormap(cmap)
                    .override_redirect(1),
            )
            .unwrap();
            set_x11_click_through(&conn, win, true);
            conn.map_window(win).unwrap();
            let _ =
                conn.configure_window(win, &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE));
            conn.flush().unwrap();
            let attrs = conn.get_window_attributes(win).unwrap().reply().unwrap();
            assert!(attrs.override_redirect);
            println!(
                "x11 override_redirect + shape_rectangles verified ({}x{})",
                rw, rh
            );
        } else {
            panic!("neither WAYLAND_DISPLAY nor DISPLAY is set");
        }
    }
}
