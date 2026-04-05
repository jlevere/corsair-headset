# iCUE 4.33.138 Reverse Engineering Findings

## Overview

Corsair iCUE is a modular Qt5 application with a plugin-based device architecture. All
protocol logic lives in `libiCUE.dylib` (~50MB) and `libLegacyProtocols.dylib` (7.1MB).
Device-specific dylibs (241 of them) are pure manifest/config plugins containing embedded
JSON manifests (`.cuebmf` format) — all device-specific behavior is data-driven.

**Corsair USB VID**: `0x1B1C` (6940), legacy VID: `0x170D` (5901)

## Three Protocol Generations

### 1. Legacy Headset Protocol (Avnera/CMA chip-based)
Used by: VOID series, HS60, HS70, older headsets.
- Enumerator: `Legacy`
- HidProto: `Headset` or `Generic_wireless_headset`
- Transport: HID reports via IOKit (`IOHIDDeviceSetReport` / `IOHIDDeviceGetReport`)

### 2. Bragi Protocol (newer generation)
Used by: HS80, HS55 Wireless Core, newer devices.
- Enumerator: `Hid`
- HidProto: `Bragi_headset`
- Transport: Property-based HID protocol
- Timing: 20ms standard, 750ms long, 2000ms very long wait
- Key classes: `bragi::PhysicalHidDevice`, `bragiprotocol::Properties`, `bragi::DeviceIoAdapter`
- Logical subdevices: `HeadsetLogicalSubdevice`, `DongleLogicalSubdevice`

### 3. NXP Protocol (keyboards/mice)
Used by: keyboards, mice, mousepads
- `NxpHidIoBase` with `nxp_msg::ReportId` / `nxp_msg::CommandId`
- Subclasses: `NxpKeyboardHidIo`, `NxpMouseHidIo`, `NxpMousePadHidIo`, `NxpRfHidIo`

### 4. CxAudio HID (Conexant audio chips)
Low-level transport beneath the feature layer for audio chips:
- `CxAudioHidDev20562` (CX20562 chip)
- `CxAudioHidDev20805` (CX20805/CAPE chip)
- Direct register/EEPROM/memory access
- C API: `CxAudioHidEnumerate()`, `CxAudioHidDevConnect()`, etc.

---

## Legacy Headset HID Protocol

### Input Reports (Device -> Host)

| Report ID | Enum | Layout | Purpose |
|-----------|------|--------|---------|
| 0x64 (100) | 0 | `StateReportLayout` | Battery, mic boom, buttons |
| 0x65 (101) | 1 | `DeviceModeReportLayout` | Operating mode notification |
| 0x66 (102) | 2 | `FirmwareVersionReportLayout` | FW version (tx + rx) |

### Output/Feature Reports (Host -> Device)

| Report ID | Enum | Layout | Purpose |
|-----------|------|--------|---------|
| 0xC8 (200) | 3 | `DeviceModeReportLayout` | Set operating mode |
| 0xC9 (201) | 4 | `RequestDataReportLayout` | Request data from device |
| 0xCA (202) | 5 | `SetValueReportLayout` | Set value by ValueId enum |
| 0xCB (203) | 6 | `DirectLedControlReportLayout` | Direct LED color control |
| 0xCB (203) | 7 | `CmaDirectLedControlReportLayout` | CMA-variant LED control |
| 0xCC (204) | 8 | `SetPowerStateReportLayout` | Shutdown/reset |
| 0xFF (255) | 9 | `SetAutoShutdownReportLayout` | Auto-shutdown timer |
| 0xFF (255) | 10 | `SetSidetoneLevelReportLayout` | Sidetone level |
| 0xFF (255) | 12 | `StartPairingReportLayout` | Initiate wireless pairing |
| 0xFF (255) | 13 | `LinkStateNotifyReportLayout` | Link state notification |

All reports have minimum 4-byte payload.

### State Report (0x64) Layout
```
Byte 0:    Pressed buttons bitmap
Byte 1:    Battery level (bits 0–6 = 0–127%), bit 7 = mic boom (0=up, 1=down)
Byte 2:    LinkState enum (lower nibble, bits 0–3)
Byte 3:    BatteryState enum (lower 3 bits, masked & 0x07)
```

