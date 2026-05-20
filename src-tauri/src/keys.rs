use futures_util::SinkExt;
use futures_util::StreamExt;
use tauri::Emitter;

use crate::config::read_config;
use crate::KEY_SENDER;

pub async fn start_key_server(app: tauri::AppHandle) {
    let listener = match tokio::net::TcpListener::bind("127.0.0.1:24051").await {
        Ok(l) => l,
        Err(_) => return,
    };
    
    let app_clone = app.clone();
    std::thread::spawn(move || {
        if let Err(_) = rdev::listen(move |event| {
            match event.event_type {
                rdev::EventType::KeyPress(key) => {
                    let key_str = format!("{:?}", key);
                    let msg = serde_json::json!({ "event": "key-down", "key": key_str }).to_string();
                    let _ = KEY_SENDER.send(msg);
                    let _ = app_clone.emit("global-key", key_str);
                }
                rdev::EventType::KeyRelease(key) => {
                    let key_str = format!("{:?}", key);
                    let msg = serde_json::json!({ "event": "key-up", "key": key_str }).to_string();
                    let _ = KEY_SENDER.send(msg);
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
                let (mut ws_sender, _) = ws_stream.split();
                
                let config = read_config();
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
