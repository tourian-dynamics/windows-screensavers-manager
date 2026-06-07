//! Application state, focus, and key bindings.
//!
//! # Model-Render Split
//! rIdle uses a strict Model-Render architectural split:
//!
//! * **Model (`app.rs`)**: Owns all the application state, configuration structures,
//!   selection metrics, event handling, and mutations. It is completely decoupled from
//!   direct drawing operations and does not import or know about specific rendering layouts.
//! * **Render (`ui.rs`)**: Responsible for presenting the state stored in `App` onto a
//!   Ratatui `Frame`. It is a pure visual mapping from the current state to the terminal screen.
//!
//! This ensures that the state logic can be easily tested in isolation without having to
//! mock terminal drawing frames or deal with layout constraints.

use std::path::PathBuf;

use crate::config::{GlobalConfig, LocalConfig};
use crate::preview::Screensaver;
use crate::theme::TuiTheme;

const README_CONTENT: &str = include_str!("../README.md");
const SUPPORT_CONTENT: &str = include_str!("../SUPPORT.md");
const LICENSE_CONTENT: &str = include_str!("../LICENSE.md");
const COPYRIGHT_CONTENT: &str = include_str!("../COPYRIGHT.md");
const PRIVACY_CONTENT: &str = include_str!("../PRIVACY.md");
const SECURITY_CONTENT: &str = include_str!("../SECURITY.md");
const CONTRIBUTING_CONTENT: &str = include_str!("../CONTRIBUTING.md");

const TIMEOUT_STEP_SECS: u32 = 60;
const TIMEOUT_MIN_SECS: u32 = 60;
const TIMEOUT_MAX_SECS: u32 = 7200;