### BatteryState Enum (from QMetaObject)
| Value | Name | Description |
|-------|------|-------------|
| 0 | Invalid | Unknown/not reported |
| 1 | Ok | Normal |
| 2 | Low | Low battery |
| 3 | Shutdown | Critical/shutting down |
| 4 | ChargeComplete | Fully charged |
| 5 | Charging | Currently charging |
| 6 | Error | Battery error |

### LinkState Enum (from QMetaObject)
| Value | Name |
|-------|------|
| 0 | Invalid |
| 1 | Active |
| 2 | Pair |
| 3 | Search |
| 4 | Standby |
| 5 | ReSync |
| 6 | InitialScan |
| 7 | TestMode |
| 8 | PairCancel |
| 9 | ActiveSearch |
| 11 | ActivePair |
| 12 | SearchViaSynchro |
| 13 | PairViaSynchro |

### Firmware Version Report (0x66) Layout
```
Byte 0-1:  Transmitter firmware version (major, minor)
Byte 2-3:  Receiver firmware version (major, minor)
```

### Device Mode Report (0x65/0xC8) Layout
```
Byte 0:    OperatingMode enum (0=Hardware, 1=Software)
Byte 1:    bit 0 = media events disabled flag
```

### ValueId Enum for SetValue Report (0xCA)
| Value | Name | Used By |
|-------|------|---------|
| 0 | Invalid | — |
| 1 | EqIndex | notifyActiveEqPresetIndex |
| 2 | SurroundState | notifySurroundState |
| 3 | MicState | setMicMute |
| 4 | AudioIndicationsState | setAudioIndicationsMute |
| 5 | SidetoneState | setSidetoneMute |
| 6 | SurroundIndicatorState | setSurroundIndicatorState |

### Set Value Report (0xCA) Layout
```
Byte 0:    ValueId enum (which parameter)
Byte 1+:   Value data
```

### Sidetone Level (Report ID 0xFF, type 10)
11-byte payload with sub-command header:
```
Bytes 0–7:  0B 00 FF 04 0E 01 05 01  (header from magic constant 0x0105010e04ff000b)
Bytes 8–9:  04 00  (sub-command)
Byte 10:    <level_db>
```
Level byte uses **logarithmic dB conversion with noise floor**:
`level_db = truncate(20.0 * log10f((level / 100.0 + 0.000631) as f32))`
The `0.000631` offset (~10^-3.2) prevents log10(0) when level=0.

### Sidetone Mute (via SetValue 0xCA, not Extended 0xFF)
Non-silent mute sends 3 SetValue reports:
1. ValueId=4 (AudioIndicationsState), value=mute_state
2. ValueId=5 (SidetoneState), value=mute_state
3. ValueId=0x04/0x01 (extended), value=0
Silent mute sends only report 2.

### Remaining Output Report Byte Layouts

#### SetPowerState (Report ID 0xCC)
```
Byte 0:    PowerDownState enum (0=Invalid, 1=Reset, 2=Shutdown)
```
1-byte payload. Triggers a device reset or full power-down.

#### RequestData (Report ID 0xC9)
```
Byte 0:    ReportId enum value to poll (0x64=State, 0x65=DeviceMode, 0x66=FirmwareVersion)
```
1-byte payload. Requests the device send back the specified input report. Used
during initialisation to retrieve battery state, operating mode, and firmware
version.

#### StartPairing (Report ID 0xFF, type 12)
```
Byte 0:    0x02  (pairing sub-command)
Byte 1:    0x00
Byte 2:    0x40  (start pairing)
```
3-byte payload. Initiates wireless pairing with the dongle.

#### LinkStateNotify (Report ID 0xFF, type 13)
```
Bytes 0-4: 5 bytes (exact sub-command header TBD)
```
5-byte payload. Notifies the device of a link state change. Full header layout
not yet confirmed from disassembly.

#### SetAutoShutdown (Report ID 0xFF, type 9)
```
Bytes 0-7:  sub-command header (exact bytes TBD, currently zeroed placeholder)
Bytes 8-9:  timeout in minutes, little-endian u16 (0 = disabled)
```
10-byte payload. Sets the automatic power-off timer.

#### ApplySidetoneLevel (Report ID 0xFF, type 11)
No on-the-wire format — this is an internal-only report type used by iCUE to
trigger application of the sidetone level previously set via the extended
sidetone report (type 10). It does not generate an HID report.

### DirectLedControl (Report ID 0xCB)

