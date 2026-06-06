rSaver Windows Screensaver Manager

You can install rSaver globally via the Windows Package Manager (WinGet):
winget install TourianDynamics.rsav

What's New (Current Release)
This release aligns the visual styling, interfaces, and experience of rSaver and rMonitor under a unified dashboard design system:
  Standardized Borders: Separate bordered blocks for each UI section, removing the outer screen-wide border.
  Unified Title Bar: Standardized header layout displaying App Name, user@host, and OS & Build.
  Standard Text Selection: Restored terminal text selection by disabling raw crossterm mouse capture.
  Console Tab Titles: Sets the active tab title to rSav on startup and restores it upon exit.
  Clean Status Bar: Bottom status bar has a matching accent-colored Status border title and clean, bold-colored feedback messages.
  Interactive Status Feedback: Status bar dynamically reports focus transitions and descriptions when navigating using Tab.

What rSaver Does
rSaver manages the native Windows Screen Saver system by bridging standard OS-level registry settings with modern terminal-based configuration. Key capabilities include:
  Automatic Discovery: Scans Windows system folders (System32, SysWOW64, etc.) and a dedicated user folder in %APPDATA%\rSaver\screensavers for .scr executables.
  Config Sync Alerts: Automatically monitors registry changes in the background and hot-reloads rSaver if the system screensaver is modified externally (for example, via the native Windows Settings).
  Prevent System Sleep: Easily toggle system sleep prevention on/off directly from the dashboard (useful for presentations, downloads, or simply keeping custom screensavers running indefinitely).
  High-DPI Scaling & Aesthetics: Dynamically resizes the layout to a compact 110x38 terminal window and adopts the Windows accent color for highlighted interfaces.
  Curated Screensaver Catalog: Discover, download, and manage a collection of curated retro terminal screensavers directly from the TUI interface.

Curated Screensaver Collection
rSaver comes integrated with a catalog of retro terminal-style screensavers optimized for Windows 11 (now fully dynamic with live OS name + kernel logos pulled from the system):
  beams: Sweeping vertical colored spotlights in purple, blue, and pink. Features live dynamic OS name and kernel subtext.
  bhop: Cyberpunk Neon Pink and Cyan Hacker TUI with scrolling live Bhop game panel and full system diagnostics (including GPUs and monitors).
  fire: Majestic Doom fire simulation sweeping upward, reacting to system load with a live dynamic OS logo.
  fireworks: Cozy city skyline silhouette with warm lit windows and exploding fireworks illuminating the live OS logo.
  life: Conway's Game of Life simulated on a grid with nebula trails, interacting with the live dynamic OS logo.
  matrix: Dense Matrix-style falling digital rain using system environment variables and live system data, over the dynamic OS logo.
  party: Rave party simulation featuring twinkling background stars, a disco ball, and the live dynamic OS logo.
  pour: Waterfall cascade of characters (including live system data) assembling the dynamic OS logo and kernel.
  unstable: A dynamic OS logo exploding outward into chaotic particles (with new explosion types and side effects) and snapping back.

CLI Subcommands and Flags
rSaver acts as both a dashboard and a screensaver command-line handler.
rsav.exe [OPTIONS] [COMMAND]

Options:
  --theme <THEME> : Force a specific TUI theme (dark, light, high-contrast, no-color).

Commands:
  tui : Launch the interactive TUI dashboard (default when run without arguments).
  run : Launch the currently active screensaver in fullscreen mode (rsav run).
  stop : Kill any active screensavers running on the system.
  toggle-active : Enable or disable the screensaver timeout system-wide.
  doctor : Verify registry access, discovery folders, and log files. Pass --fix to auto-heal missing configuration assets.

TUI Keyboard Controls
Use the following shortcuts to navigate the dashboard:
  Tab / Shift-Tab : Cycle focus between Global Preferences and the Screensaver List.
  Up / Down or k / j : Navigate lists and settings.
  Enter : Trigger selection, toggle checkboxes, or open configurations.
  Space : Preview the highlighted screensaver in fullscreen.
  a : Apply the highlighted screensaver as the active system-wide screensaver.
  c : Open the custom settings dialog for the highlighted screensaver (if supported).
  f : Toggle search filtering on the screensaver list.
  d : Download and install the selected screensaver from the catalog.
  q / Esc : Quit rSaver or close active overlay popups.

Custom Preferences and Data Storage
All data is stored locally under your Windows user profile:
  rSaver Custom Preferences: Stored at %APPDATA%\rSaver\config.yaml (contains last-selected screensaver, prevent-sleep status, custom cycle interval, and catalog feed URLs).
  Screensaver Drop Path: Put custom .scr screensavers in %APPDATA%\rSaver\screensavers to have rSaver discover them.
  Logs File: Diagnostics are written to %APPDATA%\rSaver\rSaver.log so they do not clutter raw terminal outputs.

Custom Feeds:
To add custom online registry catalogs, open %APPDATA%\rSaver\config.yaml (or ~/.config/rSaver/config.yaml on Linux) and add your feed URLs separated by semicolons:
feed_urls: https://raw.githubusercontent.com/tourian-dynamics/rSaver/master/registry.json;https://example.com/custom-screensavers.json

Cross-platform support:
rSaver detects the current OS (`current_platform()`) and selects the right asset from the "downloads" map in the registry.
- Windows: .scr binary
- Linux: Picks the best variant from the downloads map:
  - Prefers `linux-deb` when dpkg/apt detected
  - Then `linux-rpm` when rpm/dnf/yum/zypper detected
  - Then `linux-arch` (`.pkg.tar.zst`) when pacman detected
  - Falls back to raw `linux` ELF otherwise
  For the raw ELF, rSaver downloads it, installs the executable to `~/.xscreensaver/<name>`, chmod +x, and generates a minimal .xml descriptor under `~/.xscreensaver/config/<name>.xml` (option B — rSaver generates client-side so the published linux/ folder stays a simple collection of ELF + convenience packages).
- Packages (.deb / .rpm / .pkg.tar.zst): downloaded to the local rSaver cache (`~/.local/share/rSaver/screensavers/` or XDG equivalent). A `<name>.install.txt` sidecar is written with the exact command, and the TUI status bar shows the one-liner (e.g. `sudo pacman -U ...`). Yes, we include Arch (linux-arch + .pkg.tar.zst).

This design lets a single `screensavers/<style>/linux/` folder in rScreensavers contain the portable ELF + the three package formats. No tarballs.

Building From Source
Ensure you have the Rust compiler toolchain installed on Windows.

To build, clone the repository and navigate to the folder:
cd rSaver

Then build the release binary:
just build

The optimized binary will be compiled to target/release/rsav.exe. You can rename this to rsav.scr to install it directly as a Windows screensaver!
