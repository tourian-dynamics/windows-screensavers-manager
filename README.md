# Windows Screensavers Manager (WSM)

A lightweight, modern Windows Screen Saver Management TUI dashboard built in Rust. WSM offers a centralized controller for discovering, previewing, configuring, and cycling screensavers on mixed-DPI multi-monitor environments without touching intrusive registry editors.

```
+================================================================+
| WINDOWS SCREENSAVERS MANAGER  (WSM)                            |
| Version 0.8.0                                                 |
+----------------------------------------------------------------+
|  Global System Preferences                                     |
|  ▶ Active:         ACTIVE                                      |
|    Timeout:        10 minutes                                  |
|    Prevent sleep:  DISABLED (NORMAL)                           |
|    Cycle time:     30 seconds                                  |
+================================================================+
```

---

## 📥 Installation

You can install WSM globally via the Windows Package Manager (WinGet):

```powershell
winget install TourianDynamics.WSM
```

---

## 📖 What WSM Does

WSM manages the native Windows Screen Saver system by bridging standard OS-level registry settings with modern terminal-based configuration. Key capabilities include:

* **Modern TUI Dashboard**: Real-time console interface utilizing [ratatui](https://crates.io/crates/ratatui) and [crossterm](https://crates.io/crates/crossterm).
* **Automatic Discovery**: Scans Windows system folders (`System32`, `SysWOW64`, etc.) and a dedicated user folder in `%APPDATA%\wsm\screensavers` for `.scr` executables.
* **Extensible Catalog Feeds**: Loads curated screensavers from customizable remote or local JSON feeds.
* **Background Downloader**: Renders an interactive Pacman animation track in the TUI when installing new screensavers from the catalog feeds.
* **Config Sync Alerts (Out-of-Sync Detection)**: Automatically monitors registry changes in the background and hot-reloads WSM if the system screensaver is modified externally (e.g., via the native Windows Settings).
* **Seamless Transitions**: Applies a topmost Win32 borderless black overlay mask for 300ms during screensaver cycle transitions to eliminate desktop/terminal flash.
* **Sleep Inhibition**: Temporarily keeps the display/system awake with `SetThreadExecutionState` when activated in preferences.
* **Color Harmonization**: Automatically queries Windows high-contrast state, active accent colors, dark mode, and console palettes to dynamically style the TUI to match your OS theme context.
* **Vanity Mode**: Toggle retro terminal firework animations on demand (off by default, toggled via `V`) that explode whenever selection preferences are modified.

---

## 📺 Curated Screensavers (Catalog Tools)

WSM comes integrated with a catalog of retro terminal-style screensavers optimized for Windows 11. Here is what each tool does:

1. **Win-beams**
   * *What it does*: Renders sweeping vertical colored spotlights in purple, blue, and pink across the screen, creating a relaxed, stage-lighting effect.
2. **Win-bhop**
   * *What it does*: Displays a cyberpunk neon pink and cyan hacker dashboard featuring a scrolling live bunny-hopping (Bhop) game simulation panel.
3. **Win-fire**
   * *What it does*: Simulates a classic, retro Doom-style fire system sweeping upwards from the bottom of the screen with smooth flame color transitions.
4. **Win-fireworks**
   * *What it does*: Renders a cozy, dark city skyline silhouette with warm, randomly lit windows and colorful exploding fireworks overhead.
5. **Win-life**
   * *What it does*: Simulates Conway's Game of Life on a pixel grid, leaving glowing nebula trails as cells evolve and die.
6. **Win-matrix**
   * *What it does*: Renders a dense, scrolling Matrix-style digital rain of falling characters, dynamically populated using system environment variables.
7. **Win-party**
   * *What it does*: Simulates a rave/party scene featuring a rotating disco ball, neon equalizers, and twinkling background stars.
8. **Win-pour**
   * *What it does*: Cascades terminal characters downward in a waterfall stream, assembling the classic Windows ASCII logo flag piece-by-piece.
9. **Win-unstable**
   * *What it does*: Features a purple Windows logo that suddenly explodes outward into a cloud of chaotic physics particles before snapping back together.

---

## ⚙️ Subcommands & CLI Reference

WSM acts as both a dashboard and a screensaver command-line handler.

```bash
wsm.exe [OPTIONS] [COMMAND]

Options:
  --theme <THEME>  Force TUI theme: dark, light, high-contrast, no-color
```

### Commands:
* `tui` or `configure` (or no command): Launch the interactive TUI configuration manager (default).
* `run` or `start` or `/s`: Launch the currently active screensaver fullscreen.
* `stop`: Kill all running screensavers discovered on the system.
* `toggle-active`: Toggle whether the screensaver is enabled system-wide.
* `lock`: Lock the Windows workstation first, then immediately launch the active screensaver.
* `preview <HWND>` or `/p:<HWND>`: Render a preview of the active screensaver inside a specific host window (used by Windows Screen Saver Settings).
* `doctor`: Run diagnostic report checking registry readability, file paths, logs, and directory structures.
  * Use **`--fix`** to automatically correct invalid registry paths, recreate missing app folders, or fix invalid screensaver selections.

---

## ⌨️ TUI Keybindings

Navigate and configure your preferences dynamically using the keyboard:

| Key | Action |
| :--- | :--- |
| **`Tab`** / **`BackTab`** | Cycle focus between **Global System Preferences** and **Screen Saver Preferences** |
| **`↑ / ↓`** | Navigate fields in preferences or entries in the screensaver list |
| **`← / →`** | Adjust Screensaver Timeout (when Timeout highlighted) or Cycle Time (when Cycle time highlighted) |
| **`Space`** | Toggle preferences (Active state, Prevent sleep) or check/uncheck screensavers in the list |
| **`Enter`** | Apply current selection configuration to the registry (sets cycle of checked options if >1 checked) |
| **`/`** | Open filter search input (type to filter screensavers, press `Esc` to clear) |
| **`F5`** / **`R`** | Re-scan the system and `%APPDATA%` directories for new screensavers |
| **`P`** | Launch a full-screen preview of the highlighted screensaver |
| **`V`** | Toggle Vanity Mode (off by default; displays interactive fireworks upon selection/apply) |
| **`q / Esc`** | Quit WSM |

---

## 📂 File & Configuration Paths

* **System Preferences**: Read and written to standard registry values under `HKCU\Control Panel\Desktop` (`SCRNSAVE.EXE`, `ScreenSaveActive`, `ScreenSaveTimeOut`).
* **WSM Custom Preferences**: Stored at `%APPDATA%\wsm\config.yaml` (contains last-selected screensaver, prevent-sleep status, custom cycle interval, and catalog feed URLs).
* **Screensaver Drop Path**: Put custom `.scr` screensavers in `%APPDATA%\wsm\screensavers` to have WSM discover them.
* **Logs File**: Diagnostics are written to `%APPDATA%\wsm\wsm.log` so they do not clutter raw terminal outputs.

### Custom Catalog Feeds in `config.yaml`
To add custom online registry catalogs, open `%APPDATA%\wsm\config.yaml` and add your feed URLs separated by semicolons:
```yaml
feed_urls: https://raw.githubusercontent.com/tourian-dynamics/windows-screensavers-manager/master/registry.json;https://example.com/custom-screensavers.json
```

---

## 🌐 Environment Variables

* **`NO_COLOR`**: Set `NO_COLOR=1` to disable styling colors and fall back to monochromatic black & white.
* **`RUST_LOG`**: Set `RUST_LOG=debug` or `RUST_LOG=trace` to adjust logging verbosity in `wsm.log`.

---

## 🛠️ Build Guide

Ensure you have Rust and Cargo installed.

```bash
# Clone the repository
git clone <repository-url>
cd windows-screensavers-manager

# Build debug binary
cargo build

# Build optimized release binary
cargo build --release
```

The optimized binary will be compiled to `target/release/wsm.exe`. You can rename this to `wsm.scr` to install it directly as a Windows screensaver!
