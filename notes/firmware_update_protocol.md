# Corsair Headset Firmware Update Protocol

Reverse-engineered from three firmware updater binaries shipped with iCUE for macOS.

---

## 1. Binary Overview

| Binary | Size | Architecture | Purpose |
|--------|------|-------------|---------|
| `CorsairAudioFWUpdRtx` | ~126 KB | x86_64 Mach-O, C++, not stripped | RTX (DECT) headset flasher. Uses RTX/Phoenix API messaging system over HID. |
| `BragiFwUpd` | ~1.4 MB | x86_64 Mach-O, C++/Qt, not stripped | Bragi protocol headset flasher. Larger binary with full `bragiprotocol` library. |
| `CorsairFWUpd` | ~270 KB | x86_64 Mach-O, C++, not stripped | Generic NXP-based headset flasher. Uses `cue::dev::legacy::NxpMessage` protocol. |

All three use `IOHIDDevice{SetReport,GetReport}` via the IOKit HID API for communication.
Default USB VID across all: `0x1B1C` (Corsair).

---

## 2. CorsairAudioFWUpdRtx -- RTX Protocol (DECT Headsets)

### 2.1 Command-Line Interface

Uses `QCommandLineParser` with these options:

| Option | Long Name | Description | Default |
|--------|-----------|-------------|---------|
| `--vid` | `device_vendor_id` | Device VID | `0x1B1C` |
| `--pid` | `device_product_id` | Device PID | (required) |
| `--serial` | `device_serial` | Serial number filter | (optional) |
| `--firmware` | `filepath` | Firmware file path (`*.fwu`) | (required) |
| `--useScufDisableUsbTraffic` | | Send Scuf USB disable/enable commands | `false` |

### 2.2 Architecture: ROS Mail System

The RTX updater uses a microkernel-style message-passing system ("ROS" -- RTX Operating System emulation on the host). All protocol actions are encoded as **mail messages** allocated via `RosMailAllocate(taskId, size)` and sent via `RosMailDeliver()`.

Components:
- **GfFwu** -- firmware update state machine
- **GfMacHidDriver** -- macOS HID transport layer
- **GfTrace** -- logging (outputs HTML trace files)
- **GfComponent** -- component lifecycle (start/stop sequences)
- **SclTask**, **ApiTask** -- task scheduling

### 2.3 API Message IDs (FWU Protocol)

Every message starts with a 16-bit **primitive ID** at offset 0. Extracted from disassembly:

