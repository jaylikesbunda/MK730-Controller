# MK730 Controller

A small desktop app for controlling the Cooler Master MK730 keyboard.

## Features

- Paint keys and lightbar zones, adjust brightness, and apply lighting effects.
- Save and load five lighting profiles; import and export profiles as JSON.
- Create macros that run from a Windows hotkey while the app is open.
- Switch between light and dark themes.

Macros run on the PC and are not stored in the keyboard firmware.

## Build

Requires a current stable Rust toolchain and the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/).

```powershell
cd src-tauri
cargo tauri dev
cargo tauri build --bundles nsis
```

The Windows lighting interface is HID interface 1; if another driver has claimed it, bind that interface to WinUSB while leaving interface 0 unchanged.

## Project layout

- `frontend/` — static HTML, CSS, and JavaScript
- `src-tauri/` — Tauri and Windows HID backend
- `crates/mk730-core/` — protocol, keymap, profiles, and macro model
- `docs/` — protocol notes

## License

The application is MIT licensed. Protocol notes credit [libcmmk](https://github.com/chmod222/libcmmk) (LGPL-3.0).
