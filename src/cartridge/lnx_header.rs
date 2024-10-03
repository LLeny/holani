use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct LNXHeader {
    rotation: u8,
    manufacturer: String,
    title: String,
    version: u16,
    bank0_size: u16,
    bank1_size: u16,
    spare: Vec<u8>,
}

#[allow(dead_code)]
impl LNXHeader {
    pub fn new() -> Self {
        Self {
            rotation: 0,
            manufacturer: "unknown".to_string(),
            title: "unknown".to_string(),
            version: 0,
            bank0_size: 0,
            bank1_size: 0,
            spare: Default::default(),
        }
    }

    pub fn rotation(&self) -> u8 {
        self.rotation
    }

    pub fn manufacturer(&self) -> &str {
        &self.manufacturer
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn version(&self) -> u16 {
        self.version
    }

    pub fn bank0_size(&self) -> u16 {
        self.bank0_size
    }

    pub fn bank1_size(&self) -> u16 {
        self.bank1_size
    }

    pub fn spare(&self) -> &Vec<u8> {
        &self.spare
    }
    
    pub fn set_rotation(&mut self, rotation: u8) {
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

    pub fn eeprom(&self) -> u8 {
        self.spare[1]
    }
}

impl Default for LNXHeader {
    fn default() -> Self {
        Self::new()
    }
}