| Primitive ID | Name | Direction | Payload Size | Description |
|-------------|------|-----------|-------------|-------------|
| `0x4F00` | `API_FWU_ENABLE_REQ` | Host -> Device | 4 bytes | Enable firmware update mode. Byte[2]=deviceId, Byte[3]=flags |
| `0x4F01` | `API_FWU_ENABLE_CFM` | Device -> Host | 3 bytes | Confirm FWU mode enabled. Byte[2]=status |
| `0x4F02` | `API_FWU_DEVICE_NOTIFY_IND` | Device -> Host | 45 bytes | Device info notification. Contains device type, FW version, link date |
| `0x4F03` | `API_FWU_UPDATE_REQ` | Host -> Device | 5 bytes | Request to start update. Word[2]=terminalId, Byte[4]=flags |
| `0x4F04` | `API_FWU_UPDATE_CFM` | Device -> Host | 5 bytes | Confirm update request. Word[2]=terminalId, Byte[4]=status |
| `0x4F05` | `API_FWU_UPDATE_IND` | Host -> Device | 26 bytes | Update indication with file metadata. Contains DeviceId, link date, file offset, start address, size |
| `0x4F06` | `API_FWU_UPDATE_RES` | Device -> Host | 17+ bytes | Update response. Contains version info + variable area data (n * 8 bytes) |
| `0x4F07` | `API_FWU_GET_BLOCK_IND` | Host -> Device | 16 bytes | Request data block. Word[2]=addr, DWord[4]=offset, DWord[8]=length, DWord[0xC]=blockSize |
| `0x4F08` | `API_FWU_GET_BLOCK_RES` | Device -> Host | 16+ bytes | Block data response. Header + payload data copied at offset 0x10 |
| `0x4F09` | `API_FWU_GET_CRC_IND` | Host -> Device | 16 bytes | Request CRC check. Word[2]=addr, DWord[4]=offset, DWord[8]=length, DWord[0xC]=expected |
| `0x4F0A` | `API_FWU_GET_CRC_RES` | Device -> Host | 18 bytes | CRC result. DWord[4]=offset, DWord[8]=length, DWord[0xC]=crc, Word[0x10]=status |
| `0x4F0B` | `API_FWU_COMPLETE_IND` | Host -> Device | 6 bytes | Update complete notification. DWord[2]=status |
| `0x4F0C` | `API_FWU_STATUS_IND` | Both | 5+ bytes | Status indication. Word[2]=terminalId, Byte[4]=dataLen, then data |
| `0x4F0D` | `API_FWU_MULTI_CRC_IND` | Host -> Device | 13+ bytes | Multi-block CRC check. DWord[4]=offset, DWord[8]=length, Byte[0xC]=count, then CRC pairs |
| `0x4F14` | `API_FWU_CRC32_IND` | Host -> Device | 13+ bytes | CRC32 variant of multi-CRC. Same layout as MULTI_CRC_IND |
| `0x4F16` | `API_FWU_PROGRESS_IND` | Both | 13 bytes | Progress report. Word[2]=terminalId, Byte[4]=phase, DWord[5]=current, DWord[9]=total |
| `0x4F17` | `API_FWU_DISABLE_USB_REQ` | Host -> Device | 2 bytes | Disable USB traffic (Scuf mode) |
| `0x4F18` | `API_FWU_ENABLE_USB_REQ` | Host -> Device | 2 bytes | Re-enable USB traffic |

### 2.4 FWU File Format (`.fwu`)

The `.fwu` file format is parsed by `GetFwuHeader` and `ScanFwuBlocks`:

**File Header** (64 bytes / 0x40, read at offset 0):
- Offset 0x00: Magic bytes `"FWU"` (0x46, 0x57, 0x55)
- Remaining 61 bytes: version info, block count, device IDs, link dates
- The header is validated with CRC-16/CCITT (using `Crc16Ccitt` and table at `crc16tbl`)
- Header is copied to internal struct at offset +0x28 (64 bytes from +0x68 raw buffer)

**Block Table** (scanned after header):
- Each block descriptor is 8 bytes, read at sequential offsets starting at 0x40
- Block descriptor fields: `DWord[0]` = DeviceId/flags, `DWord[4]` = size/offset
- Block data immediately follows each descriptor
- Maximum read chunk size per block: **512 bytes** (0x200)
- Blocks are stored in linked lists (lo list at +0xA8, hi list at +0xB8)

**Per-block logging format** (from format strings):
```
FWU:   lo: DeviceId=%X %s LD=20%02X%02X%02X %02X:%02X Offset=%X Start=%X Size=%X
FWU:   hi: DeviceId=%X %s LD=20%02X%02X%02X %02X:%02X Offset=%X Start=%X Size=%X
```
This reveals each block has: DeviceId, a name string, a link date (year/month/day hour:minute), file offset, start address, and size.

### 2.5 Component Model: Aux vs Main

The firmware update distinguishes between two component types:

- **Aux** firmware (auxiliary processor)
- **Main** firmware (main processor)

Evidence from strings:
```
FWU: EvtStatus %X Aux %X=%s
FWU: EvtStatus %X Main %X=%s
FWU: EvtProgress %X Aux %X=%s %u%%
FWU: EvtProgress %X Main %X=%s %u%%
```

### 2.6 CRC/Checksum Support

Three CRC algorithms are available:

| Function | Algorithm | Usage |
|----------|-----------|-------|
| `Crc16Ccitt` | CRC-16/CCITT | FWU file header validation |
| `Crc16Legacy` | CRC-16 (legacy variant) | Older device compatibility |
| `Crc32x` | CRC-32 (custom table at `Crc32xTable`) | Block-level verification (`API_FWU_CRC32_IND`) |

### 2.7 State Machine

