# trance

> A local screensaver host and picker.

`trance` is a single-binary app (and Windows screensaver host) that
lists installed `.scr` files — the 10 library-powered scenes or any
third-party `.scr` — and lets the user pick one. Once a scene is
selected, `trance` launches it as a fullscreen screensaver using the
OS's standard screensaver API.

`trance` is part of the [local76](https://github.com/local76/local76)
ecosystem and depends on [`library`](https://github.com/local76/library)
for the design system and widgets.

---

## Features

- **Built-in registry.** A `registry.json` lists the 10 known scenes
  with their official download URLs. Out-of-the-box experience.
- **Auto-discovery.** Scans `%WINDIR%\System32` and
  `%LOCALAPPDATA%\local76\screensavers` for any `.scr` file and adds
  it to the picker.
- **Picker.** Ratatui list with live preview pane. Arrow keys, Enter
  to launch, `d` to download a missing scene from the registry, `r`
  to refresh.
- **Windows screensaver host.** Registers itself as the active
  screensaver via the standard
  `HKCU\Control Panel\Desktop\SCRNSAVE.EXE` registry key. The Windows
  "Screen Saver Settings" control panel launches `trance.scr`
  directly.
- **`trance doctor`.** Diagnostics: registry state, scene directory,
  missing dependencies.

---

## Install

### Windows
- **Standalone**: download `trance.scr` from the
  [latest release](https://github.com/local76/trance/releases).
  Right-click → Install.

### Linux (companion CLI)
- **Debian/Ubuntu**: `sudo dpkg -i trance.deb` (downloaded from the
  release page)

On Linux, `trance` operates as a picker only. The screensaver runtime
side is provided by the 10 `screensaver-*` repos.

---

## Usage

```
trance                     # launch the picker
trance.scr /s              # fullscreen mode (Windows screensaver entry point)
trance.scr /c              # configure mode (Windows screensaver entry point)
trance.scr /p <hwnd>       # preview mode (Windows screensaver entry point)
trance list                # one-shot: print installed scenes to stdout
trance doctor              # diagnostics
trance --version
trance --help
```

Inside the picker:

| Key | Action |
|---|---|
| `↑` / `↓` | Move selection |
| `Enter` | Launch the selected scene |
| `d` | Download a missing scene from the registry |
| `r` | Refresh the scene list |
| `q` | Quit |

---

## Configuration

- **Windows**: `%APPDATA%\local76\app\trance\config.yaml` and
  `%APPDATA%\local76\app\trance\registry.json` (auto-refreshed).
- **Linux**: `~/.config/local76/app/trance/config.yaml`.

The default registry ships with the 10 known scenes; if a scene is
missing locally, the picker shows a download link.

---

## Build from source

```pwsh
git clone https://github.com/local76/trance.git
cd trance
cargo build --release
```

The release artifact is `target/release/trance.scr` (Windows) or
`target/release/trance` (Linux).

---

## License

MIT. See [LICENSE.md](LICENSE.md).