const CYCLE_TIME_STEP_SECS: u32 = 5;
const CYCLE_TIME_MIN_SECS: u32 = 5;
const CYCLE_TIME_MAX_SECS: u32 = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedSection {
    GlobalPrefs,
    SaverList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalField {
    Active,
    Timeout,
    PreventSleep,
    CycleTime,
    HideStock,
}

impl GlobalField {
    pub const ALL: &[GlobalField] = &[
        GlobalField::Active,
        GlobalField::Timeout,
        GlobalField::PreventSleep,
        GlobalField::CycleTime,
        GlobalField::HideStock,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(feature = "downloader")]
pub enum PendingAction {
    Apply,
    ToggleSelection,
    Preview,
    Configure,
    ToggleAndApply,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusMessage {
    pub text: String,
    pub kind: StatusKind,
}

pub struct App {
    pub screensavers: Vec<Screensaver>,
    pub highlighted: usize,
    pub focused: FocusedSection,
    pub global_field: GlobalField,
    pub global: GlobalConfig,
    pub local: LocalConfig,
    pub theme: TuiTheme,
    pub status: Option<StatusMessage>,
    pub should_quit: bool,
    pub list_offset: usize,
    /// Cached list items for rendering the screensavers list.
    pub list_items: Vec<ratatui::widgets::ListItem<'static>>,
    pub visual_progress: f64,
    #[cfg(feature = "downloader")]
    pub download_state: Option<std::sync::Arc<std::sync::Mutex<crate::downloader::DownloadState>>>,
    #[cfg(feature = "downloader")]
    pub registry_results: Option<std::sync::Arc<std::sync::Mutex<Option<Vec<crate::downloader::RegistryEntry>>>>>,
    #[cfg(feature = "downloader")]
    pub registry_entries: Vec<crate::downloader::RegistryEntry>,
    #[cfg(feature = "downloader")]
    pub pending_action: Option<PendingAction>,
    pub show_help: bool,
    pub selection_start: Option<(u16, u16)>,
    pub selection_end: Option<(u16, u16)>,
    pub selection_pending_copy: bool,
    pub show_markdown: Option<String>,
    pub markdown_lines: Vec<ratatui::text::Line<'static>>,
    pub markdown_scroll: usize,
    /// Loaded terminal character fallbacks (Adaptive Emoji/Glyph fallback)
    pub glyphs: crate::win32::GlyphMap,
    /// Whether the computer is currently running on battery power (Throttling)
    pub on_battery: bool,
    /// Last Instant the power/battery status was queried
    pub last_power_check: std::time::Instant,
    pub quit_btn_bounds: Option<(u16, u16, u16)>,
    pub help_btn_bounds: Option<(u16, u16, u16)>,
    pub drag_active: bool,
    pub drag_start_cursor: Option<(i32, i32)>,
    pub drag_start_window: Option<(i32, i32)>,
}

impl App {
    pub fn new(
        screensavers: Vec<Screensaver>,
        global: GlobalConfig,
        local: LocalConfig,
        theme: TuiTheme,
    ) -> Self {
        let mut local = local;
        if local.hide_stock {
            // If stock screensavers are hidden, they should not be active in the cycle.
            let orig_len = local.selected_paths.len();
            local.selected_paths.retain(|p| {
                !crate::preview::is_stock_screensaver(std::path::Path::new(p))
            });
            if local.selected_paths.len() != orig_len {
                let _ = local.save();
            }
        }

        let highlighted = local
            .last_selected
            .as_deref()
            .and_then(|name| {
                screensavers
                    .iter()
                    .position(|s| s.path.file_name().and_then(|f| f.to_str()) == Some(name))
            })
            .unwrap_or(0)
            .min(screensavers.len().saturating_sub(1));

        #[cfg(feature = "downloader")]
        let registry_results = {
            let state = std::sync::Arc::new(std::sync::Mutex::new(None));
            let thread_state = state.clone();
            let feed_urls = local.feed_urls.clone();
            std::thread::spawn(move || {
                let mut all_entries = Vec::new();

                // Load local registry.json first (for local dev/testing the catalog/URLs
                // without needing to push to GitHub first). This takes precedence.
                if let Ok(local_entries) = crate::downloader::load_local_registry() {
                    for entry in local_entries {
                        if !all_entries.iter().any(|e: &crate::downloader::RegistryEntry| e.name.eq_ignore_ascii_case(&entry.name)) {
                            all_entries.push(entry);
                        }
                    }
                }

                for url in feed_urls {
                    if let Ok(entries) = crate::downloader::fetch_registry(&url) {
                        for entry in entries {
                            // Dedup by name (stable across feeds and the downloads map format where legacy download_url is often None)
                            if !all_entries.iter().any(|e: &crate::downloader::RegistryEntry| e.name.eq_ignore_ascii_case(&entry.name)) {
                                all_entries.push(entry);
                            }
                        }
                    }
                }

                if !all_entries.is_empty() {
                    if let Ok(mut lock) = thread_state.lock() {
                        *lock = Some(all_entries);
                    }
                }
            });
            Some(state)
        };

        let mut app = App {
            screensavers,
            highlighted,
            focused: FocusedSection::GlobalPrefs,
            global_field: GlobalField::Active,
            global,
            local,
            theme,
            status: None,
            should_quit: false,
            list_offset: 0,
            list_items: Vec::new(),
            visual_progress: 0.0,
            #[cfg(feature = "downloader")]
            download_state: None,
            #[cfg(feature = "downloader")]
            registry_results,
            #[cfg(feature = "downloader")]
            registry_entries: Vec::new(),
            #[cfg(feature = "downloader")]
            pending_action: None,
            selection_start: None,
            selection_end: None,
            selection_pending_copy: false,
            show_help: false,
            show_markdown: None,
            markdown_lines: Vec::new(),
            markdown_scroll: 0,
            glyphs: crate::win32::GlyphMap::load(),
            on_battery: !crate::win32::query_power_status().ac_online,
            last_power_check: std::time::Instant::now(),
            quit_btn_bounds: None,
            help_btn_bounds: None,
            drag_active: false,
            drag_start_cursor: None,
            drag_start_window: None,
        };
        app.update_list_items();
        app
    }

    /// Indices into `self.screensavers` that match the current filter.
    /// Empty filter → all indices, in order.
    pub fn filtered_indices(&self) -> Vec<usize> {
        let indices: Vec<usize> = (0..self.screensavers.len()).collect();

        if self.local.hide_stock {
            indices
                .into_iter()
                .filter(|&i| !crate::preview::is_stock_screensaver(&self.screensavers[i].path))
                .collect()
        } else {
            indices
        }
    }

    /// Map a position in the filtered list to the real index, clamping.
    pub fn resolve_highlight(&mut self) {
        let indices = self.filtered_indices();
        if indices.is_empty() {
            self.highlighted = 0;
            return;
        }
        // Try to keep the current highlighted item selected if it's still
        // visible after filtering.
        if let Some(pos) = indices.iter().position(|&i| i == self.highlighted) {
            self.highlighted = indices[pos];
        } else {
            self.highlighted = indices[0];
        }
    }

    /// Update the cached ListItem widgets in `self.list_items`.
    /// Update the cached ListItem widgets in `self.list_items`.
    pub fn update_list_items(&mut self) {
        let theme = self.theme;

        self.list_items = self
            .screensavers
            .iter()
            .map(|s| {
                let is_checked = self.local.selected_paths.contains(&s.path.to_string_lossy().into_owned());
                let exists = s.path.exists();
                let is_online = s.download_url.is_some() && !exists;
                let is_stock = crate::preview::is_stock_screensaver(&s.path);

                // Simplified columns per user request:
                // Active (yes or no) | Name | Type (stock or custom)
                let active_str = if is_checked { "yes" } else { "no" };
                let active_color = if is_checked { theme.applied } else { theme.text_dim };

                let name = crate::ui::truncate(&s.name, 28);
                let name_str = format!("{:<30}  ", name);
                let name_color = if is_online {
                    theme.accent_primary
                } else if is_checked {
                    theme.text_main
                } else {
                    theme.text_dim
                };

                let type_str = if is_stock {
                    "Stock"
                } else if is_online {
                    "Custom"  // curated items become custom once downloaded
                } else {
                    "Custom"
                };
                let type_color = if is_stock {
                    theme.text_dim
                } else {
                    theme.accent_secondary
                };

                let spans = vec![
                    ratatui::text::Span::styled(
                        format!("{:<8}  ", active_str),
                        ratatui::style::Style::default().fg(active_color),
                    ),
                    ratatui::text::Span::styled(
                        name_str,
                        ratatui::style::Style::default().fg(name_color),
                    ),
                    ratatui::text::Span::styled(
                        type_str.to_string(),
                        ratatui::style::Style::default().fg(type_color),
                    ),
                ];
                ratatui::widgets::ListItem::new(ratatui::text::Line::from(spans))
            })
            .collect();
    }

    pub fn current_screensaver(&self) -> Option<&Screensaver> {
        self.screensavers.get(self.highlighted)
    }

    /// Apply the currently-highlighted screensaver as the system screensaver.
    pub fn apply_highlighted(&mut self) {
        #[cfg(feature = "downloader")]
        if self.trigger_online_download(PendingAction::Apply) {
            return;
        }

        let exe = std::env::current_exe().unwrap_or_default();

        let count = self.local.selected_paths.len();
        if count > 1 {
            self.global.active_scr = exe.to_string_lossy().into_owned();
            self.global.active = true;
            self.status = Some(StatusMessage {
                text: format!("Applied cycle of {} screensavers", count),
                kind: StatusKind::Info,
            });
        } else if count == 1 {
            let path = self.local.selected_paths[0].clone();
            self.global.active_scr = path.clone();
            self.global.active = true;

            // Find the name of the screensaver for the status message
            let name = self.screensavers.iter()
                .find(|s| s.path.to_string_lossy() == path)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "Selected Screensaver".to_string());

            self.status = Some(StatusMessage {
                text: format!("Applied: {}", name),
                kind: StatusKind::Info,
            });
        } else {
            self.global.active_scr = String::new();
            self.global.active = false;
            self.status = Some(StatusMessage {
                text: "Screensaver deactivated (turned off)".to_string(),
                kind: StatusKind::Info,
            });
        }

        if let Err(e) = self.global.save() {
            self.status = Some(StatusMessage {
                text: format!("Failed to save: {e}"),
                kind: StatusKind::Error,
            });
            return;
        }

        if let Some(s) = self.current_screensaver() {
            if let Some(name) = s.path.file_name().and_then(|f| f.to_str()) {
                self.local.last_selected = Some(name.to_string());
            }
        }
        let _ = self.local.save();
        self.update_list_items();
    }

    /// Toggle selection of the highlighted screensaver and immediately apply it to the registry.
    pub fn toggle_and_apply_highlighted(&mut self) {
        #[cfg(feature = "downloader")]
        if self.trigger_online_download(PendingAction::ToggleAndApply) {
            return;
        }

        self.toggle_highlighted_selection();
        self.apply_highlighted();
    }

    /// Toggle the global `active` flag in the registry.
    pub fn toggle_active(&mut self) {
        self.global.active = !self.global.active;
        match self.global.save() {
            Ok(()) => {
                self.status = Some(StatusMessage {
                    text: format!("Active = {}", self.global.active),
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

    /// Toggle the "prevent system sleep" mode.  The state lives in
    /// `LocalConfig` because it's a per-user preference, not a system one.
    pub fn toggle_prevent_sleep(&mut self) {
        self.local.prevent_sleep = !self.local.prevent_sleep;
        if let Some(s) = self.current_screensaver() {
            if let Some(name) = s.path.file_name().and_then(|f| f.to_str()) {
                self.local.last_selected = Some(name.to_string());
            }
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
        if self.local.hide_stock {
            // If stock screensavers are hidden, they should not be active in the cycle.
            self.local.selected_paths.retain(|p| {
                !crate::preview::is_stock_screensaver(std::path::Path::new(p))
            });
        }
        if let Some(s) = self.current_screensaver() {
            if let Some(name) = s.path.file_name().and_then(|f| f.to_str()) {
                self.local.last_selected = Some(name.to_string());
            }
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

    /// Adjust the screensaver cycle time by one step.
    pub fn adjust_cycle_time(&mut self, delta: i32) {
        let next = (self.local.random_cycle_secs as i32 + delta * CYCLE_TIME_STEP_SECS as i32)
            .clamp(CYCLE_TIME_MIN_SECS as i32, CYCLE_TIME_MAX_SECS as i32) as u32;
        if next == self.local.random_cycle_secs {
            return;
        }
        self.local.random_cycle_secs = next;
        if let Err(e) = self.local.save() {
            self.status = Some(StatusMessage {
                text: format!("Save failed: {e}"),
                kind: StatusKind::Error,
            });
        }
    }

    /// Re-discover screensavers and refresh the list.
    pub fn refresh_screensavers(&mut self) {
        self.screensavers = crate::preview::discover();
        #[cfg(feature = "downloader")]
        {
            let entries = self.registry_entries.clone();
            self.merge_registry_entries(entries);
        }
        self.resolve_highlight();
        self.status = Some(StatusMessage {
            text: "Refreshed screensavers list.".to_string(),
            kind: StatusKind::Info,
        });
        self.update_list_items();
    }

    /// Spawn the currently-highlighted screensaver fullscreen.
    pub fn preview_highlighted(&mut self) {
        #[cfg(feature = "downloader")]
        if self.trigger_online_download(PendingAction::Preview) {
            return;
        }

        let Some(s) = self.current_screensaver() else {
            return;
        };
        if let Err(e) = std::process::Command::new(&s.path).arg("/s").spawn() {
            self.status = Some(StatusMessage {
                text: format!("Preview failed: {e}"),
                kind: StatusKind::Error,
            });
        }
    }

    /// Spawn the currently-highlighted screensaver's native configuration dialog.
    pub fn configure_highlighted(&mut self) {
        #[cfg(feature = "downloader")]
        if self.trigger_online_download(PendingAction::Configure) {
            return;
        }

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

    /// Delete a downloaded screensaver file from disk.
    pub fn delete_highlighted(&mut self) {
        let (path, name) = {
            let Some(s) = self.current_screensaver() else {
                return;
            };
            (s.path.clone(), s.name.clone())
        };

        if crate::preview::is_stock_screensaver(&path) {
            self.status = Some(StatusMessage {
                text: "Cannot delete stock Windows screensavers.".to_string(),
                kind: StatusKind::Error,
            });
            return;
        }

        if !path.exists() {
            self.status = Some(StatusMessage {
                text: "Screensaver is not downloaded locally.".to_string(),
                kind: StatusKind::Error,
            });
            return;
        }

        match std::fs::remove_file(&path) {
            Ok(()) => {
                self.status = Some(StatusMessage {
                    text: format!("Deleted screensaver: {}", name),
                    kind: StatusKind::Info,
                });
                let path_str = path.to_string_lossy().into_owned();
                if let Some(pos) = self.local.selected_paths.iter().position(|p| p == &path_str) {
                    self.local.selected_paths.remove(pos);
                    let _ = self.local.save();
                }
                self.refresh_screensavers();
            }
            Err(e) => {
                self.status = Some(StatusMessage {
                    text: format!("Failed to delete: {e}"),
                    kind: StatusKind::Error,
                });
            }
        }
    }

    /// Merge online screensaver entries into local list.
    #[cfg(feature = "downloader")]
    pub fn merge_registry_entries(&mut self, entries: Vec<crate::downloader::RegistryEntry>) {
        self.registry_entries = entries.clone();

        let local_filenames: std::collections::HashSet<String> = self.screensavers.iter()
            .map(|s| s.path.file_name().and_then(|f| f.to_str()).unwrap_or("").to_lowercase())
            .collect();

        for entry in entries {
            let url = entry.download_url_for_current_platform()
                .or_else(|| entry.download_url.clone())
                .unwrap_or_default();
            if url.is_empty() {
                continue;
            }
            let filename = url.split('/').next_back().unwrap_or("").to_lowercase();
            if filename.is_empty() {
                continue;
            }
            if local_filenames.contains(&filename) {
                continue; // Already downloaded/present locally
            }

            let path = crate::config::LocalConfig::config_path()
                .and_then(|p| p.parent().map(|parent| {
                    parent.join("screensavers").join(&filename)
                }))
                .unwrap_or_else(|| std::path::PathBuf::from(&filename));

            self.screensavers.push(Screensaver {
                name: entry.name,
                path,
                download_url: Some(url),
            });
        }

        // Re-sort alphabetically
        self.screensavers.sort_by_key(|s| s.name.to_lowercase());
        self.resolve_highlight();
        self.update_list_items();
    }

    /// Trigger download of the curated screensaver, performing action once done.
    #[cfg(feature = "downloader")]
    pub fn trigger_online_download(&mut self, action: PendingAction) -> bool {
        if let Some(s) = self.current_screensaver() {
            if s.download_url.is_some() && !s.path.exists() {
                if let Some(ref url) = s.download_url {
                    let entry = crate::downloader::RegistryEntry {
                        name: s.name.clone(),
                        author: String::new(),
                        description: String::new(),
                        download_url: Some(url.clone()),
                        downloads: None,
                        version: String::new(),
                    };
                    self.pending_action = Some(action);
                    self.download_state = Some(crate::downloader::spawn_download(&entry));
                    self.visual_progress = 0.0;
                    return true;
                }
            }
        }
        false
    }

    /// Update visual progress towards the actual download progress.
    pub fn update_download_progress(&mut self) {
        #[cfg(feature = "downloader")]
        if self.download_state.is_some() {
            let mut actual_progress = 0.0;
            if let Some(ref state_mutex) = self.download_state {
                if let Ok(state) = state_mutex.lock() {
                    actual_progress = state.progress;
                }
            }
            // Increment visual progress smoothly (e.g. by 0.02 per frame, roughly ~1.5s total duration for instant downloads)
            let target = if actual_progress >= 1.0 { 1.0 } else { actual_progress };
            if self.visual_progress < target {
                self.visual_progress = (self.visual_progress + 0.015).min(target);
            }
        }
    }

    /// Toggle selection of the highlighted screensaver for custom cycling.
    pub fn toggle_highlighted_selection(&mut self) {
        #[cfg(feature = "downloader")]
        if self.trigger_online_download(PendingAction::ToggleSelection) {
            return;
        }

        let (path_str, name) = {
            let Some(s) = self.current_screensaver() else {
                return;
            };
            (s.path.to_string_lossy().into_owned(), s.name.clone())
        };

        if let Some(pos) = self.local.selected_paths.iter().position(|p| p == &path_str) {
            self.local.selected_paths.remove(pos);
            self.status = Some(StatusMessage {
                text: format!("Deselected: {}", name),
                kind: StatusKind::Info,
            });
        } else {
            self.local.selected_paths.push(path_str);
            self.status = Some(StatusMessage {
                text: format!("Selected: {}", name),
                kind: StatusKind::Info,
            });
        }
        let _ = self.local.save();
        self.update_list_items();
    }

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

    /// Handle a single key event.  Returns `true` if the app should quit.
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
                KeyCode::F(1) => {
                    self.show_help = false;
                    self.open_embedded_markdown("README.md", README_CONTENT);
                }
                KeyCode::F(2) => {
                    self.show_help = false;
                    self.open_embedded_markdown("SUPPORT.md", SUPPORT_CONTENT);
                }
                KeyCode::F(3) => {
                    self.show_help = false;
                    self.open_embedded_markdown("LICENSE.md", LICENSE_CONTENT);
                }
                KeyCode::F(4) => {
                    self.show_help = false;
                    self.open_embedded_markdown("COPYRIGHT.md", COPYRIGHT_CONTENT);
                }
                KeyCode::F(5) => {
                    self.show_help = false;
                    self.open_embedded_markdown("PRIVACY.md", PRIVACY_CONTENT);
                }
                KeyCode::F(6) => {
                    self.show_help = false;
                    self.open_embedded_markdown("SECURITY.md", SECURITY_CONTENT);
                }
                KeyCode::F(7) => {
                    self.show_help = false;
                    self.open_embedded_markdown("CONTRIBUTING.md", CONTRIBUTING_CONTENT);
                }
                _ => {}
            }
            return false;
        }

        if self.show_markdown.is_some() {
            match code {
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                    self.show_markdown = None;
                }
                KeyCode::F(1) => {
                    self.open_embedded_markdown("README.md", README_CONTENT);
                }
                KeyCode::F(2) => {
                    self.open_embedded_markdown("SUPPORT.md", SUPPORT_CONTENT);
                }
                KeyCode::F(3) => {
                    self.open_embedded_markdown("LICENSE.md", LICENSE_CONTENT);
                }
                KeyCode::F(4) => {
                    self.open_embedded_markdown("COPYRIGHT.md", COPYRIGHT_CONTENT);
                }
                KeyCode::F(5) => {
                    self.open_embedded_markdown("PRIVACY.md", PRIVACY_CONTENT);
                }
                KeyCode::F(6) => {
                    self.open_embedded_markdown("SECURITY.md", SECURITY_CONTENT);
                }
                KeyCode::F(7) => {
                    self.open_embedded_markdown("CONTRIBUTING.md", CONTRIBUTING_CONTENT);
                }
                KeyCode::Up => {
                    self.markdown_scroll = self.markdown_scroll.saturating_sub(1);
                }
                KeyCode::Down => {
                    if self.markdown_scroll + 10 < self.markdown_lines.len() {
                        self.markdown_scroll += 1;
                    }
                }
                KeyCode::PageUp => {
                    self.markdown_scroll = self.markdown_scroll.saturating_sub(15);
                }
                KeyCode::PageDown => {
                    if self.markdown_scroll + 15 < self.markdown_lines.len() {
                        self.markdown_scroll += 15;
                    } else {
                        self.markdown_scroll = self.markdown_lines.len().saturating_sub(10);
                    }
                }
                _ => {}
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
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if self.focused == FocusedSection::SaverList {
                    self.delete_highlighted();
                }
            }
            KeyCode::F(1) => {
                self.open_embedded_markdown("README.md", README_CONTENT);
            }
            KeyCode::F(2) => {
                self.open_embedded_markdown("SUPPORT.md", SUPPORT_CONTENT);
            }
            KeyCode::F(3) => {
                self.open_embedded_markdown("LICENSE.md", LICENSE_CONTENT);
            }
            KeyCode::F(4) => {
                self.open_embedded_markdown("COPYRIGHT.md", COPYRIGHT_CONTENT);
            }
            KeyCode::F(5) => {
                self.open_embedded_markdown("PRIVACY.md", PRIVACY_CONTENT);
            }
            KeyCode::F(6) => {
                self.open_embedded_markdown("SECURITY.md", SECURITY_CONTENT);
            }
            KeyCode::F(7) => {
                self.open_embedded_markdown("CONTRIBUTING.md", CONTRIBUTING_CONTENT);
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
                GlobalField::CycleTime => self.adjust_cycle_time(-1),
                _ => {}
            }
        }
    }

    fn on_right(&mut self) {
        if self.focused == FocusedSection::GlobalPrefs {
            match self.global_field {
                GlobalField::Timeout => self.adjust_timeout(1),
                GlobalField::CycleTime => self.adjust_cycle_time(1),
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
                GlobalField::Timeout | GlobalField::CycleTime => {}
            },
            FocusedSection::SaverList => self.toggle_and_apply_highlighted(),
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

pub use ratatui::crossterm::event::{KeyCode, KeyModifiers};



/// Convenience: kick off the random cycle and return when it finishes.
pub fn run_random_cycle() {
    let local_config = LocalConfig::load();
    let exe = std::env::current_exe().ok();

    let candidates: Vec<PathBuf> = local_config.selected_paths
        .iter()
        .map(PathBuf::from)
        .filter(|p| p.exists() && !is_self(p, exe.as_ref()) && !is_uninstall(p))
        .collect();

    if candidates.is_empty() {
        return;
    }

    let cycle_duration = std::time::Duration::from_secs(local_config.random_cycle_secs as u64);

    let mut seed: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut mask = None;
    loop {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let idx = (seed as usize) % candidates.len();
        let target = &candidates[idx];
        let mut child = match std::process::Command::new(target).arg("/s").spawn() {
            Ok(c) => c,
            Err(_) => break,
        };

        if mask.is_some() {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let _ = mask.take();
        }

        let start = std::time::Instant::now();
        let mut exited = false;
        while start.elapsed() < cycle_duration {
            match child.try_wait() {
                Ok(Some(_)) => {
                    exited = true;
                    break;
                }
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
                Err(_) => {
                    exited = true;
                    break;
                }
            }
        }
        if exited {
            break;
        }
        mask = crate::win32::CycleMask::new();
        std::thread::sleep(std::time::Duration::from_millis(50));
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &child.id().to_string()])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();

            let start_wait = std::time::Instant::now();
            let mut grace_exit = false;
            while start_wait.elapsed() < std::time::Duration::from_millis(300) {
                if let Ok(Some(_)) = child.try_wait() {
                    grace_exit = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            if !grace_exit {
                let _ = child.kill();
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = child.kill();
        }
    }
}

fn is_self(p: &PathBuf, exe: Option<&PathBuf>) -> bool {
    exe.map(|e| e == p).unwrap_or(false)
}

fn is_uninstall(p: &std::path::Path) -> bool {
    p.file_name()
        .and_then(|f| f.to_str())
        .map(str::to_lowercase)
        .map(|n| n.contains("uninstall"))
        .unwrap_or(false)
}

impl App {
    /// Load and open an embedded markdown document in the viewer modal.
    pub fn open_embedded_markdown(&mut self, title: &str, content: &str) {
        self.markdown_lines = parse_markdown_to_lines(content, &self.theme);
        self.show_markdown = Some(title.to_string());
        self.markdown_scroll = 0;
        self.status = Some(StatusMessage {
            text: format!("Opened document: {}", title),
            kind: StatusKind::Info,
        });
    }

    /// Checks system power status periodically and adjusts throttling state.
    pub fn sync_power_status_if_needed(&mut self) {
        if self.last_power_check.elapsed() > std::time::Duration::from_millis(5000) {
            self.last_power_check = std::time::Instant::now();
            let power = crate::win32::query_power_status();
            let current_on_battery = !power.ac_online;
            if current_on_battery != self.on_battery {
                self.on_battery = current_on_battery;
                let state = if current_on_battery {
                    "Battery (Power-Saving Throttling Enabled)"
                } else {
                    "AC Power (Full Speed)"
                };
                tracing::info!("Power source changed. Status: {}", state);
                self.status = Some(StatusMessage {
                    text: format!("Power Source Changed: {}", state),
                    kind: StatusKind::Info,
                });
            }
        }
    }
}

fn parse_markdown_to_lines(content: &str, theme: &TuiTheme) -> Vec<ratatui::text::Line<'static>> {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};

    let mut lines = Vec::new();
    let mut in_code_block = false;
    let mut current_paragraph = String::new();

    let flush_paragraph = |para: &mut String, lines: &mut Vec<Line<'static>>| {
        if !para.is_empty() {
            lines.push(Line::from(Span::styled(
                para.clone(),
                Style::default().fg(theme.text_main),
            )));
            para.clear();
        }
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            flush_paragraph(&mut current_paragraph, &mut lines);
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(Color::Rgb(150, 240, 150)),
            )));
            continue;
        }

        if trimmed.is_empty() {
            flush_paragraph(&mut current_paragraph, &mut lines);
            lines.push(Line::from(""));
            continue;
        }

        if trimmed.starts_with("# ") {
            flush_paragraph(&mut current_paragraph, &mut lines);
            let header = trimmed[2..].to_string();
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("=== {} ===", header.to_uppercase()),
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
        } else if trimmed.starts_with("## ") {
            flush_paragraph(&mut current_paragraph, &mut lines);
            let header = trimmed[3..].to_string();
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("--- {} ---", header),
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
        } else if trimmed.starts_with("### ") {
            flush_paragraph(&mut current_paragraph, &mut lines);
            let header = trimmed[4..].to_string();
            lines.push(Line::from(Span::styled(
                header,
                Style::default().fg(theme.accent_primary),
            )));
        } else if trimmed.starts_with("* ") {
            flush_paragraph(&mut current_paragraph, &mut lines);
            let item = trimmed[2..].to_string();
            lines.push(Line::from(vec![
                Span::styled(" • ", Style::default().fg(theme.accent_primary)),
                Span::styled(item, Style::default().fg(theme.text_main)),
            ]));
        } else if trimmed.starts_with("- ") {
            flush_paragraph(&mut current_paragraph, &mut lines);
            let item = trimmed[2..].to_string();
            lines.push(Line::from(vec![
                Span::styled(" • ", Style::default().fg(theme.accent_primary)),
                Span::styled(item, Style::default().fg(theme.text_main)),
            ]));
        } else if trimmed.starts_with("1. ")
            || trimmed.starts_with("2. ")
            || trimmed.starts_with("3. ")
            || trimmed.starts_with("4. ")
            || trimmed.starts_with("5. ")
        {
            flush_paragraph(&mut current_paragraph, &mut lines);
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", &trimmed[..3]),
                    Style::default().fg(theme.accent_primary),
                ),
                Span::styled(trimmed[3..].to_string(), Style::default().fg(theme.text_main)),
            ]));
        } else {
            if !current_paragraph.is_empty() {
                current_paragraph.push(' ');
            }
            current_paragraph.push_str(trimmed);
        }
    }

    flush_paragraph(&mut current_paragraph, &mut lines);
    lines
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GlobalConfig;
    use crate::theme::TuiTheme;
    use ratatui::crossterm::event::KeyCode;

    fn mock_app() -> App {
        let screensavers = vec![
            Screensaver {
                name: "Bubbles".to_string(),
                path: PathBuf::from("C:\\Windows\\System32\\bubbles.scr"),
                #[cfg(feature = "downloader")]
                download_url: None,
            },
            Screensaver {
                name: "Mystify".to_string(),
                path: PathBuf::from("C:\\Windows\\System32\\mystify.scr"),
                #[cfg(feature = "downloader")]
                download_url: None,
            },
            Screensaver {
                name: "Ribbons".to_string(),
                path: PathBuf::from("C:\\Windows\\System32\\ribbons.scr"),
                #[cfg(feature = "downloader")]
                download_url: None,
            },
        ];
        let global = GlobalConfig::default();
        let local = LocalConfig::default();
        let theme = TuiTheme::high_contrast(true);
        App::new(screensavers, global, local, theme)
    }

    #[test]
    fn test_is_uninstall() {
        assert!(is_uninstall(std::path::Path::new(
            "C:\\some\\uninstall.exe"
        )));
        assert!(is_uninstall(std::path::Path::new("UNINSTALL_scr.scr")));
        assert!(!is_uninstall(std::path::Path::new("bubbles.scr")));
    }

    #[test]
    fn test_filtered_indices() {
        let mut app = mock_app();

        // No filter -> all indices
        assert_eq!(app.filtered_indices(), vec![0, 1, 2]);

        // Hide stock screensavers
        app.local.hide_stock = true;
        assert_eq!(app.filtered_indices(), Vec::<usize>::new());
    }

    #[test]
    fn test_handle_key_navigation_and_focus() {
        let mut app = mock_app();
        assert_eq!(app.focused, FocusedSection::GlobalPrefs);
        assert_eq!(app.global_field, GlobalField::Active);

        // Move down within GlobalPrefs
        app.handle_key(KeyCode::Down, KeyModifiers::empty());
        assert_eq!(app.global_field, GlobalField::Timeout);

        app.handle_key(KeyCode::Down, KeyModifiers::empty());
        assert_eq!(app.global_field, GlobalField::PreventSleep);

        // Move down to CycleTime
        app.handle_key(KeyCode::Down, KeyModifiers::empty());
        assert_eq!(app.global_field, GlobalField::CycleTime);

        // Move down to HideStock
        app.handle_key(KeyCode::Down, KeyModifiers::empty());
        assert_eq!(app.global_field, GlobalField::HideStock);

        // Move down wraps around to Active
        app.handle_key(KeyCode::Down, KeyModifiers::empty());
        assert_eq!(app.global_field, GlobalField::Active);

        // Tab cycles focus to SaverList
        app.handle_key(KeyCode::Tab, KeyModifiers::empty());
        assert_eq!(app.focused, FocusedSection::SaverList);

        // SaverList navigation
        assert_eq!(app.highlighted, 0);
        app.handle_key(KeyCode::Down, KeyModifiers::empty());
        assert_eq!(app.highlighted, 1);
    }

    #[test]
    fn test_selection_and_apply() {
        let _lock = crate::config::TEST_LOCK.lock().unwrap();

        // Create a unique temp dir for the test to avoid collisions
        let temp_dir = std::env::temp_dir().join(format!(
            "ridle_test_app_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Set APPDATA to redirect LocalConfig load/save
        unsafe {
            std::env::set_var("APPDATA", &temp_dir);
        }

        let mut app = mock_app();
        assert!(app.local.selected_paths.is_empty());

        // Focus SaverList
        app.focused = FocusedSection::SaverList;

        // Toggle selection on the first item (Bubbles)
        app.handle_key(KeyCode::Char(' '), KeyModifiers::empty());
        assert_eq!(app.local.selected_paths.len(), 1);
        assert_eq!(app.local.selected_paths[0], "C:\\Windows\\System32\\bubbles.scr");

        // Toggle selection on the second item (Mystify)
        app.highlighted = 1;
        app.handle_key(KeyCode::Char(' '), KeyModifiers::empty());
        assert_eq!(app.local.selected_paths.len(), 2);
        assert_eq!(app.local.selected_paths[1], "C:\\Windows\\System32\\mystify.scr");

        // Hitting Enter on the list applies the multi-selection.
        // It should set registry/global config active_scr to the path of ridle.exe itself.
        let exe = std::env::current_exe().unwrap_or_default();
        assert_eq!(app.global.active_scr, exe.to_string_lossy().into_owned());

        // Toggle second item again (uncheck Mystify) using Enter (which does the same thing)
        app.handle_key(KeyCode::Enter, KeyModifiers::empty());
        assert_eq!(app.local.selected_paths.len(), 1);
        assert_eq!(app.global.active_scr, "C:\\Windows\\System32\\bubbles.scr");

        // Clean up temp dir
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    #[cfg(feature = "downloader")]
    fn test_registry_merge_and_automated_downloader() {
        let _lock = crate::config::TEST_LOCK.lock().unwrap();

        // Create a unique temp dir for the test to avoid collisions
        let temp_dir = std::env::temp_dir().join(format!(
            "ridle_test_app_downloader_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Set APPDATA to redirect LocalConfig load/save
        unsafe {
            std::env::set_var("APPDATA", &temp_dir);
        }

        let mut app = mock_app();
        app.focused = FocusedSection::SaverList;

        // Verify initial list has 3 items (Bubbles, Mystify, Ribbons)
        assert_eq!(app.screensavers.len(), 3);

        // Manually merge registry entries
        let entries = vec![
            crate::downloader::RegistryEntry {
                name: "beams".to_string(),
                author: "UberMetroid".to_string(),
                description: "Beams screensaver".to_string(),
                download_url: Some("https://example.com/beams.scr".to_string()),
                downloads: None,
                version: "2.0".to_string(),
            },
        ];
        app.merge_registry_entries(entries.clone());

        // Verify list now has 4 items (alphabetically ordered: beams, Bubbles, Mystify, Ribbons)
        assert_eq!(app.screensavers.len(), 4);
        assert_eq!(app.screensavers[0].name, "beams");
        assert_eq!(app.screensavers[0].download_url.as_deref(), Some("https://example.com/beams.scr"));

        // Highlight beams (which is index 0)
        app.highlighted = 0;
        assert_eq!(app.highlighted, 0);

        // Pressing space (toggle selection) should trigger background download of beams
        app.toggle_highlighted_selection();
        assert!(app.download_state.is_some());
        assert_eq!(app.pending_action, Some(PendingAction::ToggleSelection));

        // Clean up temp dir
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_console_modes() {
        println!("TESTING RAW MODES:");
        match ratatui::crossterm::terminal::enable_raw_mode() {
            Ok(_) => println!("  enable_raw_mode: OK"),
            Err(e) => println!("  enable_raw_mode: ERROR: {}", e),
        }
        match ratatui::crossterm::terminal::disable_raw_mode() {
            Ok(_) => println!("  disable_raw_mode: OK"),
            Err(e) => println!("  disable_raw_mode: ERROR: {}", e),
        }
    }
}