The progress phases (from strings and `evtProgressHandler`/`evtStatusHandler`):

1. **Idle** -- waiting for command
2. **Checking** -- validating firmware file, comparing versions
3. **Writing** -- transferring block data
4. **Verifying** -- CRC verification of written blocks

Status values (from `evtStatusHandler` strings):
- "Up to date"
- "No files found"
- "No files matching"
- "Update pending"
- "Update in progress"
- "No device"
- "No response"
- "No capability"
- "Blocked"
- "Configuration error"

### 2.8 Progress Reporting

Progress is reported to stdout:
```
Progress: <percentage>
```
The parent process (iCUE) parses this stdout output to drive its progress bar.

### 2.9 Device Identification

Devices are matched by HID properties:
- `VendorID`, `ProductID`, `VersionNumber`, `LocationID`
- HID path format: `\\?\hid#vid_%04x&pid_%04x&rev_%04x&locid_%08x`
- Report sizes: `MaxOutputReportSize`, `MaxInputReportSize`
- HID report uses `IOHIDDeviceSetReport` for TX, `IOHIDDeviceRegisterInputReportCallback` for RX

### 2.10 Known Device Codenames

From the device name table in the binary:
- Udinese, Capri, RTX2300, Raffle HS (4MB/8MB flash), Edon base, RTX 2011
- Dixie FP/PP, Natalie variants, 4088 FP/PP, DECT500/Phantom PTI, RTX4024
- Amelie, Aiko, Dana2, Reef, SmartBeat, Nina, Senna (1.9/2.4GHz), Dwight, SophiaUSB
- FP = Fixed Part (base station), PP = Portable Part (headset)

### 2.11 Scuf USB Traffic Control

For Scuf-branded devices, the updater can send `API_FWU_DISABLE_USB_REQ` (0x4F17) before update and `API_FWU_ENABLE_USB_REQ` (0x4F18) after completion, controlled by `--useScufDisableUsbTraffic true`.

---

## 3. BragiFwUpd -- Bragi Protocol (Modern Headsets)

### 3.1 Architecture

Much larger binary (~1.4 MB) built on Qt with a full `bragiprotocol` library. Uses a command-pattern architecture:

**Key Classes:**
- `bragi_fw_upd::BragiFwUpdateFacade` -- main controller/state machine
- `bragi_fw_upd::BragiPhysicalDevice` -- device I/O abstraction
- `bragi_fw_upd::WriteFirmwareFiles` -- writes firmware data to device
- `bragi_fw_upd::ApplyFirmware` -- triggers firmware apply on device
- `bragi_fw_upd::SetOperatingMode` / `GetOperatingMode` -- mode switching
- `bragi_fw_upd::DeviceStreamBuffer` -- buffered write abstraction
- `bragiprotocol::Serializer` -- command/reply serialization

### 3.2 Command-Line Interface

```
--usb-vid     USB VID (default: 0x1b1c)
--usb-pid     USB PID
--hid-up      HID Usage Page (default: 0xff42)
--hid-usage   HID Usage
--subdevice-id   Target subdevice ID (for wireless update)
--firmware-image         Firmware image file
--bootloader-image       Bootloader image file
--combined-image         Combined (firmware + bootloader) image file
--secondary-chip-image   Secondary chip firmware image file
--deviceSerial           Device serial number
--validate-checksum      Validate checksum per firmware file
--one-hunk               Write firmware in one hunk
--delays                 Short,medium,large delays (comma-separated)
--update-method          Firmware update method
--hasReplyToApply        Device replies to ApplyFirmware
--needRestartAfterApply  Device restarts after ApplyFirmware
--needDriverReinstall    Reinstall USB composite device after reconnect
--supportedUpdateUsbIds  VID,PID pairs for post-update detection
--requiredOperatingMode  Target mode: Bootloader|HostControlled|SelfOperated|McuBootloader
--applyFirmwareTimeout   Timeout in ms for apply (default: 8000)
--interactive            Interactive mode
--socket-name            Socket for cancel support
```

### 3.3 HID Usage Page

