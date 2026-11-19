use std::sync::atomic::Ordering;
use std::time::Duration;

use eframe::egui::{self, Key};

use crate::config::write_config;
use crate::keys::{end_key_capture, key_name_to_display_label, take_captured_key, LIVE_KEY_STATES};

use super::App;

const KEY_NAME_TABLE: &[(Key, &str)] = &[
    (Key::A, "KeyA"),
    (Key::B, "KeyB"),
    (Key::C, "KeyC"),
    (Key::D, "KeyD"),
    (Key::E, "KeyE"),
    (Key::F, "KeyF"),
    (Key::G, "KeyG"),
    (Key::H, "KeyH"),
    (Key::I, "KeyI"),
    (Key::J, "KeyJ"),
    (Key::K, "KeyK"),
    (Key::L, "KeyL"),
    (Key::M, "KeyM"),
    (Key::N, "KeyN"),
    (Key::O, "KeyO"),
    (Key::P, "KeyP"),
    (Key::Q, "KeyQ"),
    (Key::R, "KeyR"),
    (Key::S, "KeyS"),
    (Key::T, "KeyT"),
    (Key::U, "KeyU"),
    (Key::V, "KeyV"),
    (Key::W, "KeyW"),
    (Key::X, "KeyX"),
    (Key::Y, "KeyY"),
    (Key::Z, "KeyZ"),
    (Key::Num0, "Num0"),
    (Key::Num1, "Num1"),
    (Key::Num2, "Num2"),
    (Key::Num3, "Num3"),
    (Key::Num4, "Num4"),
    (Key::Num5, "Num5"),
    (Key::Num6, "Num6"),
    (Key::Num7, "Num7"),
    (Key::Num8, "Num8"),
    (Key::Num9, "Num9"),
    (Key::F1, "F1"),
    (Key::F2, "F2"),
    (Key::F3, "F3"),
    (Key::F4, "F4"),
    (Key::F5, "F5"),
    (Key::F6, "F6"),
    (Key::F7, "F7"),
    (Key::F8, "F8"),
    (Key::F9, "F9"),
    (Key::F10, "F10"),
    (Key::F11, "F11"),
    (Key::F12, "F12"),
    (Key::Space, "Space"),
    (Key::Enter, "Return"),
    (Key::Tab, "Tab"),
    (Key::Backspace, "Backspace"),
    (Key::Insert, "Insert"),
    (Key::Delete, "Delete"),
    (Key::Home, "Home"),
    (Key::End, "End"),
    (Key::PageUp, "PageUp"),
    (Key::PageDown, "PageDown"),
    (Key::Minus, "Minus"),
    (Key::Equals, "Equal"),
    (Key::OpenBracket, "LeftBracket"),
    (Key::CloseBracket, "RightBracket"),
    (Key::Semicolon, "SemiColon"),
    (Key::Quote, "Quote"),
    (Key::Backtick, "BackQuote"),
    (Key::Comma, "Comma"),
    (Key::Period, "Dot"),
    (Key::Slash, "Slash"),
    (Key::Backslash, "BackSlash"),
    (Key::ArrowLeft, "LeftArrow"),
    (Key::ArrowRight, "RightArrow"),
    (Key::ArrowUp, "UpArrow"),
    (Key::ArrowDown, "DownArrow"),
];

pub fn egui_key_to_name(k: Key) -> Option<String> {
    KEY_NAME_TABLE
        .iter()
        .find(|&&(key, _)| key == k)
        .map(|&(_, name)| name.to_string())
}

