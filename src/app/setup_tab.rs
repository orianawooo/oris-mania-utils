#[cfg(target_os = "windows")]
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{self, Color32, Rounding, Stroke};

use crate::bootstrap::{detect_osu_songs_path, is_valid_tosu_root};
use crate::calc::rebuild_songs_index;
use crate::theme::ThemeScope;
use crate::tosu_manager::{
    download_latest_daniel, download_latest_minacalc, install_tosu_core, launch_tosu_process,
    managed_tosu_dir_standalone, read_tosu_env_network, restart_tosu_process, stop_tosu_process,
};
#[cfg(target_os = "windows")]
use crate::tosu_manager::{ensure_tosu_overlay_env, push_tosu_runtime_settings};

use super::widgets::{
    COLOR_BG_CARD, COLOR_BG_DEEP, COLOR_BG_SURFACE, COLOR_BORDER, COLOR_TEXT_MUTED,
    COLOR_TEXT_PRIMARY, COLOR_TEXT_SECONDARY, COLOR_WARN,
};
use super::App;

impl App {
    pub(super) fn ui_setup_tab(&mut self, ui: &mut egui::Ui, accent: Color32) {
        egui::ScrollArea::vertical()
            .id_salt("setup_cfg_scroll")
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    egui::Frame::none()
                        .fill(COLOR_BG_CARD)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("osu! songs directory")
                                    .size(14.0)
                                    .strong()
                                    .color(COLOR_TEXT_PRIMARY),
                            );
                            ui.label(
                                egui::RichText::new(
                                    "used by minacalc 515 to parse .osu beatmaps in real time",
                                )
                                .size(11.5)
                                .color(COLOR_TEXT_MUTED),
                            );
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                let path_display = if self.config.osu_songs_path.is_empty() {
                                    "not configured".to_string()
                                } else {
                                    self.config.osu_songs_path.clone()
                                };

