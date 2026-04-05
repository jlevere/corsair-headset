/// Corsair USB Vendor ID (current).
pub const CORSAIR_VID: VendorId = VendorId(0x1B1C);

/// Corsair USB Vendor ID (legacy, used for bootloaders).
pub const CORSAIR_VID_LEGACY: VendorId = VendorId(0x170D);

/// USB Vendor ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VendorId(pub u16);

/// USB Product ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProductId(pub u16);

/// Combined VID + PID device identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeviceId {
    pub vid: VendorId,
    pub pid: ProductId,
}

impl DeviceId {
    #[must_use]
    pub const fn new(vid: u16, pid: u16) -> Self {
        Self {
            vid: VendorId(vid),
            pid: ProductId(pid),
        }
    }

    /// Shorthand for Corsair VID (0x1B1C) + a product ID.
    #[must_use]
    pub const fn corsair(pid: u16) -> Self {
        Self::new(0x1B1C, pid)
    }
}

impl core::fmt::Display for VendorId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{:04X}", self.0)
    }
}

impl core::fmt::Display for ProductId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{:04X}", self.0)
    }
}

impl core::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}", self.vid, self.pid)
    }
}
