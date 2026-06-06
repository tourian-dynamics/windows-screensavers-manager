//! Safe wrappers around the Win32 APIs that rSaver touches.
//!
//! Everything that calls into `windows-sys` lives here so the rest of the
//! codebase never needs `unsafe`.

use std::ffi::c_void;
use std::ptr::NonNull;

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::System::Console::{
    CONSOLE_SCREEN_BUFFER_INFOEX, GetConsoleScreenBufferInfoEx, GetConsoleTitleW, GetConsoleWindow,
    SetConsoleTitleW,
};
use windows_sys::Win32::System::Console::{GetStdHandle, STD_OUTPUT_HANDLE};
use windows_sys::Win32::System::Power::{
    ES_AWAYMODE_REQUIRED, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
    GetSystemPowerStatus, SYSTEM_POWER_STATUS, SetThreadExecutionState,
};
use windows_sys::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
use windows_sys::Win32::UI::HiDpi::{GetDpiForSystem, GetDpiForWindow};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_STYLE, GetSystemMetrics, GetWindowLongPtrW, SM_CXSCREEN, SM_CYSCREEN, SPI_GETHIGHCONTRAST,
    SPI_SETSCREENSAVEACTIVE, SPI_SETSCREENSAVETIMEOUT, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOZORDER, SWP_NOSIZE, SetWindowLongPtrW, SetWindowPos,
    SystemParametersInfoW, WS_CAPTION, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
    GetCursorPos,
};

const STYLE_MASK_TO_STRIP: i32 =
    (WS_CAPTION | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU) as i32;

/// An RGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// Convert a 0x00BBGGRR (COLORREF / standard BGR) color into RGB.
    pub const fn from_bgr(bgr: u32) -> Self {
        Rgb(bgr as u8, (bgr >> 8) as u8, (bgr >> 16) as u8)
    }

    /// Convert a 0xAARRGGBB (ARGB) color into RGB.
    pub const fn from_argb(argb: u32) -> Self {
        Rgb((argb >> 16) as u8, (argb >> 8) as u8, argb as u8)
    }
}

/// 16-color console palette. Index matches the standard ANSI / Windows color
/// table (0 = black, 1 = red, ..., 8 = bright black / dark grey, etc).
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub colors: [Rgb; 16],
}

impl Default for Palette {
    fn default() -> Self {
        // Windows console defaults; used as a fallback if the API call fails.
        let c = |r, g, b| Rgb(r, g, b);
        Palette {
            colors: [
                c(12, 12, 12),    // 0 black
                c(197, 15, 31),   // 1 red
                c(19, 161, 14),   // 2 green
                c(193, 156, 0),   // 3 yellow
                c(0, 0, 238),     // 4 blue
                c(136, 23, 152),  // 5 magenta
                c(58, 150, 221),  // 6 cyan
                c(204, 204, 204), //7 white
                c(118, 118, 118), //8 dark grey
                c(231, 72, 86),   // 9 light red
                c(22, 198, 12),   // 10 light green
                c(249, 241, 165), //11 light yellow
                c(59, 120, 255),  //12 light blue
                c(180, 0, 158),   // 13 light magenta
                c(97, 214, 214),  //14 light cyan
                c(242, 242, 242), //15 white
            ],
        }
    }
}

impl Palette {
    /// Query the live console palette via `GetConsoleScreenBufferInfoEx`.
    pub fn query() -> Self {
        let stdout = match unsafe { stdout_handle() } {
            Some(h) => h.as_ptr(),
            None => return Self::default(),
        };
        let mut info: CONSOLE_SCREEN_BUFFER_INFOEX = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<CONSOLE_SCREEN_BUFFER_INFOEX>() as u32;
        let ok = unsafe { GetConsoleScreenBufferInfoEx(stdout, &mut info) };
        if ok == 0 {
            return Self::default();
        }
        let mut colors = [Rgb(0, 0, 0); 16];
        for (i, slot) in info.ColorTable.iter().enumerate() {
            colors[i] = Rgb::from_bgr(*slot);
        }
        Palette { colors }
    }
}

// SAFETY: Caller must ensure standard handle handles are valid.
unsafe fn stdout_handle() -> Option<NonNull<c_void>> {
    // SAFETY: STD_OUTPUT_HANDLE is query-safe.
    let h = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
    if h.is_null() { None } else { NonNull::new(h) }
}

/// RAII guard that strips the console window's title bar / borders / system
/// menu and restores them on drop.
pub struct BorderlessConsole {
    hwnd: HWND,
    original_style: i32,
    original_rect: RECT,
    active: bool,
}