Bragi devices use a **vendor-specific HID usage page**: `0xFF42` (compared to standard HID pages). This is the custom Corsair Bragi protocol endpoint.

### 3.4 Firmware Image Types

Four distinct image types supported:

| Type | CLI Flag | Description |
|------|----------|-------------|
| Firmware | `--firmware-image` | Main application firmware |
| Bootloader | `--bootloader-image` | Bootloader firmware |
| Combined | `--combined-image` | Firmware + bootloader in one file |
| Secondary Chip | `--secondary-chip-image` | Secondary processor firmware |

Constraints:
- Combined image cannot be used together with separate firmware/bootloader images
- "Strict" update method supports only firmware image type

### 3.5 Operating Modes

The Bragi protocol has distinct operating modes the device can be in:

| Mode | Description |
|------|-------------|
| `Bootloader` | Device is in bootloader, ready for flashing |
| `HostControlled` | Device is controlled by host software |
| `SelfOperated` | Device is operating independently |
| `McuBootloader` | MCU-level bootloader mode |

The update flow requires switching to the correct mode first via `--requiredOperatingMode`.

### 3.6 Update State Machine

States from `BragiFwUpdateFacade`:

```
1. Initial          -> Device detection, wait for device
2. Open Device      -> "Push 'Open device' command"
3. Set Delays       -> "Push 'Set delays' command"
4. Check Mode       -> "Check operating mode"
5. Set Mode         -> "Set preferred mode" (may trigger reconnect)
6. Write FW         -> "Push 'Write FW files' command" / "Write FW started"
7. Apply FW         -> "Push 'Apply FW' command" / "Apply FW started"
8. Finalize         -> "Finalizing update" (restore self-operating mode)
9. Device Reinstall -> "Starting device reinstall" (Windows driver reinstall)
```

On mode change, the device disconnects and reconnects. The updater handles reconnection with configurable timeouts.

### 3.7 Data Transfer: Hunks and Chunks

The Bragi protocol uses a **stream buffer** abstraction:

- **writeHunk()** -- writes a "hunk" of data (one logical write operation)
  - `writeHunkImpl(bufferIndex, data, size, flag, reportOptions)`
- **readChunk()** -- reads back data for verification
  - `readChunk(bufferIndex, buffer, size, reportOptions)`
- **DeviceStreamBuffer** -- manages buffer lifecycle with `BufferLock`
  - Constructor: `(BufferLock, device, WriteMode, param1, param2, reportOptions, chunkSize)`

Key protocol messages reference:
```
WriteBufferBegin  (output report, initiates write)
WriteBuffer       (output report, continuation data)
ReadBuffer        (input report, read back)
```

Error messages confirm report size constraints:
- "Output report size too small for WriteBuffer"
- "Output report size too small for WriteBufferBegin"
- "Input report size too small for ReadBuffer"

### 3.8 Bragi Report Structure

From `bragiprotocol::serialization`, reports have headers with variants:

**ShortHeaderLayout:**
- SubDeviceId, NeedReply flag, ErrorCode

**ExtendedHeaderLayout:**
- SubDeviceId, NeedReply flag, ErrorCode (extended fields)

Reports are typed (from `bragiprotocol::Report`):
- Command reports (CommandId)
- Reply reports (CommandId)
- Notification reports (NotificationId)

Known CommandIds used in firmware update: 3, 5, 8, 15, 16, 22.

### 3.9 Checksum Validation

The Bragi updater supports checksum validation:
- CRC32-MPEG2 algorithm (class `CRC32_MPEG2Calculator`)
- Abstract base: `AbstractChecksumCalculator`
- Validated per firmware file when `--validate-checksum` is set
- Error: "Checksum didn't match. Invalid firmware file."

### 3.10 Progress Reporting

Same stdout format as RTX:
```
Progress: <percentage>
Finished: success
Finished: fail
```

### 3.11 Subdevice Addressing

Bragi supports **subdevice IDs** for wireless devices. The `--subdevice-id` parameter targets a specific subdevice (e.g., updating the headset through a wireless dongle). Reports carry a `subDeviceId` field in their headers.

### 3.12 Protocol Error Codes

