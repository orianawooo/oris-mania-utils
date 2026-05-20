use futures_util::SinkExt;
use futures_util::StreamExt;
use std::collections::HashSet;
use std::sync::RwLock;

use crate::KEY_SENDER;

lazy_static::lazy_static! {
    pub static ref ACTIVE_KEYS: RwLock<Vec<String>> = RwLock::new(vec![]);
    static ref ACTIVE_RDEV_KEYS: RwLock<HashSet<String>> = RwLock::new(HashSet::new());
}

fn sync_rdev_set(keys: &[String]) {
    if let Ok(mut set) = ACTIVE_RDEV_KEYS.write() {
        set.clear();
        for k in keys {
            set.insert(k.clone());
        }
    }
}

pub async fn start_key_server(_app: tauri::AppHandle) {
    {
        let config = crate::config::read_config();
        if let Ok(mut keys) = ACTIVE_KEYS.write() {
            *keys = config.keys.clone();
            sync_rdev_set(&config.keys);
        }
    }

    let listener = match tokio::net::TcpListener::bind("127.0.0.1:24051").await {
        Ok(l) => l,
        Err(_) => return,
    };

    std::thread::spawn(move || {
        if let Err(_) = rdev::listen(move |event| {
            match event.event_type {
                rdev::EventType::KeyPress(key) => {
                    let key_str = format!("{:?}", key);
                    let is_active = ACTIVE_RDEV_KEYS
                        .read()
                        .map(|s| s.contains(&key_str))
                        .unwrap_or(false);
                    if is_active {
                        let msg = serde_json::json!({ "event": "key-down", "key": key_str }).to_string();
                        let _ = KEY_SENDER.send(msg);
                    }
                }
                rdev::EventType::KeyRelease(key) => {
                    let key_str = format!("{:?}", key);
                    let is_active = ACTIVE_RDEV_KEYS
                        .read()
                        .map(|s| s.contains(&key_str))
                        .unwrap_or(false);
                    if is_active {
                        let msg = serde_json::json!({ "event": "key-up", "key": key_str }).to_string();
                        let _ = KEY_SENDER.send(msg);
                    }
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
    if let Ok(mut active) = ACTIVE_KEYS.write() {
        *active = keys.to_vec();
    }
    sync_rdev_set(keys);
}
