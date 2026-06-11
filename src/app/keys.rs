//! Keyboard input and focus control handling.
//!
//! **Taxonomy Classification**: Interface (Keyboard & Focus).

use crate::app::{App, FocusedSection, GlobalField, StatusMessage, StatusKind};
use crate::config::GlobalConfig;
use crossterm::event::{KeyCode, KeyModifiers};

impl App {
    /// Adjust the highlight in the saver list, clamping to bounds.
    pub fn move_highlight(&mut self, delta: i32) {
        let indices = self.filtered_indices();
        if indices.is_empty() {
            return;
        }
        let current_pos = indices
            .iter()
            .position(|&i| i == self.highlighted)
            .unwrap_or(0);
        let len = indices.len() as i32;
        let next = (current_pos as i32 + delta).rem_euclid(len);
        self.highlighted = indices[next as usize];
    }

    /// Cycle the focused section.
    pub fn cycle_focus(&mut self) {
        self.focused = match self.focused {
            FocusedSection::GlobalPrefs => FocusedSection::SaverList,
            FocusedSection::SaverList => FocusedSection::GlobalPrefs,
        };
        self.status = Some(StatusMessage {
            text: format!("Focused Section: {}", match self.focused {
                FocusedSection::GlobalPrefs => "Global Preferences",
                FocusedSection::SaverList => "Screensaver List",
            }),
            kind: StatusKind::Info,
        });
    }

    /// Move focus / highlight depending on direction.
    pub fn move_focus(&mut self, delta: i32) {
        match self.focused {
            FocusedSection::GlobalPrefs => {
                let idx = GlobalField::ALL
                    .iter()
                    .position(|f| *f == self.global_field)
                    .unwrap_or(0) as i32;
                let len = GlobalField::ALL.len() as i32;
                let next = (idx + delta).rem_euclid(len);
                self.global_field = GlobalField::ALL[next as usize];
            }
            FocusedSection::SaverList => self.move_highlight(delta),
        }
    }

    /// Handle a single key event. Returns `true` if the app should quit.
    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        // Clear any error status on any user keypress. Info status remains subject to the timer.
        if let Some(ref msg) = self.status {
            if msg.kind == StatusKind::Error {
                self.status = None;
            }
        }

        if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
            return true;
        }

        if self.show_help {
            match code {
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc | KeyCode::Char('h') | KeyCode::Char('H') => {
                    self.show_help = false;
                    self.status = Some(StatusMessage {
                        text: "Help overlay closed.".to_string(),
                        kind: StatusKind::Info,
                    });
                }
                _ => {
                    // F1..F7 -> open a different doc, closing the help overlay
                    if let Some(name) = library::apps::chrome::open_embedded_markdown(code) {
                        self.show_help = false;
                        self.open_embedded_markdown(name, doc_content(name));
                    }
                }
            }
            return false;
        }

        if self.show_markdown.is_some() {
            // F1..F7 -> swap to a different doc
            if let Some(name) = library::apps::chrome::open_embedded_markdown(code) {
                self.open_embedded_markdown(name, doc_content(name));
                return false;
            }
            // Up/Down/PageUp/PageDown -> scroll the markdown
            if let Some(new_scroll) = library::apps::chrome::scroll_for_key(
                code,
                self.markdown_scroll,
                self.markdown_lines.len(),
                10,
            ) {
                self.markdown_scroll = new_scroll;
                return false;
            }
            // Esc/q close the viewer
            if matches!(code, KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc) {
                self.show_markdown = None;
            }
            return false;
        }

        match code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char('r') | KeyCode::Char('R') => self.refresh_screensavers(),
            KeyCode::Tab => self.cycle_focus(),
            KeyCode::BackTab => self.cycle_focus(),
            KeyCode::Up => self.move_focus(-1),
            KeyCode::Down => self.move_focus(1),
            KeyCode::Left => self.on_left(),
            KeyCode::Right => self.on_right(),
            KeyCode::Char(' ') => {
                self.on_activate();
            }
            KeyCode::Enter => self.on_activate(),
            KeyCode::Char('p') | KeyCode::Char('P') | KeyCode::Char('t') | KeyCode::Char('T') => {
                self.preview_highlighted()
            }
            KeyCode::Char('c') | KeyCode::Char('C') => self.configure_highlighted(),
            KeyCode::F(1..=7) => {
                if let Some(name) = library::apps::chrome::open_embedded_markdown(code) {
                    self.open_embedded_markdown(name, doc_content(name));
                }
            }
            KeyCode::Char('h') | KeyCode::Char('H') => {
                self.show_help = true;
                self.status = Some(StatusMessage {
                    text: "Help overlay active. Press ESC/q to close.".to_string(),
                    kind: StatusKind::Info,
                });
            }
            _ => {}
        }
        self.should_quit
    }

    fn on_left(&mut self) {
        if self.focused == FocusedSection::GlobalPrefs {
            match self.global_field {
                GlobalField::Timeout => self.adjust_timeout(-1),
                _ => {}
            }
        }
    }

    fn on_right(&mut self) {
        if self.focused == FocusedSection::GlobalPrefs {
            match self.global_field {
                GlobalField::Timeout => self.adjust_timeout(1),
                _ => {}
            }
        }
    }

    fn on_activate(&mut self) {
        match self.focused {
            FocusedSection::GlobalPrefs => match self.global_field {
                GlobalField::Active => self.toggle_active(),
                GlobalField::PreventSleep => self.toggle_prevent_sleep(),
                GlobalField::HideStock => self.toggle_hide_stock(),
                GlobalField::Timeout => {}
            },
            FocusedSection::SaverList => self.apply_highlighted(),
        }
    }

    /// Check if the registry matches the global config, reload if out of sync.
    pub fn check_registry_sync(&mut self) -> bool {
        let current_reg = GlobalConfig::load();
        if current_reg.active_scr != self.global.active_scr
            || current_reg.active != self.global.active
            || current_reg.timeout != self.global.timeout
        {
            self.global = current_reg;
            self.status = Some(StatusMessage {
                text: "External registry change detected! Config reloaded.".to_string(),
                kind: StatusKind::Info,
            });
            self.update_list_items();
            true
        } else {
            false
        }
    }
}

/// Resolve a doc filename (e.g. "README.md") to its embedded markdown content.
/// Each app embeds its own copy of the 7 docs at compile time, so this lookup
/// lives in the app crate (not in library).
fn doc_content(name: &str) -> &'static str {
    match name {
        "README.md" => super::README_CONTENT,
        "SUPPORT.md" => super::SUPPORT_CONTENT,
        "LICENSE.md" => super::LICENSE_CONTENT,
        "COPYRIGHT.md" => super::COPYRIGHT_CONTENT,
        "PRIVACY.md" => super::PRIVACY_CONTENT,
        "SECURITY.md" => super::SECURITY_CONTENT,
        "CONTRIBUTING.md" => super::CONTRIBUTING_CONTENT,
        _ => "",
    }
}
