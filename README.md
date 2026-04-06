# corsair-headset

macOS menu bar app for Corsair wireless headsets. Shows battery, controls EQ/sidetone/LEDs. No vendor software needed.

## Install

```
brew tap jlevere/tap
brew install --cask corsair-headset
xattr -cr /Applications/Corsair\ Headset.app
```

To start at login: System Settings → General → Login Items → add Corsair Headset.

### From source

```
nix build .#app
nix flake check
```

## What it does

- Battery % in menu bar (hides when dongle is unplugged)
- EQ presets (Pure Direct, Bass Boost, Clear Chat, FPS Competition, Movie Theater)
- LED color control
- Sidetone on/off
- Auto-sleep timer
- Low battery notifications
- Reconnects automatically on dongle replug
- Settings persist in `~/Library/Application Support/Corsair Headset/settings.toml`

## Supported devices

Tested on VOID RGB Elite Wireless. Should work with other Legacy protocol headsets (VOID Pro, HS60, HS70). Bragi protocol devices (HS80, HS55) have protocol support but need hardware testing.

## Crates

```
corsair-proto      HID protocol codec (Legacy, Bragi, CxAudio)
corsair-transport  async hidapi transport
corsair-tray       menu bar app
corsair-cli        command-line tool
```