impl BorderlessConsole {
    pub fn enable() -> Self {
        let hwnd = unsafe { GetConsoleWindow() };
        if hwnd.is_null() {
            return BorderlessConsole {
                hwnd: std::ptr::null_mut(),
                original_style: 0,
                original_rect: unsafe { std::mem::zeroed() },
                active: false,
            };
        }
        let original = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as i32;
        use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;
        let mut original_rect: RECT = unsafe { std::mem::zeroed() };
        unsafe {
            GetWindowRect(hwnd, &mut original_rect);
        }
        let new_style = original & !STYLE_MASK_TO_STRIP;
        unsafe {
            SetWindowLongPtrW(hwnd, GWL_STYLE, new_style as isize);
        }

        let metrics = SystemMetrics::query();
        let dpi = metrics.window_dpi;
        let scale = dpi as f32 / 96.0;
        let width = (900.0 * scale) as i32;
        let height = (900.0 * scale) as i32;

        let mut x = (metrics.screen_w - width) / 2;
        let mut y = (metrics.screen_h - height) / 2;

        let h_monitor = unsafe { windows_sys::Win32::Graphics::Gdi::MonitorFromWindow(hwnd, 2) }; // MONITOR_DEFAULTTONEAREST = 2
        if h_monitor != std::ptr::null_mut() {
            let mut mi: windows_sys::Win32::Graphics::Gdi::MONITORINFO = unsafe { std::mem::zeroed() };
            mi.cbSize = std::mem::size_of::<windows_sys::Win32::Graphics::Gdi::MONITORINFO>() as u32;
            if unsafe { windows_sys::Win32::Graphics::Gdi::GetMonitorInfoW(h_monitor, &mut mi as *mut _ as *mut _) } != 0 {
                let monitor_w = mi.rcWork.right - mi.rcWork.left;
                let monitor_h = mi.rcWork.bottom - mi.rcWork.top;
                x = mi.rcWork.left + (monitor_w - width) / 2;
                y = mi.rcWork.top + (monitor_h - height) / 2;
            }
        }

        unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                x,
                y,
                width,
                height,
                SWP_FRAMECHANGED | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }

        BorderlessConsole {
            hwnd,
            original_style: original,
            original_rect,
            active: true,
        }
    }
}