From `bragiprotocol::ProtocolError`:
- No error
- Device is busy
- Malformed report
- (Application-specific codes via `errorDescriptionByCode`)

---

## 4. CorsairFWUpd -- NXP Protocol (Legacy/Generic Headsets)

### 4.1 Architecture

Uses `fwupd::NxpAppUpdateProcessMac` -- a process-based update mechanism targeting NXP microcontrollers.

**Key Classes:**
- `fwupd::DeviceUpdater` -- device detection and lifecycle
- `fwupd::AbstractUpdateProcess` -- base update process
- `fwupd::NxpAppUpdateProcessMac` -- macOS-specific NXP update
- `cue::dev::legacy::NxpMessage` -- NXP HID message format
- `cue::dev::legacy::NxpCookie` -- NXP session cookie
- `cue::dev::legacy::fw_upd_util` -- firmware probe/validation

### 4.2 Command-Line Interface

```
--path=<firmware_file>
--usb-vid=<vid>
--usb-pid=<pid>
--hid-up=<usage_page>
--test-storage=<bool>
--privileged=<bool>
```

The `--privileged` flag triggers `setuid(0)` for raw USB access.

### 4.3 NXP Message Format

From `NxpMessage` constructor: messages are **64 bytes** (0x40), matching a standard HID report:

```
Offset  Size  Field
0x00    1     ReportId
0x01    1     CommandId
0x02    62    Payload (zeroed on init)
```

The constructor `NxpMessage(ReportId, CommandId)` stores ReportId at [0], CommandId at [1], and zeros bytes [2..63].

### 4.4 NXP Report IDs and Commands

From disassembly of `run()`:

| Report Word | Meaning | Description |
|-------------|---------|-------------|
| `0x0C07` | Check Storage | ReportId=0x0C, CommandId=0x07. Checks if device storage is ready |
| `0x0D07` | Write Block | ReportId=0x0D, CommandId=0x07. Writes a data block to device |

The `run()` method constructs messages:
1. **Storage check**: `NxpMessage(0x0C, 0x07)` with protocol version byte at [2], flag=1 at [3]
2. **Data write**: `NxpMessage(0x0D, 0x07)` with protocol version at [2], flag=0 at [3], block size at [4..5], sequence at [6..7]

### 4.5 Protocol Versions

Two protocol versions detected:
- Protocol `0xF0` (240) -- standard/legacy protocol
- Protocol `0x06` (6) -- newer protocol (selected when `probeFirmware` returns 3)

When protocol version is 6, additional fields are populated in write messages (block length word at offset [4]).

### 4.6 Data Transfer

The NXP transfer loop in `run()`:

1. **Map firmware file** into memory via `fwupd::mapFile()`
2. **Probe firmware** via `fw_upd_util::probeFirmware()` to determine protocol version
3. **Check storage** by sending `0x0C07` messages in a retry loop (up to 5 retries with 1-second sleeps)
4. **Write blocks** in 256-byte chunks:
   - Maximum chunk size: **256 bytes** (0x100)
   - For each chunk: construct `0x0D07` message, call `writeDeviceBuffer()`
   - Read back via `readDeviceBuffer()` for verification
   - Sleep between blocks: ~750ms (0x2CB41780 ns) for standard, ~950ms (0x389FD980 ns) for protocol 6
5. **Report progress** incrementally (0%, 3%, 5%, then proportional during write)

### 4.7 Device Communication

Uses `IOHIDDeviceSetReport` for writes and `IOHIDDeviceGetReport` for reads. The `deviceHandle()` method maps `ReportType` enums to HID device handles, supporting multiple report types per device.

HID properties used for matching: `VendorID`, `ProductID`, `PrimaryUsage`, `PrimaryUsagePage`, `MaxInputReportSize`, `MaxOutputReportSize`, `MaxFeatureReportSize`.

### 4.8 Progress Reporting

```
Progress: <percentage>
Finished: success
Finished: fail
```

---

## 5. Protocol Comparison

