# trance

> A local screensaver host and picker.

`trance` is a single-binary TUI (and Windows screensaver host) that lists installed `.scr` files — the 10 `library`-powered scenes or any third-party `.scr` — and lets the user pick one. Once a scene is selected, `trance` launches it as a fullscreen screensaver using the OS's standard screensaver API.

`trance` is part of the [local76](https://github.com/local76/local76) ecosystem and depends on [`library`](https://github.com/local76/library) for the design system and TUI widgets.

---

## Features

- **Built-in registry.** A `registry.json` lists the 10 known scenes with their official download URLs. Out-of-the-box experience.
- **Auto-discovery.** Scans `%WINDIR%\System32` and `%LOCALAPPDATA%\local76\screensavers` for any `.scr` file and adds it to the picker.
- **TUI picker.** Ratatui list with live preview pane. Arrow keys, Enter to launch, `d` to download a missing scene from the registry, `r` to refresh.
- **Windows screensaver host.** Registers itself as the active screensaver via the standard `HKCU\Control Panel\Desktop\SCRNSAVE.EXE` registry key. The Windows "Screen Saver Settings" control panel launches `trance.scr` directly.
- **`trance doctor`.** Diagnostics: registry state, scene directory, missing dependencies.

---

## Install

### Windows
- **Standalone**: download `trance.scr` from the [latest release](https://github.com/local76/trance/releases). Right-click → Install.
- **winget**: `winget install local76.trance`
- **MSI**: download the `.msi` from the releases page.

### Linux (companion CLI)
- **Debian/Ubuntu**: `sudo dpkg -i trance.deb`
- **Red Hat/Fedora**: `sudo rpm -i trance.rpm`
- **Arch (AUR)**: `yay -S trance-bin`

On Linux, `trance` operates as a TUI picker only. The screensaver-runtime side is provided by the `screensavers` workspace.

---

## Usage

```
trance                     # launch the picker TUI
trance.scr /s              # fullscreen mode (Windows screensaver entry point)
trance.scr /c              # configure mode (Windows screensaver entry point)
trance.scr /p <hwnd>       # preview mode (Windows screensaver entry point)
trance list                # one-shot: print installed scenes to stdout
trance doctor              # diagnostics
trance --version
trance --help
```

Inside the TUI:

| Key | Action |
|---|---|
| `↑` / `↓` | Move selection |
| `Enter` | Launch the selected scene |
| `d` | Download a missing scene from the registry |
| `r` | Refresh the scene list |
| `q` | Quit |

---

## Configuration

- **Windows**: `%APPDATA%\trance\config.yaml` and `%APPDATA%\trance\registry.json` (auto-refreshed).
- **Linux**: `~/.config/trance/config.yaml`.

The default registry ships with the 10 known scenes; if a scene is missing locally, the picker shows a download link.

---

## Build from source

```pwsh
git clone https://github.com/local76/trance.git
cd trance
cargo build --release
```

The release artifact is `target/release/trance.scr` (Windows) or `target/release/trance` (Linux).

---

## License

MIT. See [LICENSE.md](LICENSE.md).
