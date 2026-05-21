use futures_util::SinkExt;
use futures_util::StreamExt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tauri::Emitter;

use crate::KEY_SENDER;

pub static IS_LISTENING_FOR_BIND: AtomicBool = AtomicBool::new(false);

static ACTIVE_KEY_0: AtomicU64 = AtomicU64::new(9999);
static ACTIVE_KEY_1: AtomicU64 = AtomicU64::new(9999);
static ACTIVE_KEY_2: AtomicU64 = AtomicU64::new(9999);
static ACTIVE_KEY_3: AtomicU64 = AtomicU64::new(9999);

fn parse_rdev_key(s: &str) -> Option<rdev::Key> {
    match s {
        "KeyA" => Some(rdev::Key::KeyA),
        "KeyB" => Some(rdev::Key::KeyB),
        "KeyC" => Some(rdev::Key::KeyC),
        "KeyD" => Some(rdev::Key::KeyD),
        "KeyE" => Some(rdev::Key::KeyE),
        "KeyF" => Some(rdev::Key::KeyF),
        "KeyG" => Some(rdev::Key::KeyG),
        "KeyH" => Some(rdev::Key::KeyH),
        "KeyI" => Some(rdev::Key::KeyI),
        "KeyJ" => Some(rdev::Key::KeyJ),
        "KeyK" => Some(rdev::Key::KeyK),
        "KeyL" => Some(rdev::Key::KeyL),
        "KeyM" => Some(rdev::Key::KeyM),
        "KeyN" => Some(rdev::Key::KeyN),
        "KeyO" => Some(rdev::Key::KeyO),
        "KeyP" => Some(rdev::Key::KeyP),
        "KeyQ" => Some(rdev::Key::KeyQ),
        "KeyR" => Some(rdev::Key::KeyR),
        "KeyS" => Some(rdev::Key::KeyS),
        "KeyT" => Some(rdev::Key::KeyT),
        "KeyU" => Some(rdev::Key::KeyU),
        "KeyV" => Some(rdev::Key::KeyV),
        "KeyW" => Some(rdev::Key::KeyW),
        "KeyX" => Some(rdev::Key::KeyX),
        "KeyY" => Some(rdev::Key::KeyY),
        "KeyZ" => Some(rdev::Key::KeyZ),
        "Num0" => Some(rdev::Key::Num0),
        "Num1" => Some(rdev::Key::Num1),
        "Num2" => Some(rdev::Key::Num2),
        "Num3" => Some(rdev::Key::Num3),
        "Num4" => Some(rdev::Key::Num4),
        "Num5" => Some(rdev::Key::Num5),
        "Num6" => Some(rdev::Key::Num6),
        "Num7" => Some(rdev::Key::Num7),
        "Num8" => Some(rdev::Key::Num8),
        "Num9" => Some(rdev::Key::Num9),
        "Space" => Some(rdev::Key::Space),
        "Backspace" => Some(rdev::Key::Backspace),
        "Tab" => Some(rdev::Key::Tab),
        "Return" => Some(rdev::Key::Return),
        "ShiftLeft" => Some(rdev::Key::ShiftLeft),
        "ShiftRight" => Some(rdev::Key::ShiftRight),
        "ControlLeft" => Some(rdev::Key::ControlLeft),
        "ControlRight" => Some(rdev::Key::ControlRight),
        "Alt" => Some(rdev::Key::Alt),
        "AltGr" => Some(rdev::Key::AltGr),
        "Semicolon" => Some(rdev::Key::SemiColon),
        "Equal" => Some(rdev::Key::Equal),
        "Comma" => Some(rdev::Key::Comma),
        "Minus" => Some(rdev::Key::Minus),
        "Dot" => Some(rdev::Key::Dot),
        "Slash" => Some(rdev::Key::Slash),
        "Backquote" => Some(rdev::Key::BackQuote),
        "LeftBracket" => Some(rdev::Key::LeftBracket),
        "Backslash" => Some(rdev::Key::BackSlash),
        "RightBracket" => Some(rdev::Key::RightBracket),
        "Quote" => Some(rdev::Key::Quote),
        _ => {
            if s.starts_with("Unknown(") && s.ends_with(')') {
                if let Ok(code) = s[8..s.len() - 1].parse::<u32>() {
                    return Some(rdev::Key::Unknown(code));
                }
            }
            None
        }
    }
}

