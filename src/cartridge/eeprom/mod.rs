mod ee93cxx;

use ee93cxx::Ee93cxx;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum EEpromType {
    Ee93c46x8,
    Ee93c56x8,
    Ee93c66x8,
    Ee93c76x8,
    Ee93c86x8,
    Ee93c46x16,
    Ee93c56x16,
    Ee93c66x16,
    Ee93c76x16,
    Ee93c86x16,
}

#[derive(Serialize, Deserialize)]
pub enum EepromI {
    EE93CXX(Ee93cxx),
}

#[derive(Serialize, Deserialize)]
pub struct Eeprom {
    eeprom: EepromI,
}

impl Eeprom {
    pub fn new(t: &EEpromType) -> Self {
        Self {
            eeprom: match t {
                EEpromType::Ee93c46x8 => {
                    EepromI::EE93CXX(Ee93cxx::new(ee93cxx::Ee93cxxType::C46x8))
                }
                EEpromType::Ee93c56x8 => {
                    EepromI::EE93CXX(Ee93cxx::new(ee93cxx::Ee93cxxType::C56x8))
                }
                EEpromType::Ee93c66x8 => {
                    EepromI::EE93CXX(Ee93cxx::new(ee93cxx::Ee93cxxType::C66x8))
                }
                EEpromType::Ee93c76x8 => {
                    EepromI::EE93CXX(Ee93cxx::new(ee93cxx::Ee93cxxType::C76x8))
                }
                EEpromType::Ee93c86x8 => {
                    EepromI::EE93CXX(Ee93cxx::new(ee93cxx::Ee93cxxType::C86x8))
                }
                EEpromType::Ee93c46x16 => {
                    EepromI::EE93CXX(Ee93cxx::new(ee93cxx::Ee93cxxType::C46x16))
                }
                EEpromType::Ee93c56x16 => {
                    EepromI::EE93CXX(Ee93cxx::new(ee93cxx::Ee93cxxType::C56x16))
                }
                EEpromType::Ee93c66x16 => {
                    EepromI::EE93CXX(Ee93cxx::new(ee93cxx::Ee93cxxType::C66x16))
                }
                EEpromType::Ee93c76x16 => {
                    EepromI::EE93CXX(Ee93cxx::new(ee93cxx::Ee93cxxType::C76x16))
                }
                EEpromType::Ee93c86x16 => {
                    EepromI::EE93CXX(Ee93cxx::new(ee93cxx::Ee93cxxType::C86x16))
                }
            },
        }
    }

    pub fn tick(&mut self, cart_pins: u32) {
        match &mut self.eeprom {
            EepromI::EE93CXX(ee) => ee.tick(cart_pins),
        }
    }

    pub fn audin(&self) -> bool {
        match &self.eeprom {
            EepromI::EE93CXX(ee) => ee.audin(),
        }
    }
}