impl Drop for BorderlessConsole {
    fn drop(&mut self) {
        if !self.active || self.hwnd.is_null() {
            return;
        }
        unsafe {
            SetWindowLongPtrW(self.hwnd, GWL_STYLE, self.original_style as isize);
            let width = self.original_rect.right - self.original_rect.left;
            let height = self.original_rect.bottom - self.original_rect.top;
            SetWindowPos(
                self.hwnd,
                std::ptr::null_mut(),
                self.original_rect.left,
                self.original_rect.top,
                width,
                height,
                SWP_FRAMECHANGED | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
}

pub fn query_cursor_pos() -> Option<(i32, i32)> {
    unsafe {
        let mut pt = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
        if GetCursorPos(&mut pt) != 0 {
            Some((pt.x, pt.y))
        } else {
            None
        }
    }
}

pub fn get_window_rect() -> Option<RECT> {
    let hwnd = unsafe { GetConsoleWindow() };
    if hwnd.is_null() {
        return None;
    }
    let mut rect: RECT = unsafe { std::mem::zeroed() };
    let ok = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect) };
    if ok == 0 { None } else { Some(rect) }
}

pub fn set_window_pos(x: i32, y: i32) {
    let hwnd = unsafe { GetConsoleWindow() };
    if !hwnd.is_null() {
        unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
}

/// System metrics collected at startup.
#[derive(Debug, Clone, Copy)]
pub struct SystemMetrics {
    pub screen_w: i32,
    pub screen_h: i32,
    pub dpi: u32,
    pub window_dpi: u32,
    pub dark_mode: bool,
    pub high_contrast: bool,
    pub accent: Rgb,
    pub power: PowerStatus,
}

#[derive(Debug, Clone, Copy)]
pub struct PowerStatus {
    pub ac_online: bool,
    pub battery_percent: u8, // 0..=100, 255 = unknown
}

impl SystemMetrics {
    pub fn query() -> Self {
        let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        let dpi = unsafe { GetDpiForSystem() };
        let hwnd = unsafe { GetConsoleWindow() };
        let window_dpi = if hwnd.is_null() {
            dpi
        } else {
            unsafe { GetDpiForWindow(hwnd) }
        };

        SystemMetrics {
            screen_w,
            screen_h,
            dpi,
            window_dpi,
            dark_mode: query_dark_mode(),
            high_contrast: query_high_contrast(),
            accent: query_accent_color(),
            power: query_power_status(),
        }
    }
}

// SAFETY: Caller must verify query target memory representation matching T.
unsafe fn system_parameters_info_get<T>(action: u32, mut payload: T) -> Option<T> {
    let size = std::mem::size_of::<T>() as u32;
    // SAFETY: SPI query is safe with correct type size and layout.
    let ok = unsafe { SystemParametersInfoW(action, size, &mut payload as *mut _ as *mut _, 0) };
    if ok == 0 { None } else { Some(payload) }
}

fn query_high_contrast() -> bool {
    let mut hc: HIGHCONTRASTW = unsafe { std::mem::zeroed() };
    hc.cbSize = std::mem::size_of::<HIGHCONTRASTW>() as u32;
    let Some(res) = (unsafe { system_parameters_info_get(SPI_GETHIGHCONTRAST, hc) }) else {
        return false;
    };
    res.dwFlags & HCF_HIGHCONTRASTON != 0
}

/// Tell Windows whether the calling thread should keep the system / display
/// awake.  `prevent = true` requests ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED
/// | ES_AWAYMODE_REQUIRED; `prevent = false` returns to the default
/// ES_CONTINUOUS state.  Always pairs with `ES_CONTINUOUS` so subsequent
/// changes take effect immediately.
pub fn set_thread_execution_state(prevent: bool) {
    let flags = if prevent {
        ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED | ES_AWAYMODE_REQUIRED
    } else {
        ES_CONTINUOUS
    };
    unsafe { SetThreadExecutionState(flags) };
}

fn query_dark_mode() -> bool {
    // AppsUseLightTheme = 0 means dark mode is on.
    use winreg::RegKey;
    use winreg::enums::*;
    let Ok(key) = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize")
    else {
        return true; // default to dark if we can't tell
    };
    key.get_value::<u32, _>("AppsUseLightTheme")
        .map(|v| v == 0)
        .unwrap_or(true)
}

fn query_accent_color() -> Rgb {
    // DwmGetColorizationColor returns an ARGB color (0xAARRGGBB).
    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmGetColorizationColor(pcr_color: *mut u32, pf_opaque_blend: *mut i32) -> i32;
    }
    let mut color: u32 = 0;
    let mut _opaque: i32 = 0;
    let hr = unsafe { DwmGetColorizationColor(&mut color, &mut _opaque) };
    if hr != 0 {
        return Rgb(0, 120, 215); // canonical Windows blue
    }
    Rgb::from_argb(color)
}

/// Query system battery life and charging source.
pub fn query_power_status() -> PowerStatus {
    let mut s: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetSystemPowerStatus(&mut s) };
    if ok == 0 {
        return PowerStatus {
            ac_online: true,
            battery_percent: 255,
        };
    }
    PowerStatus {
        ac_online: s.ACLineStatus == 1,
        battery_percent: s.BatteryLifePercent,
    }
}

/// Bounding rect of the console window, in screen pixels.
#[allow(dead_code)]
pub fn console_window_rect() -> Option<RECT> {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;
    let hwnd = unsafe { GetConsoleWindow() };
    if hwnd.is_null() {
        return None;
    }
    let mut r: RECT = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetWindowRect(hwnd, &mut r) };
    if ok == 0 { None } else { Some(r) }
}

// SAFETY: Action must represent a valid parameter write.
unsafe fn system_parameters_info_set(action: u32, param: u32) {
    // SAFETY: Parameter write fits typical Win32 representation bounds.
    unsafe {
        SystemParametersInfoW(
            action,
            param,
            std::ptr::null_mut(),
            SPIF_SENDCHANGE | SPIF_UPDATEINIFILE,
        );
    }
}

/// Notify the OS whether the screensaver active flag is enabled or disabled.
pub fn update_screensaver_active(active: bool) {
    // SAFETY: SPI_SETSCREENSAVEACTIVE action is safe.
    unsafe {
        system_parameters_info_set(SPI_SETSCREENSAVEACTIVE, active as u32);
    }
}

/// Notify the OS of the screensaver timeout, in seconds.
pub fn update_screensaver_timeout(timeout_secs: u32) {
    // SAFETY: SPI_SETSCREENSAVETIMEOUT action is safe.
    unsafe {
        system_parameters_info_set(SPI_SETSCREENSAVETIMEOUT, timeout_secs);
    }
}