fn key_to_u64(k: rdev::Key) -> u64 {
    unsafe { std::mem::transmute::<rdev::Key, u64>(k) }
}

pub async fn start_key_server(app: tauri::AppHandle) {
    {
        let config = crate::config::read_config();
        update_active_keys(&config.keys);
    }

    let listener = match tokio::net::TcpListener::bind("127.0.0.1:24051").await {
        Ok(l) => l,
        Err(_) => return,
    };

    let app_clone = app.clone();
    std::thread::spawn(move || {
        if let Err(_) = rdev::listen(move |event| {
            let is_listening = IS_LISTENING_FOR_BIND.load(Ordering::Relaxed);

            if !is_listening && KEY_SENDER.receiver_count() == 0 {
                return;
            }

            let key = match event.event_type {
                rdev::EventType::KeyPress(k) | rdev::EventType::KeyRelease(k) => k,
                _ => return,
            };

            let key_u64 = key_to_u64(key);

            if is_listening {
                if let rdev::EventType::KeyPress(_) = event.event_type {
                    let raw = format!("{:?}", key);
                    let _ = app_clone.emit("bind-key", raw);
                }
                return;
            }

            let idx = if key_u64 == ACTIVE_KEY_0.load(Ordering::Relaxed) {
                0
            } else if key_u64 == ACTIVE_KEY_1.load(Ordering::Relaxed) {
                1
            } else if key_u64 == ACTIVE_KEY_2.load(Ordering::Relaxed) {
                2
            } else if key_u64 == ACTIVE_KEY_3.load(Ordering::Relaxed) {
                3
            } else {
                return;
            };

            match event.event_type {
                rdev::EventType::KeyPress(_) => {
                    let msg = match idx {
                        0 => r#"{"event":"key-down","index":0}"#,
                        1 => r#"{"event":"key-down","index":1}"#,
                        2 => r#"{"event":"key-down","index":2}"#,
                        3 => r#"{"event":"key-down","index":3}"#,
                        _ => return,
                    };
                    let _ = KEY_SENDER.send(msg.to_string());
                }
                rdev::EventType::KeyRelease(_) => {
                    let msg = match idx {
                        0 => r#"{"event":"key-up","index":0}"#,
                        1 => r#"{"event":"key-up","index":1}"#,
                        2 => r#"{"event":"key-up","index":2}"#,
                        3 => r#"{"event":"key-up","index":3}"#,
                        _ => return,
                    };
                    let _ = KEY_SENDER.send(msg.to_string());
                }
                _ => {}
            }
        }) {}
    });

    while let Ok((stream, _)) = listener.accept().await {
        let _ = stream.set_nodelay(true);
        let tx = KEY_SENDER.clone();
        tokio::spawn(async move {
            if let Ok(ws_stream) = tokio_tungstenite::accept_async(stream).await {
                let (mut ws_sender, mut ws_receiver) = ws_stream.split();

                tokio::spawn(async move {
                    while let Some(_) = ws_receiver.next().await {}
                });

                let config = crate::config::read_config();
                let mut val = serde_json::to_value(&config).unwrap_or_default();
                if let Some(obj) = val.as_object_mut() {
                    obj.insert("event".to_string(), serde_json::json!("bindings"));
                }
                let bindings_msg = val.to_string();
                let _ = ws_sender.send(tokio_tungstenite::tungstenite::Message::Text(bindings_msg)).await;

                let mut rx = tx.subscribe();
                loop {
                    match rx.recv().await {
                        Ok(msg) => {
                            if ws_sender.send(tokio_tungstenite::tungstenite::Message::Text(msg)).await.is_err() {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            continue;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
            }
        });
    }
}

pub fn update_active_keys(keys: &[String]) {
    let rkeys: Vec<u64> = keys.iter()
        .filter_map(|k| parse_rdev_key(k))
        .map(key_to_u64)
        .collect();

    ACTIVE_KEY_0.store(*rkeys.get(0).unwrap_or(&9999), Ordering::Relaxed);
    ACTIVE_KEY_1.store(*rkeys.get(1).unwrap_or(&9999), Ordering::Relaxed);
    ACTIVE_KEY_2.store(*rkeys.get(2).unwrap_or(&9999), Ordering::Relaxed);
    ACTIVE_KEY_3.store(*rkeys.get(3).unwrap_or(&9999), Ordering::Relaxed);
}
