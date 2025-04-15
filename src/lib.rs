#![no_std]
#[macro_use]
extern crate alloc;

pub mod bus;
pub mod cartridge;
pub mod mikey;
pub mod ram;
pub mod rom;
pub mod suzy;
pub mod vectors;
pub mod consts;
pub mod lynx;
pub mod shared_memory;

pub fn serialize(lynx: &lynx::Lynx, data: &mut [u8]) -> Result<(), &'static str> {
    match postcard::to_slice(&lynx, data) {
        Err(_) => Err("Serialization error."),
        Ok(_) => Ok(()),
    }
}

pub fn deserialize(data: &[u8], source: &lynx::Lynx) -> Result<lynx::Lynx, &'static str> {
    let mut lynx = match postcard::from_bytes::<lynx::Lynx>(data) {
        Err(_) => return Err("Deserialization error"),
        Ok(l) => l
    };
    lynx.cart_mut().copy_from(source.cart());
    Ok(lynx)
}

pub const fn info() -> (&'static str, &'static str) {
    ("Holani", env!("CARGO_PKG_VERSION"))
}

pub const fn valid_extensions() -> &'static [&'static str] {
    &["lnx", "o"]
}