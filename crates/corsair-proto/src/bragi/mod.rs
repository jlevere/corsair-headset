//! Corsair Bragi Protocol.
//!
//! Property-based HID protocol for newer devices (HS80, HS55, keyboards, mice).
//! Communication uses typed property read/write via [`PropertyId`].

pub mod convert;
pub mod property_id;
pub mod types;

pub use property_id::PropertyId;
