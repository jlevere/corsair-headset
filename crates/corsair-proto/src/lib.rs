#![cfg_attr(not(feature = "std"), no_std)]

pub mod devices;
pub mod error;
pub mod report;
pub mod types;

pub mod bragi;
pub mod cxaudio;
pub mod legacy;

pub use error::{DecodeError, ProtocolError};
pub use report::Report;
pub use types::{DeviceId, ProductId, VendorId};
