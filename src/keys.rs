use futures_util::SinkExt;
use futures_util::StreamExt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock, RwLock};
use std::time::Instant;

use crate::KEY_SENDER;

pub static IS_LISTENING_FOR_BIND: AtomicBool = AtomicBool::new(false);
pub static LIVE_KEY_STATES: [AtomicBool; 4] = [
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
    AtomicBool::new(false),
];
pub static LIVE_KEY_HOLD_MS: [AtomicU64; 4] = [
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
];

static KEY_LISTENER_ONCE: OnceLock<()> = OnceLock::new();
static WS_SERVER_ONCE: OnceLock<()> = OnceLock::new();
static TOKIO_HANDLE: OnceLock<tokio::runtime::Handle> = OnceLock::new();

lazy_static::lazy_static! {
    static ref ACTIVE_KEYS_NORMALIZED: RwLock<Vec<String>> = RwLock::new(vec![
        "keyd".to_string(),
        "keyf".to_string(),
        "keyj".to_string(),
        "keyk".to_string()
    ]);
    static ref FAST_KEY_ENUMS: RwLock<[Option<rdev::Key>; 4]> = RwLock::new([
        Some(rdev::Key::KeyD),
        Some(rdev::Key::KeyF),
        Some(rdev::Key::KeyJ),
        Some(rdev::Key::KeyK),
    ]);
    static ref PRESS_INSTANTS: RwLock<[Option<Instant>; 4]> = RwLock::new([None, None, None, None]);
    static ref CAPTURED_BIND_KEY: Mutex<Option<(String, String)>> = Mutex::new(None);
}

pub fn register_tokio_handle(handle: tokio::runtime::Handle) {
    let _ = TOKIO_HANDLE.set(handle);
}

pub fn begin_key_capture() {
    if let Ok(mut g) = CAPTURED_BIND_KEY.lock() {
        *g = None;
    }
    IS_LISTENING_FOR_BIND.store(true, Ordering::Release);
}

pub fn end_key_capture() {
    IS_LISTENING_FOR_BIND.store(false, Ordering::Release);
    if let Ok(mut g) = CAPTURED_BIND_KEY.lock() {
        *g = None;
    }
}

pub fn take_captured_key() -> Option<(String, String)> {
    CAPTURED_BIND_KEY.lock().ok().and_then(|mut g| g.take())
}

pub fn rdev_key_to_canonical(key: rdev::Key) -> String {
    match key {
        rdev::Key::Unknown(code) => format!("Unknown:{}", code),
        other => format!("{:?}", other),
    }
}

