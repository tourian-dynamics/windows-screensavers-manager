//! Win32 platform integration re-exports.
//!
//! **Taxonomy Classification**: Platform (OS / Hardware Layer).

#![allow(unused_imports)]

pub use library::toolkit::clipboard::copy_text_to_clipboard;
pub use library::apps::event_log::log_system_event as log_windows_event;
pub use library::apps::notification::show_toast_notification;
pub use library::toolkit::sys_info::{query_os_version, GlyphMap};
pub use library::apps::window::{
    center_console_window, query_cursor_pos, get_window_rect, set_window_pos,
    BorderlessConsole, SingleInstanceGuard,
};
pub use crate::backend::saver_win32::query_power_status;
pub use crate::backend::saver_win32::PowerStatus;
pub use crate::backend::saver_win32::RECT;
pub use crate::backend::saver_win32::*;

#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub fn spawn_linux_screensaver(_path: &std::path::Path, _arg: &str) -> std::io::Result<std::process::Child> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "spawn_linux_screensaver is not supported on Windows"))
}