| Feature | RTX (`CorsairAudioFWUpdRtx`) | Bragi (`BragiFwUpd`) | NXP (`CorsairFWUpd`) |
|---------|------------------------------|----------------------|----------------------|
| Target Devices | DECT headsets (base+headset) | Modern Corsair headsets | Legacy NXP-based headsets |
| HID Usage Page | Standard vendor | `0xFF42` (custom) | Standard vendor |
| Message Format | 2-byte primitive ID + payload | Serialized commands with headers | 64-byte fixed report (ReportId + CommandId) |
| Chunk Size | 512 bytes max per block read | Variable (hunk/chunk model) | 256 bytes max |
| CRC | CRC-16/CCITT, CRC-32 | CRC32-MPEG2 | N/A (read-back verification) |
| File Format | `.fwu` (custom, 64-byte header, "FWU" magic) | Raw image files | Raw image files (mmap'd) |
| Components | Aux + Main firmware | Firmware, Bootloader, Combined, Secondary Chip | Single image |
| Mode Switch | N/A (DECT protocol handles it) | Bootloader/HostControlled/SelfOperated/McuBootloader | N/A |
| Subdevice Support | Terminal IDs (16-bit) | SubdeviceId in report headers | Single device |
| Transport | IOKit HID (SetReport) | IOKit HID (SetReport/GetReport) | IOKit HID (SetReport/GetReport) |
| Protocol Versioning | Fixed API (0x4F00-0x4F18) | ProtocolVariant (Short/Extended headers) | Protocol 0xF0 vs 0x06 |
| Post-Update | Enable USB, reboot | Apply firmware command, optional restart | Implicit |

---

## 6. FWU File Format Detail

### Header (64 bytes)

```
Offset  Size  Description
0x00    3     Magic: "FWU" (0x46 0x57 0x55)
0x03    1     Version/flags
0x04    ...   Device IDs, link dates, block count
0x1C    4     Block count (DWord, 0 = no blocks)
...
0x3E    2     CRC-16/CCITT over header (init 0xFFFF)
```

### Block Descriptors (8 bytes each, starting at offset 0x40)

```
Offset  Size  Description
0x00    4     DeviceId / type flags
0x04    4     Block data size
```

### Block Data

Immediately follows each block descriptor. Each block contains:
- Device ID identifying which processor this block targets
- A name/identifier (extracted from filename: `_aXXXX` or `_AXXXX` patterns parsed by `AllocateFwuFile`)
- Link date in BCD format (year, month, day, hour, minute)
- Offset within flash
- Start address
- Size in bytes

### Filename Convention

FWU files follow a naming convention parsed by `AllocateFwuFile`:
- Underscore-separated fields
- Fields starting with `a` or `A` followed by 3 digits encode version/ID info
- The version string is stored at offset +0x11 in the internal structure (4 bytes + null)

---

## 7. Security Observations

1. **No authentication**: None of the three protocols include cryptographic signature verification of firmware images. CRC checksums provide integrity but not authenticity.

2. **No encryption**: Firmware data is transferred in plaintext over HID reports.

3. **Privileged access**: `CorsairFWUpd` supports a `--privileged` flag that calls `setuid(0)`, indicating it may run with elevated privileges.

4. **Debug features**: The RTX updater contains debug flags:
   - "Forcing update because debug repeat is enabled"
   - "Returning bad CRC because debug inject errors is enabled"
   - "Returning bad CRC because debug erase all is enabled"
   These suggest debug/testing modes that could be triggered if the right memory locations are set.

5. **File format simplicity**: The `.fwu` format uses only CRC-16 for validation, with no signing. A modified firmware file with a corrected CRC would be accepted.

---

## 8. iCUE Integration

All three updaters are invoked as child processes by the main iCUE application. Communication is via:

1. **Stdout parsing**: Parent reads `Progress: <N>` and `Finished: success/fail` lines
2. **Exit code**: Process exit status indicates success/failure
3. **Socket** (Bragi only): `--socket-name` enables a local socket for cancel commands during interactive updates

The typical invocation flow:
```
iCUE -> detect device VID/PID -> select appropriate updater binary
     -> spawn process with --vid, --pid, --firmware args
     -> parse stdout for progress
     -> handle completion/failure
```