/// Query the current console window title.
pub fn get_console_title() -> std::io::Result<String> {
    let mut buf = [0u16; 512];
    // SAFETY: buf is valid and its size matches the size parameter.
    let len = unsafe { GetConsoleTitleW(buf.as_mut_ptr(), buf.len() as u32) };
    if len == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(String::from_utf16_lossy(&buf[..len as usize]))
}

/// Set the console window title.
pub fn set_console_title(title: &str) -> std::io::Result<()> {
    let title_w: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: title_w is null-terminated and its pointer is valid.
    let ok = unsafe { SetConsoleTitleW(title_w.as_ptr()) };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// A guard that holds a named system mutex to ensure only one instance of rSaver TUI is running.
pub struct SingleInstanceGuard {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

impl SingleInstanceGuard {
    /// Attempt to acquire the single-instance mutex. Returns Err if another instance is running.
    pub fn try_new() -> Result<Self, String> {
        use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn CreateMutexW(
                lp_mutex_attributes: *const std::ffi::c_void,
                b_initial_owner: i32,
                lp_name: *const u16,
            ) -> windows_sys::Win32::Foundation::HANDLE;
        }

        let name: Vec<u16> = "Local\\rsav_SingleInstanceMutex_2026"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: The name pointer is valid and null-terminated.
        let handle = unsafe { CreateMutexW(std::ptr::null(), 1, name.as_ptr()) };
        if handle as isize == 0 || handle as isize == -1 {
            return Err("Failed to create single-instance mutex.".to_string());
        }

        // SAFETY: GetLastError is safe to call.
        let err = unsafe { GetLastError() };
        if err == ERROR_ALREADY_EXISTS {
            // SAFETY: CloseHandle is safe to call on non-null handle.
            unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) };
            return Err("Another instance of rSaver is already running.".to_string());
        }

        Ok(SingleInstanceGuard { handle })
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        if self.handle as isize != 0 && self.handle as isize != -1 {
            // SAFETY: CloseHandle is safe to call on valid non-null handle.
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.handle);
            }
        }
    }
}

/// A temporary topmost full-screen black window to mask desktop flashes during cycle transition.
pub struct CycleMask {
    hwnd: HWND,
}

unsafe extern "system" fn mask_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    // SAFETY: DefWindowProcW is safe to call
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

impl CycleMask {
    /// Create and show a new topmost black full-screen window to cover the screen.
    pub fn new() -> Option<Self> {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            RegisterClassW, CreateWindowExW, ShowWindow, WNDCLASSW, WS_POPUP, SW_SHOW,
            CS_HREDRAW, CS_VREDRAW, WS_EX_TOPMOST, PeekMessageW, TranslateMessage,
            DispatchMessageW, MSG,
        };
        use windows_sys::Win32::Graphics::Gdi::{GetStockObject, BLACK_BRUSH, HBRUSH};

        let class_name: Vec<u16> = "rsaver_mask_class\0".encode_utf16().collect();

        unsafe {
            let wnd_class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(mask_wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: std::ptr::null_mut(),
                hIcon: std::ptr::null_mut(),
                hCursor: std::ptr::null_mut(),
                hbrBackground: GetStockObject(BLACK_BRUSH) as HBRUSH,
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
            };

            RegisterClassW(&wnd_class);

            let metrics = SystemMetrics::query();
            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST,
                class_name.as_ptr(),
                std::ptr::null(),
                WS_POPUP,
                0,
                0,
                metrics.screen_w,
                metrics.screen_h,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            );

            if !hwnd.is_null() {
                ShowWindow(hwnd, SW_SHOW);

                // Pump pending paint/create messages once to guarantee background renders black
                let mut msg: MSG = std::mem::zeroed();
                while PeekMessageW(&mut msg, hwnd, 0, 0, 1) != 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                Some(CycleMask { hwnd })
            } else {
                None
            }
        }
    }
}

impl Drop for CycleMask {
    fn drop(&mut self) {
        if !self.hwnd.is_null() {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                DestroyWindow, PeekMessageW, TranslateMessage, DispatchMessageW, MSG,
            };
            unsafe {
                DestroyWindow(self.hwnd);
                // Pump messages briefly to allow clean up
                let mut msg: MSG = std::mem::zeroed();
                while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, 1) != 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    }
}

