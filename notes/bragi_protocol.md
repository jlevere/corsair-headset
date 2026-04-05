# Bragi Protocol Reverse Engineering Notes

## Overview

Bragi is Corsair's newer property-based HID protocol used by HS80, HS55 Wireless Core,
and newer devices. Unlike the Legacy protocol's report-type-based commands, Bragi uses
a generic property read/write interface where each property is identified by a numeric
`PropertyId`.

## Key Classes

- `bragiprotocol::Properties` — QMetaObject with all property types
- `bragiprotocol::Property` — raw property value container (`rawData()` returns bytes)
- `bragiprotocol::PropertyId` — integer enum identifying each property
- `bragiprotocol::ReportOptions` — timing configuration per-request
- `bragiprotocol::detail::PropertyConversion<T>` — type-specific byte encode/decode
- `bragi::PropertyProvider` — interface for property read/write on a device
- `bragi::BragiAudioDeviceComponent` — audio features (sidetone, mic, EQ)
- `bragi::BragiSidetoneAudioDeviceComponent` — sidetone-specific variant

## Timing Constants

| Name | Value | Usage |
|------|-------|-------|
| `waitTime` | 20ms | Standard property read/write |
| `waitTimeLong` | 750ms | Longer operations |
| `waitTimeVeryLong` | 2000ms | Firmware operations |
| `repeatReport` | configurable | Retry count with delay |

## Value Encoding

All percentage values (sidetone, mic level, battery) use `u16 = percent * 10`:

| Conversion | Formula | Range |
|-----------|---------|-------|
| `toBragiSidetoneLevel(int)` | `level * 10` as u16 | 0–1000 |
| `fromBragiSidetoneLevel(u16)` | `value / 10` | 0–100 |
| `toBragiMicrophoneLevel(int)` | `level * 10` as u16 | 0–1000 |
| `fromBragiMicrophoneLevel(u16)` | `value / 10` | 0–100 |
| `toBatteryLevelPercent(u16)` | `value / 10` | 0–100 |

## Known PropertyId Values

Extracted from disassembly of `BragiAudioDeviceComponent` methods:

| PropertyId | Hex | Type | Description |
|-----------|-----|------|-------------|
| 0x46 | 70 | bool/u8 | Sidetone mute state |
| 0x47 | 71 | u16 | Sidetone level (percent * 10) |
| 0x8E | 142 | bool/u8 | Microphone mute state |
| 0x8F | 143 | u16 | Microphone level (percent * 10) |

### PropertyIds yet to extract (from symbol names)

These properties are referenced in conversion helpers but PropertyIds not yet confirmed:
- Battery status (`BatteryStatus` enum: values 1–3, subtract 1 for index)
- Battery level (u16, percent * 10)
- Wireless mode (`WirelessModeValue`)
- DPI stage index
- Lift height (`LiftHeightValue`)
- Host operating system
- Wireless connection configuration
- Button debounce algorithm
- Indication behavior
- Gamepad trigger/thumbstick sensitivity
- Color (RGB)
- Physical keyboard layout
- Angle
- Timeout (as chrono duration)

## Property Types (from `Properties::` inner types)

| Type | Description |
|------|-------------|
| `BatteryStatus` | Enum with 3+ values |
| `WirelessModeValue` | Wireless connection mode |
| `WirelessConnectionConfiguration` | Set of wireless configs |
| `LiftHeightValue` | Mouse lift height |
| `PhysicalKeyboardLayout` | Keyboard layout ID |
| `Color` | RGB color struct |
| `HostOperatingSystem` | OS identification |
| `ButtonDebounceAlgorithmValue` | Debounce mode |
| `IndicationBehaviorValue` | LED indication behavior |
| `TriggerThumbstickSensitivityValue` | Gamepad sensitivity |
| `HardwareActionFeaturesFlags` | Bitset<32> of HW action capabilities |
| `EnabledMouseSensorResolutionStages` | DPI stage bitmap with sniper mode |

## Notification System

Property changes trigger callbacks with signature:
`void(bragiprotocol::PropertyId, bragiprotocol::Property)`

`BragiAudioDeviceComponent` subscribes to 4 property changes (lambdas $_71–$_74):
- $_71: likely mic level changes
- $_72: likely mic mute changes
- $_73: likely sidetone level changes
- $_74: likely sidetone mute changes

## Protocol Transport

The Bragi protocol uses HID reports but with a different framing than Legacy:
- Property read: sends PropertyId, receives Property value
- Property write: sends PropertyId + new value
- vtable[0x20] = read property
- vtable[0x28] = write property
- `ReportOptions` controls per-request timing

## Next Steps

To get the complete PropertyId enum (50+ properties expected), need to analyze:
1. `libiCUE.dylib` (~50MB) — contains the main Bragi device features
2. The Bragi subdevice classes (HeadsetLogicalSubdevice, etc.)
3. The Properties QMetaObject string table (has all enum value names)
4. More component classes in libiCUE for keyboard/mouse Bragi features
