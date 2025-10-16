#![no_std]
#[macro_use]
extern crate alloc;

pub mod bus;
pub mod cartridge;
pub mod consts;
pub mod lynx;
pub mod mikey;
pub mod ram;
pub mod rom;
pub mod shared_memory;
pub mod suzy;
pub mod vectors;

/// Serializes a Lynx instance into a byte array.
///
/// # Errors
///
/// Returns `Err` with a descriptive message if:
/// - The serialization operation fails due to insufficient buffer space
/// - There are encoding issues with the data
/// - The postcard serialization encounters an error
pub fn serialize(lynx: &lynx::Lynx, data: &mut [u8]) -> Result<(), &'static str> {
    match postcard::to_slice(&lynx, data) {
        Err(_) => Err("Serialization error."),
        Ok(_) => Ok(()),
    }
}

/// Deserializes a byte array into a Lynx instance.
///
/// # Errors
///
/// Returns `Err` with a descriptive message if:
/// - The deserialization operation fails due to invalid data format
/// - The postcard deserialization encounters an error
pub fn deserialize(data: &[u8], source: &lynx::Lynx) -> Result<lynx::Lynx, &'static str> {
    let Ok(mut lynx) = postcard::from_bytes::<lynx::Lynx>(data) else {
        return Err("Deserialization error");
    };
    lynx.cart_mut().copy_from(source.cart());
    Ok(lynx)
}

#[must_use]
pub const fn info() -> (&'static str, &'static str) {
    ("Holani", env!("CARGO_PKG_VERSION"))
}

#[must_use]
pub const fn valid_extensions() -> &'static [&'static str] {
    &["lnx", "o"]
}
