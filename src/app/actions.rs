//! State mutation and action execution triggers.
//!
//! **Taxonomy Classification**: Interface (Action Handlers).

use crate::app::{App, StatusMessage, StatusKind};
use crate::backend::preview;

const TIMEOUT_STEP_SECS: u32 = 60;
const TIMEOUT_MIN_SECS: u32 = 60;
const TIMEOUT_MAX_SECS: u32 = 7200;

impl App {
    /// Apply the currently-highlighted screensaver as the system screensaver.
    pub fn apply_highlighted(&mut self) {
        let Some(s) = self.current_screensaver() else {
            self.global.active_scr = String::new();
            self.global.active = false;
            self.status = Some(StatusMessage {
                text: "Screensaver deactivated (turned off)".to_string(),
                kind: StatusKind::Info,
            });
            let _ = self.global.save();
            self.update_list_items();
            return;
        };

        let path = s.path.to_string_lossy().into_owned();
        let name = s.name.clone();
        let filename = s.path.file_name().and_then(|f| f.to_str()).map(|s| s.to_string());

        self.global.active_scr = path;
        self.global.active = true;

        self.status = Some(StatusMessage {
            text: format!("Applied: {}", name),
            kind: StatusKind::Info,
        });

        if let Err(e) = self.global.save() {
            self.status = Some(StatusMessage {
                text: format!("Failed to save: {e}"),
                kind: StatusKind::Error,
            });
            return;
        }

        if let Some(fname) = filename {
            self.local.last_selected = Some(fname);
        }
        let _ = self.local.save();
        self.update_list_items();
    }

    /// Toggle the global `active` flag in the registry.
    pub fn toggle_active(&mut self) {
        self.global.active = !self.global.active;
        match self.global.save() {
            Ok(()) => {
                self.status = Some(StatusMessage {
                    text: format!("Active = {}", self.global.active),
                    kind: StatusKind::Info,
                });
                self.update_list_items();
            }
            Err(e) => {
                self.status = Some(StatusMessage {
                    text: format!("Save failed: {e}"),
                    kind: StatusKind::Error,
                })
            }
        }
    }

    /// Toggle the "prevent system sleep" mode. The state lives in
    /// `LocalConfig` because it's a per-user preference, not a system one.
    pub fn toggle_prevent_sleep(&mut self) {
        self.local.prevent_sleep = !self.local.prevent_sleep;
        let filename = self.current_screensaver()
            .and_then(|s| s.path.file_name())
            .and_then(|f| f.to_str())
            .map(|s| s.to_string());
        if let Some(name) = filename {
            self.local.last_selected = Some(name);
        }
        match self.local.save() {
            Ok(()) => {
                self.status = Some(StatusMessage {
                    text: format!("Prevent sleep = {}", self.local.prevent_sleep),
                    kind: StatusKind::Info,
                })
            }
            Err(e) => {
                self.status = Some(StatusMessage {
                    text: format!("Save failed: {e}"),
                    kind: StatusKind::Error,
                })
            }
        }
    }

    /// Toggle hiding stock windows screensavers.
    pub fn toggle_hide_stock(&mut self) {
        self.local.hide_stock = !self.local.hide_stock;
        let filename = self.current_screensaver()
            .and_then(|s| s.path.file_name())
            .and_then(|f| f.to_str())
            .map(|s| s.to_string());
        if let Some(name) = filename {
            self.local.last_selected = Some(name);
        }
        match self.local.save() {
            Ok(()) => {
                self.resolve_highlight();
                self.status = Some(StatusMessage {
                    text: format!("Hide stock screensavers = {}", self.local.hide_stock),
                    kind: StatusKind::Info,
                });
                self.update_list_items();
            }
            Err(e) => {
                self.status = Some(StatusMessage {
                    text: format!("Save failed: {e}"),
                    kind: StatusKind::Error,
                })
            }
        }
    }

    /// Adjust the screensaver timeout by one step.
    pub fn adjust_timeout(&mut self, delta: i32) {
        let next = (self.global.timeout as i32 + delta * TIMEOUT_STEP_SECS as i32)
            .clamp(TIMEOUT_MIN_SECS as i32, TIMEOUT_MAX_SECS as i32) as u32;
        if next == self.global.timeout {
            return;
        }
        self.global.timeout = next;
        if let Err(e) = self.global.save() {
            self.status = Some(StatusMessage {
                text: format!("Save failed: {e}"),
                kind: StatusKind::Error,
            });
        }
    }

    /// Re-discover screensavers and refresh the list.
    pub fn refresh_screensavers(&mut self) {
        self.screensavers = preview::discover();
        self.resolve_highlight();
        self.status = Some(StatusMessage {
            text: "Refreshed screensavers list.".to_string(),
            kind: StatusKind::Info,
        });
        self.update_list_items();
    }

    /// Spawn the currently-highlighted screensaver fullscreen.
    pub fn preview_highlighted(&mut self) {
        let Some(s) = self.current_screensaver() else {
            return;
        };
        #[cfg(target_os = "windows")]
        let spawn_res = std::process::Command::new(&s.path).arg("/s").spawn();
        #[cfg(not(target_os = "windows"))]
        let spawn_res = crate::win32::spawn_linux_screensaver(&s.path, "/s");

        if let Err(e) = spawn_res {
            self.status = Some(StatusMessage {
                text: format!("Preview failed: {e}"),
                kind: StatusKind::Error,
            });
        }
    }

    /// Spawn the currently-highlighted screensaver's native configuration dialog.
    pub fn configure_highlighted(&mut self) {
        let Some(s) = self.current_screensaver() else {
            return;
        };
        if let Err(e) = std::process::Command::new(&s.path).arg("/c").spawn() {
            self.status = Some(StatusMessage {
                text: format!("Configure failed: {e}"),
                kind: StatusKind::Error,
            });
        } else {
            self.status = Some(StatusMessage {
                text: format!("Opened settings for {}", s.name),
                kind: StatusKind::Info,
            });
        }
    }
}
