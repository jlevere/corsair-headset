# Corsair Headset

A lightweight macOS menu bar app for Corsair wireless headsets. Built in Rust, no iCUE required.

![menu bar](https://img.shields.io/badge/macOS-menu%20bar-black) ![size](https://img.shields.io/badge/binary-2.5MB-green) ![license](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)

## Why

iCUE is a ~1GB Qt application that installs a kernel extension, an audio daemon, and three background services just to show your battery level and adjust sidetone. This replaces all of that with a single 2.5MB binary.

## Features

- **Battery status** in the menu bar — solid icon when connected, outline when not
- **Sidetone control** — mic passthrough so you don't sound like you're wearing earmuffs
- **EQ presets** — Pure Direct, Bass Boost, Clear Chat, FPS Competition, Movie Theater
- **Mic mute** toggle
- **Auto-sleep** — host-controlled inactivity shutdown with configurable timeout
- **Battery alerts** — native macOS notifications at 15% and 5%
- **Auto-reconnect** — survives dongle unplug/replug, no restart needed
- Refreshes on click — always shows current data when you open the menu
- Polls every 30s when active, backs off to 2min when idle

## Supported Devices

Currently tested on **VOID RGB Elite Wireless**. The protocol crate covers:

| Protocol | Devices | Status |
|----------|---------|--------|
| Legacy | VOID, VOID Pro, HS60, HS70 | Implemented + tested on hardware |
| Bragi | HS80, HS55 Wireless Core | Protocol reversed, needs hardware testing |
| CxAudio | CX20805 (CAPE) chip access | Implemented |

165 Corsair devices cataloged with VID/PIDs from extracted manifests.

## Install

Download the `.dmg` from [Releases](../../releases), open it, drag **Corsair Headset** to Applications.

The app is unsigned, so macOS will block it on first run. Fix with:
```bash
xattr -cr /Applications/Corsair\ Headset.app
```
Or: right-click the app → Open → Open (bypasses Gatekeeper once).

To start at login: System Settings → General → Login Items → add Corsair Headset.

### Build from source

Requires Rust toolchain, `hidapi`, and `pkg-config`:

```bash
brew install hidapi pkg-config
make app          # builds target/release/Corsair Headset.app
make dmg          # creates .dmg for distribution
```

## Architecture

```
corsair-proto      Protocol codec — Legacy, Bragi, CxAudio (68 tests)
corsair-transport  Async HID transport — hidapi native + WebHID (planned)
corsair-device     Device session management (planned)
corsair-tray       macOS menu bar app
corsair-cli        Command-line tool
corsair-web        WASM browser app (planned)
```

## License

MIT OR Apache-2.0
