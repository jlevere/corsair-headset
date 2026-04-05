# Complete Bragi PropertyId Enum (152 values)

Extracted from `bragiprotocol::staticMetaObject` in `libiCUE.dylib` via QMetaObject parsing.

## Property IDs

| ID | Hex | Name | Category |
|----|-----|------|----------|
| 0 | 0x00 | Invalid | — |
| 1 | 0x01 | PollingRate | Input |
| 2 | 0x02 | Brightness | Lighting |
| 3 | 0x03 | OperatingMode | System |
| 4 | 0x04 | DebugMode | System |
| 6 | 0x06 | LiftHeight | Mouse |
| 7 | 0x07 | AngleSnappingEnabled | Mouse |
| 8 | 0x08 | AngleSnappingAngle | Mouse |
| 9 | 0x09 | ButtonPressDebounceHackEnabled | Input |
| 10 | 0x0A | ButtonPressDebounceHackTimeout | Input |
| 11 | 0x0B | PowerSavingModeEnabled | Power |
| 12 | 0x0C | PowerSavingModeFeatures | Power |
| 13 | 0x0D | AutomaticSleepEnabled | Power |
| 14 | 0x0E | AutomaticSleepTimeoutNormal | Power |
| 15 | 0x0F | BatteryLevel | Battery |
| 16 | 0x10 | BatteryStatus | Battery |
| 17 | 0x11 | UsbVid | Device Info |
| 18 | 0x12 | UsbPid | Device Info |
| 19 | 0x13 | FirmwareVersion | Device Info |
| 20 | 0x14 | BootloaderVersion | Device Info |
| 21 | 0x15 | WirelessChipFirmwareVersion | Device Info |
| 22 | 0x16 | WirelessChipBootloaderVersion | Device Info |
| 23 | 0x17 | RfPower | Wireless |
| 24 | 0x18 | StoredMouseSensorResolutionStage1 | DPI |
| 25 | 0x19 | StoredMouseSensorResolutionStage2 | DPI |
| 26 | 0x1A | StoredMouseSensorResolutionStage3 | DPI |
| 27 | 0x1B | StoredMouseSensorResolutionStage4 | DPI |
| 28 | 0x1C | StoredMouseSensorResolutionStage5 | DPI |
| 29 | 0x1D | StoredMouseSensorResolutionStageSniper | DPI |
| 30 | 0x1E | CurrentStoredMouseSensorResolutionStageIndex | DPI |
| 31 | 0x1F | EnabledStoredMouseSensorResolutionStages | DPI |
| 32 | 0x20 | MouseSensorResolution | DPI |
| 33 | 0x21 | MouseSensorResolutionX | DPI |
| 34 | 0x22 | MouseSensorResolutionY | DPI |
| 35–46 | 0x23–0x2E | StoredMouseSensorResolutionStage{1-5,Sniper}{X,Y} | DPI |
| 47–52 | 0x2F–0x34 | StoredMouseSensorResolutionStageColor{1-5,Sniper} | DPI |
| 53 | 0x35 | WirelessModeSwitchPosition | Wireless |
| 54 | 0x36 | ConnectedSubdevicesBitmap | Wireless |
| 55 | 0x37 | AutomaticSleepTimeoutPowersave | Power |
| 56 | 0x38 | ButtonReleaseDebounceHackTimeout | Input |
| 57 | 0x39 | ButtonReleaseDebounceHackEnabled | Input |
| 58 | 0x3A | WirelessMode | Wireless |
| 59 | 0x3B | SurfaceCalibrationPresetMode | Mouse |
| 60 | 0x3C | SurfaceCalibrationPresetIndex | Mouse |
| 61 | 0x3D | TotalStorageSize | System |
| 62 | 0x3E | FreeStorageSize | System |
| 63 | 0x3F | MaximumBrightnessLimitEnabled | Lighting |
| 64 | 0x40 | IdleTime | Power |
| 65 | 0x41 | PhysicalKeyboardLayout | Keyboard |
| 68 | 0x44 | BrightnessLevelIndex | Lighting |
| 69 | 0x45 | WinLockEnabled | Keyboard |
| **70** | **0x46** | **SidetoneMuted** | **Audio** |
| **71** | **0x47** | **SidetoneLevel** | **Audio** |
| 73 | 0x49 | ProfileIndicatorColor | Lighting |
| 74 | 0x4A | WinLockDisabledShortcuts | Keyboard |
| 75 | 0x4B | WirelessEncryptionStatusSubdevice1 | Wireless |
| 79–81 | 0x4F–0x51 | HardwareProfile{1-3}ActiveMouseSensorResolutionStageIndex | DPI |
| 82 | 0x52 | AmbidextrousMode | Mouse |
| 85 | 0x55 | CoolerComponentProductLineId | Cooler |
| 86 | 0x56 | CoolerComponentProductVendorId | Cooler |
| 87 | 0x57 | CoolerComponentPumpVersionId | Cooler |
| 88 | 0x58 | CoolerComponentRadiatorSize | Cooler |
| 89 | 0x59 | LedModuleProductLineId | Lighting |
| 90 | 0x5A | LedModuleVendorId | Lighting |
| 91 | 0x5B | LedModuleVersionId | Lighting |
| 92 | 0x5C | BrightnessIndicatorColor | Lighting |
| 93 | 0x5D | WinLockEnabledIndicatorColor | Keyboard |
| 94 | 0x5E | WinLockDisabledIndicatorColor | Keyboard |
| 95 | 0x5F | WirelessFirmwareVariant | Device Info |
| 96 | 0x60 | DialRingRotation | Input |
| 98 | 0x62 | ProfileIndicationBehavior | Lighting |
| 99 | 0x63 | WinLockIndicationBehavior | Keyboard |
| 100 | 0x64 | DialModeIndicationBehavior | Input |
| 101 | 0x65 | ExceptionalState | System |
| 102–107 | 0x66–0x6B | WirelessEncryptionStatusSubdevice{2-7} | Wireless |
| 108 | 0x6C | ActiveDialModeIndex | Input |
| 109 | 0x6D | DialZoneBrightness | Lighting |
| 110 | 0x6E | ActiveHardwareProfileIndex | System |
| 115 | 0x73 | WirelessConnectionConfiguration | Wireless |
| 116 | 0x74 | WirelessConnectionConfigurationSupport | Wireless |
| 117 | 0x75 | PrimaryFirmwareStatus | Device Info |
| 118 | 0x76 | EsportsModeSwitchPosition | Input |
| 119 | 0x77 | EsportsModeBacklightColor | Lighting |
| 120 | 0x78 | HostOperatingSystem | System |
| 121 | 0x79 | HwDpiAdjustmentEnabled | Mouse |
| 122–125 | 0x7A–0x7D | {Left,Right}{Trigger,Thumbstick}Deadzone | Gamepad |
| 126–129 | 0x7E–0x81 | {Left,Right}{Trigger,Thumbstick}Sensitivity | Gamepad |
| 130–131 | 0x82–0x83 | {Left,Right}VibrationMotorPresent | Gamepad |
| 132–133 | 0x84–0x85 | {Left,Right}VibrationMotorIntensity | Gamepad |
| **142** | **0x8E** | **MicrophoneMuted** | **Audio** |
| **143** | **0x8F** | **MicrophoneLevel** | **Audio** |
| 150 | 0x96 | MaximumPollingRate | Input |
| 155 | 0x9B | CapsLockIndicationBehavior | Keyboard |
| 156 | 0x9C | ScrollLockIndicationBehavior | Keyboard |
| 158 | 0x9E | CapsLockEnabledIndicatorColor | Keyboard |
| 159 | 0x9F | ScrollLockEnabledIndicatorColor | Keyboard |
| 161 | 0xA1 | MacroRecordingIndicatorColor | Keyboard |
| 162 | 0xA2 | HwMenuFn2SwapFunctionEnabled | Keyboard |
| 164–165 | 0xA4–0xA5 | HardwareProfile{4,5}ActiveMouseSensorResolutionStageIndex | DPI |
| 166 | 0xA6 | MicrophoneMuteSwitchPosition | Audio |
| 168 | 0xA8 | GyroscopeReportingMode | Input |
| 176 | 0xB0 | ButtonDebounceAlgorithm | Input |
| 177 | 0xB1 | BrightnessIndicationBehavior | Lighting |
| 180 | 0xB4 | HardwareActionFeatures | System |
| 183–186 | 0xB7–0xBA | Tilt{Forward,Backward,Left,Right}GestureTriggerAngle | Tilt |
| 187–190 | 0xBB–0xBE | Tilt{Forward,Backward,Left,Right}GestureEnabled | Tilt |
| 191 | 0xBF | MultipleGesturesPerMovementEnabled | Tilt |
| 192 | 0xC0 | TriggerPositionsReportingEnabled | Gamepad |
| 193 | 0xC1 | ThumbstickPositionsReportingEnabled | Gamepad |
| 196 | 0xC4 | AutomaticBrightnessAdjustmentEnabled | Lighting |
| 197 | 0xC5 | AutomaticBrightnessOverrideActive | Lighting |
| 198 | 0xC6 | AnalogWASDReportingMode | Keyboard |
| 199 | 0xC7 | AnalogArrowsReportingMode | Keyboard |
| 206 | 0xCE | SubdevicesDetectionWarnings | System |

## Headset-relevant properties (bolded above)

| ID | Name | Type | Encoding |
|----|------|------|----------|
| 0x0F | BatteryLevel | u16 | percent * 10 (0–1000) |
| 0x10 | BatteryStatus | enum | 1=?, 2=?, 3=? (values 1–3) |
| 0x46 | SidetoneMuted | bool | 0/1 |
| 0x47 | SidetoneLevel | u16 | percent * 10 (0–1000) |
| 0x8E | MicrophoneMuted | bool | 0/1 |
| 0x8F | MicrophoneLevel | u16 | percent * 10 (0–1000) |
| 0xA6 | MicrophoneMuteSwitchPosition | ? | hardware mute switch |
| 0x02 | Brightness | ? | LED brightness |
| 0x03 | OperatingMode | enum | Hardware/Software |
| 0x0D | AutomaticSleepEnabled | bool | |
| 0x0E | AutomaticSleepTimeoutNormal | ? | timeout value |
| 0x13 | FirmwareVersion | ? | version struct |
| 0x3A | WirelessMode | enum | connection mode |
| 0x73 | WirelessConnectionConfiguration | set | supported modes |