/// Query the Windows OS version and build number from the registry.
pub fn query_os_version() -> String {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
    else {
        return "Windows".to_string();
    };

    let mut product_name = key.get_value::<String, _>("ProductName").unwrap_or_else(|_| "Windows".to_string());
    let current_build = key.get_value::<String, _>("CurrentBuild").unwrap_or_default();
    let display_version = key.get_value::<String, _>("DisplayVersion").unwrap_or_default();

    if product_name.starts_with("Windows 10") {
        if let Ok(build) = current_build.parse::<u32>() {
            if build >= 22000 {
                product_name = product_name.replace("Windows 10", "Windows 11");
            }
        }
    }

    let mut parts = vec![product_name];
    if !display_version.is_empty() {
        parts.push(display_version);
    }
    if !current_build.is_empty() {
        parts.push(format!("(Build {})", current_build));
    }
    parts.join(" ")
}

#[link(name = "user32")]
unsafe extern "system" {
    fn OpenClipboard(h_wnd_new_owner: *mut std::ffi::c_void) -> i32;
    fn EmptyClipboard() -> i32;
    fn SetClipboardData(u_format: u32, h_mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn CloseClipboard() -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GlobalAlloc(u_flags: u32, dw_bytes: usize) -> *mut std::ffi::c_void;
    fn GlobalLock(h_mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn GlobalUnlock(h_mem: *mut std::ffi::c_void) -> i32;
    fn GlobalFree(h_mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
}

/// Copy text to the Windows Clipboard.
pub fn copy_text_to_clipboard(text: &str) -> std::io::Result<()> {
    unsafe {
        use std::ptr;
        if OpenClipboard(ptr::null_mut()) == 0 {
            return Err(std::io::Error::last_os_error());
        }
        if EmptyClipboard() == 0 {
            let _ = CloseClipboard();
            return Err(std::io::Error::last_os_error());
        }

        let text_w: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let len = text_w.len() * 2;
        let h_mem = GlobalAlloc(0x0002, len); // GMEM_MOVEABLE = 0x0002
        if h_mem.is_null() {
            let _ = CloseClipboard();
            return Err(std::io::Error::last_os_error());
        }

        let ptr = GlobalLock(h_mem);
        if ptr.is_null() {
            let _ = GlobalFree(h_mem);
            let _ = CloseClipboard();
            return Err(std::io::Error::last_os_error());
        }

        std::ptr::copy_nonoverlapping(text_w.as_ptr(), ptr as *mut u16, text_w.len());
        GlobalUnlock(h_mem);

        if SetClipboardData(13, h_mem).is_null() {
            // CF_UNICODETEXT = 13
            let _ = GlobalFree(h_mem);
            let _ = CloseClipboard();
            return Err(std::io::Error::last_os_error());
        }

        CloseClipboard();
    }
    Ok(())
}

/// Query process hierarchy to detect active Shell and Terminal Emulator.
pub fn query_shell_and_terminal() -> (String, String) {
    let mut shell = "Unknown Shell".to_string();
    let mut terminal = "Unknown Terminal".to_string();

    #[cfg(windows)]
    {
        use sysinfo::System;
        let mut sys = System::new_all();
        sys.refresh_all();

        let mut current_pid = sysinfo::get_current_pid().ok();
        let mut depth = 0;

        while let Some(pid) = current_pid {
            if depth > 12 {
                break;
            }
            if let Some(process) = sys.process(pid) {
                let name = process.name().to_lowercase();
                if shell == "Unknown Shell" {
                    if name.contains("powershell") || name.contains("pwsh") {
                        shell = "PowerShell".to_string();
                    } else if name == "cmd.exe" || name == "cmd" {
                        shell = "CMD".to_string();
                    } else if name.contains("bash") || name.contains("sh") || name.contains("zsh") {
                        shell = name.replace(".exe", "");
                    }
                }

                if terminal == "Unknown Terminal" {
                    if name.contains("windowsterminal") || name == "openconsole.exe" {
                        terminal = "Windows Terminal".to_string();
                    } else if name.contains("code") {
                        terminal = "VS Code Terminal".to_string();
                    } else if name.contains("alacritty") {
                        terminal = "Alacritty".to_string();
                    } else if name.contains("wezterm") {
                        terminal = "WezTerm".to_string();
                    } else if name.contains("conhost") {
                        terminal = "Windows Console Host".to_string();
                    }
                }

                current_pid = process.parent();
                depth += 1;
            } else {
                break;
            }
        }
    }

    (shell, terminal)
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct GlyphMap {
    pub status_ok: &'static str,
    pub status_err: &'static str,
    pub info: &'static str,
    pub warning: &'static str,
    pub clipboard: &'static str,
    pub play: &'static str,
    pub play_empty: &'static str,
}

impl GlyphMap {
    pub fn load() -> Self {
        let (_, terminal) = query_shell_and_terminal();
        if terminal == "Windows Console Host" {
            Self {
                status_ok: "[OK]",
                status_err: "[ERR]",
                info: "[i]",
                warning: "[!]",
                clipboard: "[CLIP]",
                play: " > ",
                play_empty: " - ",
            }
        } else {
            Self {
                status_ok: "✔️",
                status_err: "❌",
                info: "ℹ️",
                warning: "⚠️",
                clipboard: "📋",
                play: " ▶ ",
                play_empty: " ▷ ",
            }
        }
    }
}

/// Trigger a native Windows Toast Notification using a PowerShell/WinRT shim.
#[allow(dead_code)]
pub fn show_toast_notification(title: &str, message: &str) {
    let script = format!(
        "[void] [Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime]; \
         [void] [Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime]; \
         $el = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02); \
         $el.GetElementsByTagName('text').Item(0).InnerText = '{}'; \
         $el.GetElementsByTagName('text').Item(1).InnerText = '{}'; \
         $notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('rsav'); \
         $notifier.Show($el)",
        title.replace('\'', "''"),
        message.replace('\'', "''")
    );

    // Spawn powershell in the background to avoid blocking the main TUI thread.
    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
        .spawn();
}

#[cfg(windows)]
#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegisterEventSourceW(
        lp_unc_server_name: *const u16,
        lp_source_name: *const u16,
    ) -> *mut std::ffi::c_void;

    fn ReportEventW(
        h_event_log: *mut std::ffi::c_void,
        w_type: u16,
        w_category: u16,
        dw_event_id: u32,
        lp_user_sid: *mut std::ffi::c_void,
        w_num_strings: u16,
        dw_data_size: u32,
        lp_strings: *const *const u16,
        lp_raw_data: *mut std::ffi::c_void,
    ) -> i32;

    fn DeregisterEventSource(h_event_log: *mut std::ffi::c_void) -> i32;
}

/// Write a record directly to the native Windows Event Log under Application.
pub fn log_windows_event(source_name: &str, event_type: u16, event_id: u32, message: &str) {
    #[cfg(windows)]
    unsafe {
        let source_w: Vec<u16> = source_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let handle = RegisterEventSourceW(std::ptr::null(), source_w.as_ptr());
        if !handle.is_null() {
            let message_w: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();
            let strings: [*const u16; 1] = [message_w.as_ptr()];

            ReportEventW(
                handle,
                event_type,
                0, // category
                event_id,
                std::ptr::null_mut(), // user sid
                1,                    // num strings
                0,                    // data size
                strings.as_ptr(),
                std::ptr::null_mut(), // raw data
            );
            DeregisterEventSource(handle);
        }
    }
}

/// If the application is running in a pseudoconsole (like Windows Terminal) and we want it
/// to run as a standalone styled window, relaunch it inside conhost.exe.
pub fn relaunch_in_conhost_if_needed() {
    #[cfg(windows)]
    {
        // 1. Check if we have the --relaunched flag to prevent any potential loops
        let args: Vec<String> = std::env::args().collect();
        if args.iter().any(|arg| arg == "--relaunched") {
            return;
        }

        // 2. Detect if we are in conhost or a pseudoconsole (like Windows Terminal)
        let hwnd = unsafe { GetConsoleWindow() };
        let is_conhost = if hwnd.is_null() {
            false
        } else {
            use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;
            let mut rect: RECT = unsafe { std::mem::zeroed() };
            let ok = unsafe { GetWindowRect(hwnd, &mut rect) };
            let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) };
            ok != 0 && (rect.right - rect.left) > 0 && style != 0
        };

        if !is_conhost {
            // Relaunch in conhost.exe
            let current_exe = std::env::current_exe().unwrap();
            let mut cmd_args = vec![
                "/c".to_string(),
                "start".to_string(),
                "".to_string(),
                "conhost.exe".to_string(),
                current_exe.to_str().unwrap().to_string(),
            ];
            // Pass all original args, plus the --relaunched flag
            for arg in args.into_iter().skip(1) {
                cmd_args.push(arg);
            }
            cmd_args.push("--relaunched".to_string());

            let _ = std::process::Command::new("cmd.exe")
                .args(&cmd_args)
                .spawn();
            std::process::exit(0);
        }
    }
}