pub fn key_name_to_display_label(s: &str) -> String {
    let trimmed = s.trim();
    if let Some(rest) = trimmed.strip_prefix("Key") {
        return rest.to_uppercase();
    }
    if let Some(rest) = trimmed.strip_prefix("Digit") {
        return rest.to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("Num") {
        if rest.len() == 1 {
            return rest.to_string();
        }
    }
    if let Some(rest) = trimmed.strip_prefix("Kp") {
        return match rest {
            "Return" => "KPE".to_string(),
            "Minus" => "KP-".to_string(),
            "Plus" => "KP+".to_string(),
            "Multiply" => "KP*".to_string(),
            "Divide" => "KP/".to_string(),
            "Delete" => "KP.".to_string(),
            other => format!("KP{}", other),
        };
    }
    if let Some(code) = trimmed.strip_prefix("Unknown:") {
        return format!("K{}", code);
    }
    match trimmed.to_lowercase().as_str() {
        "comma" => ",".to_string(),
        "dot" | "period" => ".".to_string(),
        "semicolon" => ";".to_string(),
        "quote" => "'".to_string(),
        "backquote" => "`".to_string(),
        "slash" => "/".to_string(),
        "backslash" => "\\".to_string(),
        "intlbackslash" => "<>".to_string(),
        "leftbracket" => "[".to_string(),
        "rightbracket" => "]".to_string(),
        "minus" => "-".to_string(),
        "equal" => "=".to_string(),
        "space" => "SPC".to_string(),
        "return" | "enter" => "ENT".to_string(),
        "tab" => "TAB".to_string(),
        "backspace" => "BSP".to_string(),
        "escape" => "ESC".to_string(),
        "capslock" => "CAP".to_string(),
        "shiftleft" => "LSH".to_string(),
        "shiftright" => "RSH".to_string(),
        "controlleft" => "LCT".to_string(),
        "controlright" => "RCT".to_string(),
        "alt" => "ALT".to_string(),
        "altgr" => "AGR".to_string(),
        "metaleft" => "WIN".to_string(),
        "metaright" => "RWN".to_string(),
        "leftarrow" | "arrowleft" => "<-".to_string(),
        "rightarrow" | "arrowright" => "->".to_string(),
        "uparrow" | "arrowup" => "UP".to_string(),
        "downarrow" | "arrowdown" => "DN".to_string(),
        other => other.to_uppercase(),
    }
}

fn normalize_key(s: &str) -> String {
    s.trim().to_lowercase()
}

pub fn parse_rdev_key(s: &str) -> Option<rdev::Key> {
    use rdev::Key;
    let low = normalize_key(s);
    if let Some(code_str) = low.strip_prefix("unknown:") {
        if let Ok(code) = code_str.parse::<u32>() {
            return Some(Key::Unknown(code));
        }
    }
    if let Some(inner) = low
        .strip_prefix("unknown(")
        .and_then(|v| v.strip_suffix(')'))
    {
        if let Ok(code) = inner.parse::<u32>() {
            return Some(Key::Unknown(code));
        }
    }

    let k = low.strip_prefix("key").unwrap_or(&low);
    Some(match k {
        "a" => Key::KeyA,
        "b" => Key::KeyB,
        "c" => Key::KeyC,
        "d" => Key::KeyD,
        "e" => Key::KeyE,
        "f" => Key::KeyF,
        "g" => Key::KeyG,
        "h" => Key::KeyH,
        "i" => Key::KeyI,
        "j" => Key::KeyJ,
        "k" => Key::KeyK,
        "l" => Key::KeyL,
        "m" => Key::KeyM,
        "n" => Key::KeyN,
        "o" => Key::KeyO,
        "p" => Key::KeyP,
        "q" => Key::KeyQ,
        "r" => Key::KeyR,
        "s" => Key::KeyS,
        "t" => Key::KeyT,
        "u" => Key::KeyU,
        "v" => Key::KeyV,
        "w" => Key::KeyW,
        "x" => Key::KeyX,
        "y" => Key::KeyY,
        "z" => Key::KeyZ,
        "0" | "num0" | "digit0" => Key::Num0,
        "1" | "num1" | "digit1" => Key::Num1,
        "2" | "num2" | "digit2" => Key::Num2,
        "3" | "num3" | "digit3" => Key::Num3,
        "4" | "num4" | "digit4" => Key::Num4,
        "5" | "num5" | "digit5" => Key::Num5,
        "6" | "num6" | "digit6" => Key::Num6,
        "7" | "num7" | "digit7" => Key::Num7,
        "8" | "num8" | "digit8" => Key::Num8,
        "9" | "num9" | "digit9" => Key::Num9,
        "space" => Key::Space,
        "shiftleft" => Key::ShiftLeft,
        "shiftright" => Key::ShiftRight,
        "controlleft" => Key::ControlLeft,
        "controlright" => Key::ControlRight,
        "alt" => Key::Alt,
        "altgr" => Key::AltGr,
        "metaleft" => Key::MetaLeft,
        "metaright" => Key::MetaRight,
        "tab" => Key::Tab,
        "return" | "enter" => Key::Return,
        "backspace" => Key::Backspace,
        "escape" => Key::Escape,
        "capslock" => Key::CapsLock,
        "semicolon" => Key::SemiColon,
        "quote" => Key::Quote,
        "backquote" => Key::BackQuote,
        "comma" => Key::Comma,
        "dot" | "period" => Key::Dot,
        "slash" => Key::Slash,
        "backslash" => Key::BackSlash,
        "intlbackslash" => Key::IntlBackslash,
        "leftbracket" | "openbracket" => Key::LeftBracket,
        "rightbracket" | "closebracket" => Key::RightBracket,
        "minus" => Key::Minus,
        "equal" | "equals" => Key::Equal,
        "leftarrow" | "arrowleft" => Key::LeftArrow,
        "rightarrow" | "arrowright" => Key::RightArrow,
        "uparrow" | "arrowup" => Key::UpArrow,
        "downarrow" | "arrowdown" => Key::DownArrow,
        "insert" => Key::Insert,
        "delete" => Key::Delete,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "printscreen" => Key::PrintScreen,
        "scrolllock" => Key::ScrollLock,
        "pause" => Key::Pause,
        "numlock" => Key::NumLock,
        "function" => Key::Function,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        "kp0" | "numpad0" => Key::Kp0,
        "kp1" | "numpad1" => Key::Kp1,
        "kp2" | "numpad2" => Key::Kp2,
        "kp3" | "numpad3" => Key::Kp3,
        "kp4" | "numpad4" => Key::Kp4,
        "kp5" | "numpad5" => Key::Kp5,
        "kp6" | "numpad6" => Key::Kp6,
        "kp7" | "numpad7" => Key::Kp7,
        "kp8" | "numpad8" => Key::Kp8,
        "kp9" | "numpad9" => Key::Kp9,
        "kpreturn" | "numpadenter" => Key::KpReturn,
        "kpminus" | "kp_subtract" => Key::KpMinus,
        "kpplus" | "kp_add" => Key::KpPlus,
        "kpmultiply" => Key::KpMultiply,
        "kpdivide" => Key::KpDivide,
        "kpdelete" | "kp_decimal" => Key::KpDelete,
        _ => return None,
    })
}

fn match_key_index(key: rdev::Key) -> Option<usize> {
    if let Ok(fast) = FAST_KEY_ENUMS.read() {
        let mut all_known = true;
        for (i, slot) in fast.iter().enumerate() {
            match slot {
                Some(k) if *k == key => return Some(i),
                None => all_known = false,
                _ => {}
            }
        }
        if all_known {
            return None;
        }
    }
    let norm_key = normalize_key(&rdev_key_to_canonical(key));
    ACTIVE_KEYS_NORMALIZED
        .read()
        .ok()
        .and_then(|active| active.iter().position(|k| *k == norm_key))
}

pub fn start_key_listener() {
    KEY_LISTENER_ONCE.get_or_init(|| {
        let config = crate::config::read_config();
        update_active_keys(&config.keys);

        std::thread::spawn(move || {
            let _ = rdev::listen(move |event| {
                if IS_LISTENING_FOR_BIND.load(Ordering::Acquire) {
                    if let rdev::EventType::KeyPress(k) = event.event_type {
                        if k == rdev::Key::Escape {
                            if let Ok(mut slot) = CAPTURED_BIND_KEY.lock() {
                                *slot = Some(("__CANCEL__".to_string(), String::new()));
                            }
                            IS_LISTENING_FOR_BIND.store(false, Ordering::Release);
                            return;
                        }
                        let canonical = rdev_key_to_canonical(k);
                        let label = event
                            .name
                            .as_deref()
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty() && s.chars().all(|c| !c.is_control()))
                            .map(|s| s.to_uppercase())
                            .unwrap_or_else(|| key_name_to_display_label(&canonical));
                        if let Ok(mut slot) = CAPTURED_BIND_KEY.lock() {
                            *slot = Some((canonical, label));
                        }
                        IS_LISTENING_FOR_BIND.store(false, Ordering::Release);
                    }
                    return;
                }

                let key = match event.event_type {
                    rdev::EventType::KeyPress(k) | rdev::EventType::KeyRelease(k) => k,
                    _ => return,
                };

                let Some(idx) = match_key_index(key) else {
                    return;
                };
                if idx >= 4 {
                    return;
                }

                crate::HIT_POLL_TRIGGER.store(true, Ordering::Release);
                crate::proxy::wake_overlay_thread();
                let has_ws_clients =
                    crate::config::runtime_browser_overlays() && KEY_SENDER.receiver_count() > 0;

                match event.event_type {
                    rdev::EventType::KeyPress(_) => {
                        if !LIVE_KEY_STATES[idx].swap(true, Ordering::Relaxed) {
                            if let Ok(mut times) = PRESS_INSTANTS.write() {
                                times[idx] = Some(Instant::now());
                            }
                        }
                        if has_ws_clients {
                            let msg = format!(r#"{{"event":"key-down","index":{}}}"#, idx);
                            let _ = KEY_SENDER.send(msg);
                        }
                    }
                    rdev::EventType::KeyRelease(_) => {
                        LIVE_KEY_STATES[idx].store(false, Ordering::Relaxed);
                        if let Ok(mut times) = PRESS_INSTANTS.write() {
                            if let Some(started) = times[idx].take() {
                                LIVE_KEY_HOLD_MS[idx]
                                    .store(started.elapsed().as_millis() as u64, Ordering::Relaxed);
                            }
                        }
                        if has_ws_clients {
                            let msg = format!(r#"{{"event":"key-up","index":{}}}"#, idx);
                            let _ = KEY_SENDER.send(msg);
                        }
                    }
                    _ => {}
                }
            });
        });
    });
}

pub fn ensure_ws_server_started() {
    if !crate::config::runtime_browser_overlays() {
        return;
    }
    if WS_SERVER_ONCE.get().is_some() {
        return;
    }
    if let Some(handle) = TOKIO_HANDLE.get() {
        handle.spawn(start_key_server());
    }
}

async fn bind_companion_listener() -> Option<tokio::net::TcpListener> {
    let preferred = crate::config::runtime_companion_port();
    let tosu_port = crate::config::runtime_tosu_port();

    for offset in 0..=32u16 {
        let candidate = preferred.saturating_add(offset);
        if candidate < 1024 || candidate == tosu_port {
            continue;
        }
        if let Ok(listener) = tokio::net::TcpListener::bind(("127.0.0.1", candidate)).await {
            return Some(listener);
        }
    }
    tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.ok()
}

pub async fn start_key_server() {
    if WS_SERVER_ONCE.set(()).is_err() {
        return;
    }
    start_key_listener();

    let Some(listener) = bind_companion_listener().await else {
        return;
    };
    let bound_port = listener
        .local_addr()
        .ok()
        .map(|a| a.port())
        .unwrap_or_else(crate::config::runtime_companion_port);
    crate::config::set_active_companion_port(bound_port);
    crate::config::sync_runtime_overlay_files(&crate::config::read_config());

    while let Ok((stream, _)) = listener.accept().await {
        if !crate::config::runtime_browser_overlays() {
            continue;
        }
        let _ = stream.set_nodelay(true);
        let tx = KEY_SENDER.clone();
        tokio::spawn(async move {
            if let Ok(ws_stream) = tokio_tungstenite::accept_async(stream).await {
                let (mut ws_sender, mut ws_receiver) = ws_stream.split();

                tokio::spawn(async move { while ws_receiver.next().await.is_some() {} });

                let config = crate::config::read_config();
                let mut val = serde_json::to_value(&config).unwrap_or_default();
                if let Some(obj) = val.as_object_mut() {
                    obj.insert("event".to_string(), serde_json::json!("config-updated"));
                    obj.insert(
                        "active_companion_port".to_string(),
                        serde_json::json!(crate::config::runtime_active_companion_port()),
                    );
                    obj.insert(
                        "changed_scopes".to_string(),
                        serde_json::json!(["radar", "hits", "keys"]),
                    );
                }
                if let Ok(cfg_str) = serde_json::to_string(&val) {
                    let _ = ws_sender
                        .send(tokio_tungstenite::tungstenite::Message::Text(cfg_str))
                        .await;
                }

                let cached_msd = crate::proxy::LAST_MSD_EVENT
                    .read()
                    .ok()
                    .and_then(|g| g.clone());
                if let Some(msd_msg) = cached_msd {
                    let _ = ws_sender
                        .send(tokio_tungstenite::tungstenite::Message::Text(msd_msg))
                        .await;
                }

                let mut rx = tx.subscribe();
                while let Ok(msg) = rx.recv().await {
                    if ws_sender
                        .send(tokio_tungstenite::tungstenite::Message::Text(msg))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        });
    }
}

pub fn update_active_keys(keys: &[String]) {
    if let Ok(mut active) = ACTIVE_KEYS_NORMALIZED.write() {
        *active = keys.iter().map(|k| normalize_key(k)).collect();
    }
    if let Ok(mut fast) = FAST_KEY_ENUMS.write() {
        for i in 0..4 {
            fast[i] = keys.get(i).and_then(|s| parse_rdev_key(s));
        }
    }
}