Two report variants for controlling headset LEDs via TI LP5562 or CMA (Conexant)
register writes.

#### TI Variant (ReportType 6) — 19 bytes
```
Byte  0:      count of register/value pairs (0-8)
Bytes 1-16:   up to 8 pairs of (TI_register, value)
Bytes 17-18:  padding (0x00)
```

#### CMA Variant (ReportType 7) — 7 bytes
```
Byte  0:    count of register/value pairs (0-3)
Bytes 1-6:  up to 3 pairs of (CMA_register, value)
```
Used for the right logo zone on headsets with a Conexant audio codec path.

#### TI Register Zone Mapping

| Zone ID | Name       | Colors | PWM Registers        | Brightness Registers   |
|---------|------------|--------|----------------------|------------------------|
| 0x213   | LeftLogo   | RGB    | 0x1C, 0x16, 0x17    | 0x0C, 0x06, 0x07       |
| 0x214   | RightLogo  | RGB    | 0x1D, 0x18, 0x19    | 0x0D, 0x08, 0x09       |
| 0x215   | Status     | RG     | 0x1B, 0x1A           | 0x0B, 0x0A             |
| 0x216   | MicMute    | R      | 0x1E                 | 0x0E                   |

#### Key LED Operations
- **Start engines**: write `0x2A` to register `0x01` (enable all three TI engine sequencers)
- **Stop engines**: write `0x00` to register `0x01` (disable all sequencers)
- **Set brightness**: convert percent (0-100) to PWM byte `(level / 100.0 * 255.0)`, write to zone brightness registers
- **Set color**: write R/G/B values to the zone's PWM registers (channels unused by the zone are ignored)
- **Logarithmic PWM**: write `0x20` (bit 5) to all PWM registers for perceptually linear dimming, or `0x00` to disable
- **Clear PWM**: zero all PWM and brightness registers across every zone

---

## Bragi Protocol Details

Property-based protocol with direct read/write operations:
- `bragiprotocol::Properties` defines the property space
- `bragiprotocol::detail::PropertyConversion<T>::read/write` for byte encoding
- Conversion helpers:
  - `toBragiSidetoneLevel(int)` / `fromBragiSidetoneLevel(unsigned short)`
  - `toBragiMicrophoneLevel(int)` / `fromBragiMicrophoneLevel(unsigned short)`
  - `toDeviceBatteryStatus(BatteryStatus)`

### Bragi Notification IDs
| ID | Purpose |
|----|---------|
| 2 | Key state bitmap |
| 3 | Calibration |
| 4 | Lifetime change |
| 5 | Gesture detected |
| 6 | Gyroscope value |
| 7+ | Pairing progress |

### Bragi Hardware Profile Subsystems
- `bragiprotocol::hwp::lightings`
- `bragiprotocol::hwp::actuation`
- `bragiprotocol::hwp::coolers_settings`
- `bragiprotocol::hwp::actions::key_mapping_layout`

### Bragi Protocol — PropertyId Enum

152 properties extracted from `bragiprotocol::staticMetaObject` via QMetaObject
parsing. Full list in `notes/bragi_property_ids.md`. Headset-relevant subset:

| ID | Hex | Name | Type | Encoding |
|----|-----|------|------|----------|
| 2 | 0x02 | Brightness | ? | LED brightness |
| 3 | 0x03 | OperatingMode | enum | Hardware/Software |
| 13 | 0x0D | AutomaticSleepEnabled | bool | 0/1 |
| 14 | 0x0E | AutomaticSleepTimeoutNormal | ? | timeout value |
| 15 | 0x0F | BatteryLevel | u16 | percent * 10 (0-1000) |
| 16 | 0x10 | BatteryStatus | enum | values 1-3 |
| 19 | 0x13 | FirmwareVersion | ? | version struct |
| 58 | 0x3A | WirelessMode | enum | connection mode |
| 70 | 0x46 | SidetoneMuted | bool | 0/1 |
| 71 | 0x47 | SidetoneLevel | u16 | percent * 10 (0-1000) |
| 115 | 0x73 | WirelessConnectionConfiguration | set | supported modes |
| 142 | 0x8E | MicrophoneMuted | bool | 0/1 |
| 143 | 0x8F | MicrophoneLevel | u16 | percent * 10 (0-1000) |
| 166 | 0xA6 | MicrophoneMuteSwitchPosition | ? | hardware mute switch |