                                egui::Frame::none()
                                    .fill(COLOR_BG_SURFACE)
                                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                                    .rounding(Rounding::same(6.0))
                                    .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new(path_display)
                                                .monospace()
                                                .size(11.5)
                                                .color(COLOR_TEXT_PRIMARY),
                                        );
                                    });

                                if ui.button("browse").clicked() {
                                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                        self.config.osu_songs_path = folder.to_string_lossy().to_string();
                                        rebuild_songs_index(&self.config.osu_songs_path);
                                        self.mark_dirty();
                                        self.set_toast("songs folder updated");
                                    }
                                }

                                if ui.button("auto-detect").clicked() {
                                    if let Some(detected) = detect_osu_songs_path() {
                                        self.config.osu_songs_path = detected.to_string_lossy().to_string();
                                        rebuild_songs_index(&self.config.osu_songs_path);
                                        self.mark_dirty();
                                        self.set_toast("osu! songs folder detected");
                                    } else {
                                        self.set_toast("could not auto-detect songs folder");
                                    }
                                }
                            });
                        });

                    ui.add_space(12.0);

                    egui::Frame::none()
                        .fill(COLOR_BG_CARD)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            let latest_tag = self
                                .tosu_latest_ver
                                .lock()
                                .ok()
                                .and_then(|g| g.clone())
                                .unwrap_or_default();
                            let update_avail = !latest_tag.is_empty()
                                && !self.tosu_installed_ver.is_empty()
                                && self.tosu_installed_ver != latest_tag;

                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("tosu memory reader")
                                        .size(14.0)
                                        .strong()
                                        .color(COLOR_TEXT_PRIMARY),
                                );
                                if !self.tosu_installed_ver.is_empty() {
                                    ui.label(
                                        egui::RichText::new(&self.tosu_installed_ver)
                                            .monospace()
                                            .size(11.0)
                                            .color(COLOR_TEXT_MUTED),
                                    );
                                }
                                if !latest_tag.is_empty() {
                                    let tag_text = if update_avail {
                                        format!("update available: {}", latest_tag)
                                    } else {
                                        format!("latest: {}", latest_tag)
                                    };
                                    ui.label(
                                        egui::RichText::new(tag_text)
                                            .monospace()
                                            .size(11.0)
                                            .color(if update_avail { COLOR_WARN } else { COLOR_TEXT_MUTED }),
                                    );
                                }
                            });
                            ui.label(
                                egui::RichText::new(format!(
                                    "reads osu! state safely over websocket ({}:{})",
                                    self.config.tosu_host,
                                    self.config.tosu_port.max(1024)
                                ))
                                .size(11.5)
                                .color(COLOR_TEXT_MUTED),
                            );
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                if !self.tosu_installed || update_avail {
                                    let btn_label = if update_avail {
                                        "update tosu"
                                    } else {
                                        "install tosu"
                                    };
                                    let install_btn = ui.add_enabled(
                                        !self.installing,
                                        egui::Button::new(
                                            egui::RichText::new(btn_label).color(COLOR_BG_DEEP).strong(),
                                        )
                                        .fill(accent)
                                        .rounding(Rounding::same(6.0)),
                                    );
                                    if install_btn.clicked() {
                                        self.installing = true;
                                        let progress_arc = Arc::clone(&self.install_progress);
                                        let target_dir = if !self.config.tosu_root_path.is_empty() {
                                            PathBuf::from(&self.config.tosu_root_path)
                                        } else {
                                            managed_tosu_dir_standalone()
                                        };
                                        std::thread::spawn(move || {
                                            if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                                                .enable_all()
                                                .build()
                                            {
                                                let p_clone = Arc::clone(&progress_arc);
                                                let _ = rt.block_on(install_tosu_core(target_dir, move |p| {
                                                    if let Ok(mut lock) = p_clone.lock() {
                                                        *lock = Some(p);
                                                    }
                                                }));
                                            }
                                        });
                                    }
                                }

                                if self.tosu_installed && !self.tosu_running {
                                    let start_btn = ui.add(
                                        egui::Button::new(
                                            egui::RichText::new("start tosu").color(COLOR_BG_DEEP).strong(),
                                        )
                                        .fill(accent)
                                        .rounding(Rounding::same(6.0)),
                                    );
                                    if start_btn.clicked() {
                                        let root = if !self.config.tosu_root_path.is_empty() {
                                            PathBuf::from(&self.config.tosu_root_path)
                                        } else {
                                            managed_tosu_dir_standalone()
                                        };
                                        match launch_tosu_process(&root) {
                                            Ok(_) => {
                                                self.tosu_running = true;
                                                self.set_toast("tosu started");
                                            }
                                            Err(err) => self.set_toast(&err),
                                        }
                                    }
                                } else if self.tosu_running {
                                    if ui.button("restart tosu").clicked() {
                                        let root = if !self.config.tosu_root_path.is_empty() {
                                            PathBuf::from(&self.config.tosu_root_path)
                                        } else {
                                            managed_tosu_dir_standalone()
                                        };
                                        match restart_tosu_process(&root) {
                                            Ok(_) => {
                                                self.poll_tosu_status();
                                                self.set_toast("tosu restarted");
                                            }
                                            Err(err) => self.set_toast(&err),
                                        }
                                    }
                                    if ui.button("stop managed tosu").clicked() {
                                        stop_tosu_process();
                                        self.poll_tosu_status();
                                    }
                                }

                                if ui.button("browse tosu folder").clicked() {
                                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                        if is_valid_tosu_root(&folder) {
                                            self.config.tosu_root_path =
                                                folder.to_string_lossy().to_string();
                                            let (env_host, env_port) = read_tosu_env_network(&folder);
                                            if let Some(h) = env_host {
                                                self.config.tosu_host = h;
                                            }
                                            if let Some(p) = env_port {
                                                self.config.tosu_port = p;
                                            }
                                            self.mark_dirty();
                                            self.sync_overlays();
                                            self.poll_tosu_status();
                                        } else {
                                            self.set_toast("selected folder does not contain tosu");
                                        }
                                    }
                                }
                            });

                            ui.add_space(6.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new("host")
                                        .size(11.5)
                                        .color(COLOR_TEXT_SECONDARY),
                                );
                                let host_resp = ui.add(
                                    egui::TextEdit::singleline(&mut self.config.tosu_host)
                                        .desired_width(110.0)
                                        .font(egui::TextStyle::Monospace),
                                );
                                if host_resp.changed() {
                                    self.config.tosu_host =
                                        crate::config::normalize_host_str(&self.config.tosu_host);
                                    self.mark_dirty();
                                }

                                ui.add_space(8.0);
                                ui.label(
                                    egui::RichText::new("tosu port")
                                        .size(11.5)
                                        .color(COLOR_TEXT_SECONDARY),
                                );
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut self.config.tosu_port)
                                            .range(1024..=65535)
                                            .speed(1.0),
                                    )
                                    .changed()
                                {
                                    if self.config.companion_port == self.config.tosu_port {
                                        self.config.companion_port =
                                            self.config.tosu_port.saturating_add(1).max(1025);
                                    }
                                    self.mark_dirty();
                                }

                                ui.add_space(8.0);
                                ui.label(
                                    egui::RichText::new("companion ws port")
                                        .size(11.5)
                                        .color(COLOR_TEXT_SECONDARY),
                                );
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut self.config.companion_port)
                                            .range(1024..=65535)
                                            .speed(1.0),
                                    )
                                    .changed()
                                {
                                    if self.config.companion_port == self.config.tosu_port {
                                        self.config.companion_port =
                                            self.config.tosu_port.saturating_add(1).max(1025);
                                    }
                                    self.mark_dirty();
                                }
                            });

                            ui.add_space(6.0);
                            ui.horizontal_wrapped(|ui| {
                                if ui
                                    .checkbox(
                                        &mut self.config.show_tosu_console,
                                        "show tosu console window (debug)",
                                    )
                                    .changed()
                                {
                                    self.mark_dirty();
                                    self.set_toast(if self.tosu_running {
                                        "restart tosu to apply console visibility"
                                    } else {
                                        "tosu console setting saved"
                                    });
                                }
                                #[cfg(target_os = "windows")]
                                if ui
                                    .checkbox(
                                        &mut self.config.enable_tosu_ingame_overlay,
                                        "auto-enable tosu in-game overlay",
                                    )
                                    .changed()
                                {
                                    self.mark_dirty();
                                    if !self.config.tosu_root_path.is_empty() {
                                        ensure_tosu_overlay_env(Path::new(&self.config.tosu_root_path));
                                    }
                                    push_tosu_runtime_settings(self.config.tosu_port.max(1024), &self.config);
                                    self.set_toast("tosu overlay setting updated");
                                }
                            });

                            if self.installing {
                                if let Ok(lock) = self.install_progress.lock() {
                                    if let Some(p) = &*lock {
                                        ui.add_space(6.0);
                                        ui.label(
                                            egui::RichText::new(&p.message)
                                                .size(11.0)
                                                .color(COLOR_TEXT_SECONDARY),
                                        );
                                    }
                                }
                            }
                        });

                    ui.add_space(12.0);

                    egui::Frame::none()
                        .fill(COLOR_BG_CARD)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("minacalc & daniel (xdc1) engines")
                                        .size(14.0)
                                        .strong()
                                        .color(COLOR_TEXT_PRIMARY),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "minacalc: {} · dan: {}",
                                        self.minacalc_ver, self.daniel_ver
                                    ))
                                    .monospace()
                                    .size(11.0)
                                    .color(COLOR_TEXT_MUTED),
                                );
                            });
                            ui.label(
                                egui::RichText::new(
                                    "fetch latest upstream minacalc (etternagame/minacalc) and daniel xdc1 (TheBagelOfMan/Daniel + companella onnx)",
                                )
                                .size(11.5)
                                .color(COLOR_TEXT_MUTED),
                            );
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                let mc_btn = ui.add_enabled(
                                    !self.downloading_engine,
                                    egui::Button::new(
                                        egui::RichText::new("download latest minacalc")
                                            .color(COLOR_BG_DEEP)
                                            .strong(),
                                    )
                                    .fill(accent)
                                    .rounding(Rounding::same(6.0)),
                                );
                                if mc_btn.clicked() {
                                    self.downloading_engine = true;
                                    let msg_arc = Arc::clone(&self.engine_dl_msg);
                                    std::thread::spawn(move || {
                                        if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                                            .enable_all()
                                            .build()
                                        {
                                            let res = rt.block_on(download_latest_minacalc());
                                            if let Ok(mut lock) = msg_arc.lock() {
                                                *lock = Some(match res {
                                                    Ok(v) => format!("minacalc updated to {}", v),
                                                    Err(e) => format!("minacalc download failed: {}", e),
                                                });
                                            }
                                        }
                                    });
                                }

                                let dan_btn = ui.add_enabled(
                                    !self.downloading_engine,
                                    egui::Button::new(
                                        egui::RichText::new("download latest xdc1 & dan model")
                                            .color(COLOR_TEXT_PRIMARY)
                                            .strong(),
                                    )
                                    .fill(COLOR_BG_SURFACE)
                                    .stroke(Stroke::new(1.0_f32, accent))
                                    .rounding(Rounding::same(6.0)),
                                );
                                if dan_btn.clicked() {
                                    self.downloading_engine = true;
                                    let msg_arc = Arc::clone(&self.engine_dl_msg);
                                    std::thread::spawn(move || {
                                        if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                                            .enable_all()
                                            .build()
                                        {
                                            let res = rt.block_on(download_latest_daniel());
                                            if let Ok(mut lock) = msg_arc.lock() {
                                                *lock = Some(match res {
                                                    Ok(v) => format!("daniel xdc1 + model updated to {}", v),
                                                    Err(e) => format!("daniel download failed: {}", e),
                                                });
                                            }
                                        }
                                    });
                                }
                            });
                        });

                    ui.add_space(12.0);

                    egui::Frame::none()
                        .fill(COLOR_BG_CARD)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("platform & browser overlays")
                                    .size(14.0)
                                    .strong()
                                    .color(COLOR_TEXT_PRIMARY),
                            );
                            let ws_p = crate::config::runtime_active_companion_port();
                            ui.label(
                                egui::RichText::new(if cfg!(target_os = "linux") {
                                    format!(
                                        "linux uses native wayland/x11 overlays; browser sources & ws :{ws_p} stay off by default to save cpu/ram"
                                    )
                                } else {
                                    format!(
                                        "windows feeds tosu in-game & obs browser sources via ws :{ws_p} and static overlay files"
                                    )
                                })
                                .size(11.5)
                                .color(COLOR_TEXT_MUTED),
                            );
                            ui.add_space(6.0);

                            let cb_label = format!(
                                "enable browser overlays (obs / ws :{ws_p} / static sync)"
                            );
                            if ui
                                .checkbox(&mut self.config.enable_browser_overlays, cb_label)
                                .changed()
                            {
                                self.mark_dirty();
                                if self.config.enable_browser_overlays {
                                    crate::keys::ensure_ws_server_started();
                                    self.sync_overlays();
                                } else {
                                    self.set_toast("browser overlays disabled (native overlays active)");
                                }
                            }

                            if self.config.enable_browser_overlays {
                                let host = &self.config.tosu_host;
                                let p = self.config.tosu_port.max(1024);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "urls: http://{host}:{p}/radar/ · /keys/ · /hits/",
                                    ))
                                    .monospace()
                                    .size(11.0)
                                    .color(COLOR_TEXT_MUTED),
                                );
                            }

                            ui.add_space(4.0);
                            #[cfg(target_os = "linux")]
                            {
                                let mut dummy = false;
                                ui.add_enabled(
                                    false,
                                    egui::Checkbox::new(
                                        &mut dummy,
                                        "tosu! dx/opengl in-game overlay hook (windows only — unavailable on linux)",
                                    ),
                                );
                            }
                            #[cfg(target_os = "windows")]
                            {
                                let mut dummy = false;
                                ui.add_enabled(
                                    false,
                                    egui::Checkbox::new(
                                        &mut dummy,
                                        "wayland wlr-layer-shell / x11 override_redirect backend (linux only)",
                                    ),
                                );
                            }
                        });

                    ui.add_space(12.0);

                    egui::Frame::none()
                        .fill(COLOR_BG_CARD)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(if cfg!(target_os = "linux") {
                                    "native floating overlays (wayland / x11)"
                                } else {
                                    "floating overlay windows (windows)"
                                })
                                .size(14.0)
                                .strong()
                                .color(COLOR_TEXT_PRIMARY),
                            );
                            ui.label(
                                egui::RichText::new(
                                    "spawn transparent always-on-top overlays directly without browser sources; drag when unlocked, click-through when locked",
                                )
                                .size(11.5)
                                .color(COLOR_TEXT_MUTED),
                            );
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                if ui.selectable_label(self.float_keys, "float keys").clicked() {
                                    self.float_keys = !self.float_keys;
                                    self.config.float_keys_open = self.float_keys;
                                    self.mark_dirty();
                                }
                                if ui.selectable_label(self.float_radar, "float radar").clicked() {
                                    self.float_radar = !self.float_radar;
                                    self.config.float_radar_open = self.float_radar;
                                    self.mark_dirty();
                                }
                                if ui.selectable_label(self.float_hits, "float hits").clicked() {
                                    self.float_hits = !self.float_hits;
                                    self.config.float_hits_open = self.float_hits;
                                    self.mark_dirty();
                                }
                                ui.separator();
                                if ui
                                    .checkbox(
                                        &mut self.config.lock_overlays,
                                        "lock overlays (click-through)",
                                    )
                                    .changed()
                                {
                                    self.mark_dirty();
                                }
                            });
                        });

                    ui.add_space(12.0);

                    egui::Frame::none()
                        .fill(COLOR_BG_CARD)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("themes & full backup (export / import)")
                                    .size(14.0)
                                    .strong()
                                    .color(COLOR_TEXT_PRIMARY),
                            );
                            ui.label(
                                egui::RichText::new(
                                    "share normalized themes or save backward-compatible full config snapshots across updates",
                                )
                                .size(11.5)
                                .color(COLOR_TEXT_MUTED),
                            );
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui.button("export backup (.json)").clicked() {
                                    self.export_backup_dialog();
                                }
                                if ui.button("import backup (.json)").clicked() {
                                    self.import_backup_dialog();
                                }
                                ui.separator();
                                if ui.button("export theme (.json)").clicked() {
                                    self.export_theme_dialog(ThemeScope::All);
                                }
                                if ui.button("import theme (.json)").clicked() {
                                    self.import_theme_dialog(ThemeScope::All);
                                }
                            });
                        });

                    ui.add_space(12.0);

                    egui::Frame::none()
                        .fill(COLOR_BG_CARD)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .checkbox(
                                        &mut self.config.unsafe_numbers,
                                        "enable unsafe numbers (unbounded px & scale limits)",
                                    )
                                    .changed()
                                {
                                    self.mark_dirty();
                                }
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("reset all overlays").clicked() {
                                        self.config.reset_keys();
                                        self.config.reset_radar();
                                        self.config.reset_hits();
                                        self.mark_dirty();
                                        self.set_toast("all overlay settings reset to defaults");
                                    }
                                });
                            });
                            if self.config.unsafe_numbers {
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new(
                                        "warning: unbounded px and scale values can move overlays off-screen or increase memory usage",
                                    )
                                    .size(11.5)
                                    .color(COLOR_WARN),
                                );
                            }
                        });
                });
            });
    }
}
