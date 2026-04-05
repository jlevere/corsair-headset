use crate::types::{DeviceId, ProductId};

/// Which protocol family a device uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProtocolFamily {
    /// Older headsets: VOID, HS60, HS70, etc.
    Legacy,
    /// Newer headsets: HS80, HS55 Wireless Core, etc.
    Bragi,
}

/// Static device catalog entry.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// The USB device ID (dongle/receiver PID for wireless devices).
    pub id: DeviceId,
    /// The paired/wireless headset PID (if different from dongle).
    pub paired_pid: Option<ProductId>,
    /// Human-readable device name.
    pub name: &'static str,
    /// Protocol family.
    pub protocol: ProtocolFamily,
}

// ---------------------------------------------------------------------------
// VOID RGB Elite Wireless
// ---------------------------------------------------------------------------

pub const VOID_ELITE_WIRELESS_CARBON: DeviceInfo = DeviceInfo {
    id: DeviceId::corsair(0x0A51),
    paired_pid: Some(ProductId(0x0A50)),
    name: "VOID RGB Elite Wireless (Carbon)",
    protocol: ProtocolFamily::Legacy,
};

pub const VOID_ELITE_WIRELESS_WHITE: DeviceInfo = DeviceInfo {
    id: DeviceId::corsair(0x0A55),
    paired_pid: Some(ProductId(0x0A54)),
    name: "VOID RGB Elite Wireless (White)",
    protocol: ProtocolFamily::Legacy,
};

pub const VOID_ELITE_WIRELESS_GREY: DeviceInfo = DeviceInfo {
    id: DeviceId::corsair(0x0A75),
    paired_pid: Some(ProductId(0x0A74)),
    name: "VOID RGB Elite Wireless (Grey)",
    protocol: ProtocolFamily::Legacy,
};

// ---------------------------------------------------------------------------
// VOID Pro Wireless
// ---------------------------------------------------------------------------

pub const VOID_PRO_WIRELESS_CARBON: DeviceInfo = DeviceInfo {
    id: DeviceId::corsair(0x0A2B),
    paired_pid: Some(ProductId(0x0A2A)),
    name: "VOID Pro Wireless (Carbon)",
    protocol: ProtocolFamily::Legacy,
};

// ---------------------------------------------------------------------------
// HS80 (Bragi protocol)
// ---------------------------------------------------------------------------

pub const HS80_WIRELESS: DeviceInfo = DeviceInfo {
    id: DeviceId::corsair(0x0A69),
    paired_pid: Some(ProductId(0x0A6B)),
    name: "HS80 RGB Wireless",
    protocol: ProtocolFamily::Bragi,
};

// ---------------------------------------------------------------------------
// Catalog lookup
// ---------------------------------------------------------------------------

/// All known devices.
pub const CATALOG: &[DeviceInfo] = &[
    VOID_ELITE_WIRELESS_CARBON,
    VOID_ELITE_WIRELESS_WHITE,
    VOID_ELITE_WIRELESS_GREY,
    VOID_PRO_WIRELESS_CARBON,
    HS80_WIRELESS,
];

/// Look up a device by its dongle/receiver PID.
#[must_use]
pub fn lookup_by_pid(pid: ProductId) -> Option<&'static DeviceInfo> {
    CATALOG.iter().find(|d| d.id.pid == pid)
}