Other notable categories (see full list for all values):
- **DPI/Mouse** (IDs 24-52): per-stage resolution, sniper, X/Y split, stage colors
- **Keyboard** (IDs 65, 69, 74, 93-94, 155-162): layout, Win Lock, caps/scroll lock indicators, Fn swap
- **Lighting** (IDs 2, 63, 68, 73, 89-92, 119, 196-197): brightness, brightness limit, indicator colors, auto-brightness
- **Wireless** (IDs 23, 53-54, 58, 75, 95, 102-107, 115-116): RF power, encryption status, connection config
- **Power** (IDs 11-14, 55, 64): power saving, sleep timeouts, idle time
- **Gamepad** (IDs 122-133, 192-193): deadzone, sensitivity, vibration, analog reporting
- **Tilt** (IDs 183-191): gesture trigger angles, per-direction enable, multi-gesture

---

## Complete Command Set (cue::dev::cmd namespace)

### Query Commands
| Command | Purpose |
|---------|---------|
| `GetBatteryData` | Battery level/state |
| `GetFirmwareVersions` | FW versions |
| `GetDeviceSerial` | Serial number |
| `GetManufacturerName` | Manufacturer string |
| `GetAudioDeviceInfo` | Audio device info |
| `GetHidDeviceDescriptors` | HID descriptors |
| `GetMicBoomState` | Mic boom position |
| `GetMicMuteState` | Mic mute state |
| `GetEqualizerPreset` | EQ preset |
| `GetSidetoneMuteState` | Sidetone mute |
| `GetLinkState` | Wireless link state |
| `GetConnectionType` | Connection type (USB/wireless/BT) |
| `ActiveEqPresetIndex` | Active EQ preset index |

### Set Commands
| Command | Purpose |
|---------|---------|
| `SetMicMuted` | Mic mute on/off |
| `SetMicLevel` | Mic gain level |
| `SetMicBoostValue` | Mic boost |
| `SetSidetoneLevel` | Sidetone level |
| `SetSidetoneMuted` | Sidetone mute |
| `SetEqualizerPreset` | EQ preset data |
| `SetEqualizerEnabled` | EQ on/off |
| `SetActiveEqPresetIndex` | Active EQ index |
| `SetOperatingMode` | Normal/bootloader mode |
| `SetBatteryLightingEffect` | Battery LED effect |
| `SetOnDeviceLightingEffects` | On-device LED effects |
| `SetPredefinedLightingModeOn` | Predefined lighting |
| `SetKeySpecificBacklightEnabled` | Per-key backlight |
| `SetKeySpecificBrightnessLevel` | Per-key brightness |
| `SetIndicatorBrightness` | Indicator LED brightness |
| `SetSoundNotificationEnabled` | Sound notification toggle |
| `SetFrequenciesAnalysisEnabled` | FFT analysis toggle |
| `StartPairing` | Initiate pairing |
| `LinkStateNotify` | Link state change |

---

## Legacy Headset Feature Modules

| Feature Class | Purpose |
|--------------|---------|
| `LegacyHeadsetAudioControlsFeature` | Mic, sidetone, EQ, audio |
| `LegacyHeadsetBatteryFeature` | Battery level/state |
| `LegacyHeadsetDeviceInfoFeature` | FW version, serial, manufacturer |
| `LegacyHeadsetOperatingModeFeature` | Normal/bootloader switching |
| `LegacyHeadsetBacklightFeature` | LED backlight control |
| `LegacyHeadsetOnDeviceLightingFeature` | On-device lighting effects |
| `LegacyHeadsetKeyPressFeature` | Button press handling |
| `LegacyHeadsetResetFeature` | Device reset/shutdown |
| `LegacyHeadsetMicBoomFeature` | Mic boom up/down |
| `LegacyHeadsetPairingFeature` | Wireless pairing |
| `LegacyHeadsetWirelessSubdeviceFeature` | Wireless link state |
| `LegacyHeadsetRegularFwUpdateFeature` | Regular FW update |
| `LegacyHeadsetBootloaderFwUpdateFeature` | Bootloader FW update |
| `LegacyHeadsetCmaFwUpdateFeature` | CMA chip FW update |

