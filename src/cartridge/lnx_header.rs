use alloc::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Default)]
#[repr(u8)]
pub enum LNXRotation {
    #[default]
    None = 0,
    _270 = 1,
    _90 = 2,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LNXHeader {
    rotation: LNXRotation,
    manufacturer: String,
    title: String,
    version: u16,
    bank0_size: u16,
    bank1_size: u16,
    spare: Vec<u8>,
}

#[allow(dead_code)]
impl LNXHeader {
    #[must_use]
    pub fn new() -> Self {
        Self {
            rotation: LNXRotation::default(),
            manufacturer: "unknown".into(),
            title: "unknown".into(),
            version: 0,
            bank0_size: 0,
            bank1_size: 0,
            spare: Vec::default(),
        }
    }

    #[must_use]
    pub fn rotation(&self) -> LNXRotation {
        self.rotation
    }

    #[must_use]
    pub fn manufacturer(&self) -> &str {
        &self.manufacturer
    }

    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    #[must_use]
    pub fn version(&self) -> u16 {
        self.version
    }

    #[must_use]
    pub fn bank0_size(&self) -> u16 {
        self.bank0_size
    }

    #[must_use]
    pub fn bank1_size(&self) -> u16 {
        self.bank1_size
    }

    #[must_use]
    pub fn spare(&self) -> &Vec<u8> {
        &self.spare
    }

    pub fn set_rotation(&mut self, rotation: LNXRotation) {
        self.rotation = rotation;
    }

    pub fn set_manufacturer(&mut self, manufacturer: String) {
        self.manufacturer = manufacturer;
    }

    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }

    pub fn set_version(&mut self, version: u16) {
        self.version = version;
    }

    pub fn set_bank0_size(&mut self, bank0_size: u16) {
        self.bank0_size = bank0_size;
    }

    pub fn set_bank1_size(&mut self, bank1_size: u16) {
        self.bank1_size = bank1_size;
    }

    pub fn set_spare(&mut self, spare: Vec<u8>) {
        self.spare = spare;
    }

    #[must_use]
    pub fn eeprom(&self) -> u8 {
        self.spare[1]
    }
}

impl Default for LNXHeader {
    fn default() -> Self {
        Self::new()
    }
}