pub fn egui_key_from_name(s: &str) -> Option<Key> {
    let low = s.trim().to_lowercase();
    let stripped = low.strip_prefix("key").unwrap_or(&low);
    let normalized = match stripped {
        "0" | "digit0" => "num0",
        "1" | "digit1" => "num1",
        "2" | "digit2" => "num2",
        "3" | "digit3" => "num3",
        "4" | "digit4" => "num4",
        "5" | "digit5" => "num5",
        "6" | "digit6" => "num6",
        "7" | "digit7" => "num7",
        "8" | "digit8" => "num8",
        "9" | "digit9" => "num9",
        "enter" => "return",
        "equals" => "equal",
        "openbracket" => "leftbracket",
        "closebracket" => "rightbracket",
        "period" => "dot",
        "arrowleft" => "leftarrow",
        "arrowright" => "rightarrow",
        "arrowup" => "uparrow",
        "arrowdown" => "downarrow",
        other => other,
    };
    KEY_NAME_TABLE.iter().find_map(|&(key, name)| {
        let bare = name.to_ascii_lowercase();
        let short = bare.strip_prefix("key").unwrap_or(&bare);
        (short == normalized || bare == normalized).then_some(key)
    })
}

pub fn key_name_to_display(s: &str) -> String {
    key_name_to_display_label(s)
}

impl App {
    pub(super) fn handle_input(&mut self, ctx: &egui::Context) {
        if let Some(rebind_idx) = self.key_rebind_idx {
            if let Some((key_name, rdev_label)) = take_captured_key() {
                if key_name == "__CANCEL__" || key_name.eq_ignore_ascii_case("escape") {
                    end_key_capture();
                    self.key_rebind_idx = None;
                    ctx.request_repaint();
                    return;
                }
                if rebind_idx < self.config.keys.len() {
                    self.config.keys[rebind_idx] = key_name;
                    if rebind_idx < self.config.key_labels.len() {
                        self.config.key_labels[rebind_idx] = rdev_label.clone();
                    }
                    crate::keys::update_active_keys(&self.config.keys);
                    self.mark_dirty();
                    write_config(&self.config);
                    self.set_toast(&format!("key {} bound to {}", rebind_idx + 1, rdev_label));
                }
                self.key_rebind_idx = None;
                ctx.request_repaint();
                return;
            }

            let mut detected_key: Option<String> = None;
            let mut text_label: Option<String> = None;
            let mut cancelled = false;
            ctx.input(|i| {
                for event in &i.events {
                    match event {
                        egui::Event::Key {
                            key: egui::Key::Escape,
                            pressed: true,
                            ..
                        } => {
                            cancelled = true;
                            break;
                        }
                        egui::Event::Key {
                            key, pressed: true, ..
                        } => {
                            if detected_key.is_none() {
                                detected_key = egui_key_to_name(*key);
                            }
                        }
                        egui::Event::Text(txt) => {
                            let clean = txt.trim();
                            if !clean.is_empty() && clean.chars().all(|c| !c.is_control()) {
                                text_label = Some(clean.to_uppercase());
                            }
                        }
                        _ => {}
                    }
                }
            });

            if cancelled {
                end_key_capture();
                self.key_rebind_idx = None;
                ctx.request_repaint();
                return;
            }

            if let Some(key_name) = detected_key {
                end_key_capture();
                if rebind_idx < self.config.keys.len() {
                    let display = text_label.unwrap_or_else(|| key_name_to_display(&key_name));
                    self.config.keys[rebind_idx] = key_name;
                    if rebind_idx < self.config.key_labels.len() {
                        self.config.key_labels[rebind_idx] = display.clone();
                    }
                    crate::keys::update_active_keys(&self.config.keys);
                    self.mark_dirty();
                    write_config(&self.config);
                    self.set_toast(&format!("key {} bound to {}", rebind_idx + 1, display));
                }
                self.key_rebind_idx = None;
                ctx.request_repaint();
                return;
            }
            ctx.request_repaint_after(Duration::from_millis(16));
        }

        let mut repainted = false;
        for (i, key_str) in self.config.keys.iter().enumerate().take(4) {
            let is_down = egui_key_from_name(key_str)
                .map(|k| ctx.input(|inp| inp.key_down(k)))
                .unwrap_or(false);

            if is_down != self.key_pressed[i] && !LIVE_KEY_STATES[i].load(Ordering::Relaxed) {
                self.key_pressed[i] = is_down;
                repainted = true;
            }
        }

        if repainted {
            ctx.request_repaint();
        }
    }
}
