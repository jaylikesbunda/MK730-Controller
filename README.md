# MK730 Controller

Open-source Tauri controller for Cooler Master MK730 (MasterKeys family) — no Portal required for lighting/profiles. Macros store locally until firmware RE lands.

![platform](https://img.shields.io/badge/platform-linux%20%7C%20windows-blue) ![tauri](https://img.shields.io/badge/tauri-v2-purple) ![license](https://img.shields.io/badge/license-MIT-green)

## Features

- **Lighting**: full-board (`c0 00`), per-key (`c0 01`), colormap push (`c0 02`), TKL visual editor
- **Effects**: hardware effects `51 28` + params `51 2c` (speed/direction/colors), multilayer flag
- **Profiles**: P1–P5, `51 00` switch, `50 55` save to firmware, JSON export/import, apply-to-device sequence
- **Macros**: local-first recorder/editor (trigger + HID events + delays + repeat). Firmware sync = `pending_capture`, see `docs/RE_MACROS.md`
- **Device**: VID `0x2512` scan via hidapi+rusb, mode switch `41 00..03`, hex logs for capture diffing
- **Theme**: dark minimal rounded, custom top bar with integrated window controls, `decorations:false + transparent:true`

## Quick start

### Linux

```bash
lsusb -v -d 2512:   # confirm PID, add to CANDIDATE_PIDS in crates/mk730-core/src/lib.rs
sudo cp udev/99-mk730.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger
cargo test -p mk730-core
cargo check --manifest-path src-tauri/Cargo.toml
# full launcher (needs Tauri system deps):
# sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf libusb-1.0-0-dev libudev-dev
cargo tauri dev -- --port 1420
```

### Windows

```powershell
cargo test -p mk730-core
cargo check --manifest-path src-tauri/Cargo.toml
cargo tauri dev
# If IF1 claim fails: Zadig -> bind Interface 1 only to WinUSB, keep IF0 HID.
```

## Repo layout

```
frontend/            # static UI (custom top bar, dark rounded theme)
src-tauri/           # Tauri 2 backend (hidapi+rusb, commands)
crates/mk730-core/   # pure protocol (proto/keymap/profiles/macros), no system deps
udev/                # Linux permissions
docs/                # PROTOCOL + RE_MACROS capture guide
scripts/capture/     # Wireshark filter cheat-sheet
.github/workflows/   # ci + release (version input)
```

## Protocol

See `docs/PROTOCOL.md` (derived from `chmod222/libcmmk` PROTOCOL.md, LGPL-3.0, archived 2026). Summary: 64B interrupt on IF1 EP `0x04` OUT.

## Release

`Actions -> Release -> Run workflow -> version: 0.2.0` builds Linux + Windows bundles and publishes a GitHub Release. See `.github/workflows/release.yml`.

## License

MIT. Protocol docs credit `chmod222/libcmmk` (LGPL-3.0).