### Sidetone Implementation
Two variants:
- `AudioComponentImpl` — basic audio controls (no HID sidetone)
- `AudioComponentWithHidSidetoneImpl` — HID-based sidetone control
- dB formula: `level_db = 20 * log10(level_percent / 100.0 + 1.0)`

---

## Audio Architecture

Three-tier system: iCUE plugin -> userspace daemon -> CoreAudio HAL plugin

### Tier 1: libAudioEnumerator.dylib (iCUE plugin)
- Discovers Corsair audio devices via IOKit `IOServiceMatching`
- Manages audio profiles (EQ, surround, SoundID)
- `BragiAudioDeviceComponent` subscribes to Bragi `PropertyId` changes
- `BragiSidetoneAudioDeviceComponent` — sidetone is HID-controlled, NOT audio path

### Tier 2: CorsairAudioConfigService (daemon)
- IPC server between iCUE and HAL driver
- Manages DSP audio graph (29 block types)
- Per-process profile switching (`GetProfileForPID`)

### Tier 3: CorsairAudio.driver (HAL plugin)
- Real-time DSP processing
- Entry: `_Plugin_Create` (standard CoreAudio HAL)
- Uses NineEars Surround Sound SDK (HRTF binaural) + Dolby PL2

### DSP Block Types
GainBlock, EQFilterBlock, HighPassFilterBlock, LowPassFilterBlock, BandPassFilterBlock,
NotchFilterBlock, PeakingEQFilterBlock, HighShelfFilterBlock, LowShelfFilterBlock,
LimiterBlock, CompressorBlock, StreamExpanderBlock, ChannelExtractorBlock, MixerBlock,
BypassBlock, SurroundBlock, DolbyBlock

### Audio Processing Chain
```
USB Audio In -> StreamExpander -> [NineEars Surround | Dolby PL2] -> EQ chain -> Gain -> Limiter -> Compressor -> Out
Mic In -> HPF -> Gain (boost) -> Compressor -> Mic Out
```

### CorsairAudio.kext (kernel extension)
- Matches IOUSBDevice with VID 0x1B1C + specific PIDs
- Injects `IOAudioEngineIsHidden = true` to hide native USB audio
- Allows HAL plugin to intercept audio stream for DSP

### Key insight: EQ/surround/SoundID are SOFTWARE DSP on the host.
### Sidetone and mic mute are HID-controlled directly on headset firmware.

---

## Device Identification

### Known Headset PIDs (VID 0x1B1C)

| Device | PID (hex) | PID (dec) | Protocol |
|--------|-----------|-----------|----------|
| HS80 | 0x0A69 | 2665 | Bragi |
| HS80 dongle | 0x0A6B | 2667 | Bragi |
| Void Elite Wireless | 0x0A51 | 2641 | Legacy |
| Void Elite Wireless (paired) | 0x0A50 | 2640 | Legacy |

### Headset Audio PIDs (kext matching, 58 total)
Range: 0x0A0C–0x0A97, plus 0x0D01, 0x1B23–0x1B2A

### Headset Capabilities from Manifests

| Feature | HS80 (Bragi) | Void Elite (Legacy) | Generic |
|---------|-------------|-------------------|---------|
| EQ | 10-band, 5 presets, unlimited custom | 10-band, 5 presets, max 5 | None |
| Sidetone | HID, 0-100 | HID, 0-100 | HID, 0-100 |
| Battery | 5-level thresholds | 2-level thresholds | N/A |
| Surround | DolbyAtmos (software) | Nahimic (software) | N/A |
| Lighting | BragiHeadsetRgb (6 HW effects) | OnDevice + Direct | Basic |
| LED Zones | Logo, Status, MicMute | LeftLogo, RightLogo, Status, MicMute | LeftLogo, RightLogo |
| Mic Boom | Yes | Yes | N/A |
| Wireless | Slipstream multipoint | Manual pairing | N/A |
| FW Update | BragiHeadsetFwUpdateTool | LegacyHeadsetFormat | Unsupported |

### EQ Presets (shared across devices)
1. Bass Boost
2. Clear Chat
3. FPS Competition
4. Movie Theater
5. Pure Direct

---

## Enumeration & Transport

### Device Discovery (macOS)
- `IOServiceAddMatchingNotification` on `IOHIDDevice` (wildcard VID/PID)
- Corsair VID filtering happens at plugin layer, not enumerator
- Report sizes from IOHIDDevice properties: `MaxInputReportSize`, `MaxOutputReportSize`, `MaxFeatureReportSize`

