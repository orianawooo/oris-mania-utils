use futures_util::SinkExt;
use futures_util::StreamExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use tauri::Emitter;

use crate::KEY_SENDER;

pub static IS_LISTENING_FOR_BIND: AtomicBool = AtomicBool::new(false);

lazy_static::lazy_static! {
    static ref ACTIVE_KEYS_NORMALIZED: RwLock<Vec<String>> = RwLock::new(vec![
        "d".to_string(),
        "f".to_string(),
        "j".to_string(),
        "k".to_string()
    ]);
}

fn normalize_key(s: &str) -> String {
    let base = s.replace("Key", "").to_lowercase();
    if base == "backquote" {
        "semicolon".to_string()
    } else {
        base
    }
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

            let key_str = format!("{:?}", key);

            if is_listening {
                if let rdev::EventType::KeyPress(_) = event.event_type {
                    let _ = app_clone.emit("bind-key", key_str);
                }
                return;
            }

            let norm_key = normalize_key(&key_str);

            let idx = if let Ok(active) = ACTIVE_KEYS_NORMALIZED.read() {
                active.iter().position(|k| *k == norm_key)
            } else {
                None
            };

            if let Some(idx) = idx {
                match event.event_type {
                    rdev::EventType::KeyPress(_) => {
                        let msg = format!(r#"{{"event":"key-down","index":{}}}"#, idx);
                        let _ = KEY_SENDER.send(msg);
                    }
                    rdev::EventType::KeyRelease(_) => {
                        let msg = format!(r#"{{"event":"key-up","index":{}}}"#, idx);
                        let _ = KEY_SENDER.send(msg);
                    }
                    _ => {}
                }
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
                    obj.insert("event".to_string(), serde_json::json!("config-updated"));
                    obj.insert(
                        "changed_scopes".to_string(),
                        serde_json::json!(["msdconverter", "HitCounter", "ManiaKeystrokes"]),
                    );
                }
                let config_msg = val.to_string();
                let _ = ws_sender.send(tokio_tungstenite::tungstenite::Message::Text(config_msg)).await;

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
    if let Ok(mut active) = ACTIVE_KEYS_NORMALIZED.write() {
        active.clear();
        for k in keys {
            active.push(normalize_key(k));
        }
    }
}