### Channel Architecture
- `Splitter` multiplexes physical HID device into logical channels by report type
- Each `HidChannel` carries: VID, PID, usage page, usage, direction, report size, serial
- Channel metadata keys: `usb-vid`, `usb-pid`, `hid-usagepage`, `hid-usage`, `hid-type`, `hid-direction`, `report`

### USB Reconfiguration
iCUE can reconfigure USB device configurations via `IOUSBInterfaceStruct190`, likely switching
devices into vendor-specific protocol mode.

### Dual USB Interface Model
Headsets enumerate as TWO USB devices:
1. **HID interface** — control/lighting/sidetone/battery (Bragi or Legacy protocol)
2. **USB Audio Class interface** — actual audio stream (captured by HAL plugin via kext)

---

## Firmware Update Architecture

Multi-process orchestration:
1. `CorsairHeadsetFirmwareUpdate` — CLI orchestrator (`--vid`, `--pid`, `--serial`, `--firmware`)
2. `CorsairHeadsetFirmwareUpdateHelper` — privileged helper (kills coreaudiod, uninstalls device)
3. Actual flashers: `CorsairAudioFWUpdRtx`, `BragiFwUpd`, `CorsairFWUpd`, etc.

Update flow:
1. Orchestrator launches helper with admin privileges (AppleScript `do shell script ... with administrator privileges`)
2. IPC via Qt Remote Objects over Unix socket
3. Helper stops coreaudiod, uninstalls device
4. Orchestrator spawns flasher, parses stdout for progress
5. RTX flasher: component-based (Aux+Main), CRC verification, direct HID I/O

### FW Update State Machines
- `CorsairHeadsetSimpleFwUpdateStateMachine` — single device
- `CorsairHeadsetPairedFwUpdateStateMachine` — paired tx+rx

### Chip-specific update processes
- `LegacyHeadsetAvneraAppToolFirmwareUpdateProcess` — Avnera
- `LegacyHeadsetCmaAppToolFirmwareUpdateProcess` — CMA
- `NxpAppToolFirmwareUpdateProcess` — NXP
- `CorsairBragiAppToolFirmwareUpdateProcess` — Bragi

---

## Config Data Format

All predefined data files use **Cereal C++ XML serialization**.

### Profile Format (.cueprofiledata)
Property types: `OSDProperty`, `TemperatureAlertProperty`, `AllDevicesCoolingProperty`,
`LightingLinkProperty`, `XDLProfileProperty`, `CoolingConfigurationProperty::Proxy`,
`HardwareActionsProperty::Proxy`, `AggregatedLightingsProperty::Proxy`,
`BasicLightingsProperty::Proxy`, `AdvancedLightingsProperty::Proxy`,
`HardwareMetaProperty::Proxy`

### Hardware Lighting Capabilities
`StandardRgb`, `StandardSingleColor`, `RestrictedWireless`, `ExtendedMouseRgb`,
`ZoneRgb`, `DramRgb`, `DramSingleColor`

### Protocol Names (from defaultHwLightings)
`Bragi_lighting_node`, `ClinkLightingNode`, `ClinkStarCoolers`, `ClinkPlatinumCoolers`,
`ClinkCorsairOne`, `Bragi_mouse`, `Bragi_mouse_rf`, `Bragi_kbd`,
`ClinkDram`, `Bragi_link_ecosystem_subdevice`

---

## IPC & External Services

### Internal IPC
- Qt Remote Objects over Unix sockets (FW update, helper processes)
- `CorsairLLAccessService` — low-level access service (separate daemon)
- `iCUEDevicePluginHost` — device plugin hosting

### Audio IPC
Custom packet protocol between CorsairAudioConfigService and HAL driver:
- `IPCAudioDevicePacket{DeviceOpen,Close,SetBlock,GetBlock,RemoveBlock,StreamControl,...}`
- `IPCAudioDeviceHookPacket{HookDevice,UnhookDevice,GetHookedDevices,CanHookDevice}`

### External
- mParticle analytics, Mixpanel (token: `577418fff0e87a5d473ad282f4eec87e`)
- Gigya SSO (SAP authentication)
- SoundID (Sonarworks) — license key embedded: `8cx1hxz4dcq3jwqetoipwosspkstyzeq`
- ipregistry for geolocation
