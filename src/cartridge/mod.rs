pub mod lnx_header;
mod cartridge_generic;
mod eeprom;
mod no_intro;
use std::io::Error;
use cartridge_generic::*;
use eeprom::Eeprom;
use lnx_header::{LNXHeader, LNXRotation};
use log::error;
use mikey::registers::MikeyRegisters;
use no_intro::check_no_intro;
use crate::*;

const LNX_HEADER_LENGTH: usize = 64;
const BS93_HEADER_LENGTH: usize = 10;

const DATA_PINS: [u32; 8] = [CART_PIN_D0, CART_PIN_D1, CART_PIN_D2, CART_PIN_D3, CART_PIN_D4, CART_PIN_D5, CART_PIN_D6, CART_PIN_D7];
const RIPPLE_PINS: [u32; 11] = [CART_PIN_A0, CART_PIN_A1, CART_PIN_A2, CART_PIN_A3, CART_PIN_A4, CART_PIN_A5, CART_PIN_A6, CART_PIN_A7, CART_PIN_A8, CART_PIN_A9, CART_PIN_A10];
const SHIFTER_PINS: [u32; 8] = [CART_PIN_A12, CART_PIN_A13, CART_PIN_A14, CART_PIN_A15, CART_PIN_A16, CART_PIN_A17, CART_PIN_A18, CART_PIN_A19 ];

const _128K: usize = usize::pow(2, 17);
const _256K: usize = _128K * 2;
const _512K: usize = _256K * 2;
const _1024K: usize = _512K * 2;

// Courtesy of https://github.com/42Bastian/new_bll/
const BLL_LOADER: [u8; 246] = [0xFF, 0x4A, 0x37, 0xB2, 0xB3, 0x0D, 0xEF, 0x61, 0x56, 0xAB, 0xD3, 0xC3, 0x5D, 0x4B, 0xDE, 0xB8,0x38, 0x17, 0x92, 0x59, 0xFA, 0x40, 0xB1, 0x58, 0xC4, 0x8F, 0xB6, 0x6D, 0xBE, 0xBB, 0x20, 0x8E,0x8A, 0x69, 0x86, 0x6C, 0x18, 0x12, 0x0C, 0x7C, 0x50, 0xCD, 0xAA, 0x63, 0x41, 0x3F, 0xD3, 0x89,0xAD, 0xAB, 0x37, 0x14, 0x01, 0xAD, 0xC5, 0x02, 0x49, 0xFF, 0x85, 0xF1, 0xAD, 0xC6, 0x02, 0x49,0xFF, 0x85, 0xF0, 0xAD, 0xC3, 0x02, 0x85, 0xF3, 0x85, 0xF5, 0xAD, 0xC4, 0x02, 0x85, 0xF2, 0x85,0xF4, 0xA2, 0xC0, 0x9A, 0xA0, 0x29, 0xB9, 0x2D, 0x02, 0x99, 0xC0, 0x01, 0x88, 0xD0, 0xF7, 0xA2,0x03, 0x80, 0x9F, 0xCA, 0xD0, 0x09, 0xE6, 0x00, 0xA5, 0x00, 0x20, 0x00, 0xFE, 0xA2, 0x04, 0xAD,0xB2, 0xFC, 0x92, 0xF2, 0xE6, 0xF2, 0xD0, 0x02, 0xE6, 0xF3, 0xE6, 0xF0, 0xD0, 0x07, 0xE6, 0xF1,0xD0, 0x03, 0x6C, 0xF4, 0x00, 0xC8, 0xD0, 0xE7, 0x80, 0xD9, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

#[macro_export]
macro_rules! TO_U16 {
    ($b0:expr,$b1:expr) => {
        ($b1 as u16) << 8 | $b0 as u16
    };
}

fn write_pins(mut pins: u32, data: u16, data_pins: &[u32]) -> u32 {
    let mut shift: u16 = 1;
    for pin in data_pins.iter() {
        if *pin == 0 {
            break;
        }
        if data & shift != 0 {
            pins |= 1 << (*pin-1);
        } else {
            pins &= !(1 << (*pin-1));
        }
        shift <<= 1;
    }
    pins 
}

fn write_data_pins(pins: u32, data: u8) -> u32 {
    write_pins(pins, data as u16, &DATA_PINS)
}

fn read_pins_u8(pins: u32, data_pins: &[u32]) -> u8 {
    let mut shift: u8 = 1;
    let mut r: u8 = 0;
    for p in data_pins.iter() {
        if *p == 0 {
            break;
        }
        if (pins & (1 << (*p-1))) != 0 { 
            r |= shift; 
        };
        shift <<= 1;
    }
    r
}

fn read_pins_u16(pins: u32, data_pins: &[u32]) -> u16 {
    let mut shift: u16 = 1;
    let mut r: u16 = 0;
    for p in data_pins.iter() {
        if *p == 0 {
            break;
        }
        if (pins & (1 << (*p-1))) != 0 { 
            r |= shift; 
        };
        shift <<= 1;
    }
    r
}

pub trait CartridgeI {
    fn load(&mut self, file_content: &[u8]);
    fn set_pins(&mut self, pins: u32);
    fn pins(&self) -> u32;
}

#[derive(Serialize, Deserialize)]
enum CartType {
    None(),
    Generic(CartridgeGeneric),
}

#[derive(Serialize, Deserialize)]
pub struct Cartridge {
    ticks_to_done: u8,
    #[serde(skip)]
    header: LNXHeader,
    cart: CartType,
    eeprom: Option<Eeprom>,
    healthy: bool,
}

impl Default for Cartridge {
    fn default() -> Self {
        Self { 
            ticks_to_done: 0, 
            header: LNXHeader::new(), 
            cart: CartType::None(),
            eeprom: None,
            healthy: false,
        }
    }
}

impl Cartridge {
    pub fn from_slice(data: &[u8]) -> Result<Self, Error> {
        let mut cart = Self::default();

        if cart.is_lnx(data) {
            cart.lnx(data);
        } else if cart.is_bs93(data) {
            cart.bs93(data);
        } else if cart.is_nointro(data) {
            cart.nointro(data);
        }
        else {
            return Err(Error::new(std::io::ErrorKind::Other, "Couldn't identify cart file format."));
        }
        
        Ok(cart)
    }

    pub fn reset(&mut self) {
        self.set_cart_pins(0);
        self.ticks_to_done = 0;
    }

    fn is_bs93(&self, file_content: &[u8]) -> bool {
        file_content.len() > BS93_HEADER_LENGTH && 
        &file_content[6..=9] == b"BS93"
    }

    fn is_lnx(&self, file_content: &[u8]) -> bool {
        file_content.len() > LNX_HEADER_LENGTH && 
        &file_content[0..=3] == b"LYNX"
    }

    fn is_nointro(&self, file_content: &[u8]) -> bool {
        check_no_intro(file_content).is_ok()
    }

    fn bs93(&mut self, file_content: &[u8]) {
        let mut cart = CartridgeGeneric::new(1024, &_256K_PINS);
        let mut content: Vec<u8> = vec![];
        content.extend(BLL_LOADER);
        content.extend(file_content);
        let fill_size: usize = 256 * usize::pow(2, 10) - BLL_LOADER.len() - file_content.len();
        let fill_vec: Vec<u8> = vec![0; fill_size];
        content.extend(fill_vec);
        cart.load(&content);
        self.cart = CartType::Generic(cart);
        self.healthy = true;
    }

    fn nointro(&mut self, file_content: &[u8]) {
        let l = file_content.len();
        let mut cart = 
        if l <= _128K {
           CartridgeGeneric::new(512, &_128K_PINS)
        } else if l <= _256K {
             CartridgeGeneric::new(1024, &_256K_PINS)
        } else if l <=_512K {
            CartridgeGeneric::new(2048, &_512K_PINS)
        } else if l <=_1024K {
            CartridgeGeneric::new(4096, &_1024KAUDIN_PINS)
        } else {
           panic!("Not a No-Intro")
        }; 

        cart.load(file_content);
        self.cart = CartType::Generic(cart);

        if let Ok(cart_info) = check_no_intro(file_content) {
            self.header.set_title(cart_info.0.to_string());
            self.header.set_rotation(cart_info.1);
        }

        self.healthy = true;
    }

    fn lnx(&mut self, file_content: &[u8]) {
        self.load_lnx_header(file_content);

        let bank_size = self.header.bank0_size();
        match match bank_size {
                512 => Ok(&_128K_PINS),
                1024 => Ok(&_256K_PINS),
                2048 => Ok(&_512K_PINS),
                4096 => Ok(&_1024KAUDIN_PINS),
                _ => Err("Unknown cart bank size."),
         } {
            Err(e) => error!("{:?}",e),
            Ok(pins) => {
                let mut cart = CartridgeGeneric::new(bank_size as u32, pins);
                cart.load(&file_content[LNX_HEADER_LENGTH..]);
                self.cart = CartType::Generic(cart);
                self.healthy = true;
            }
         }

         self.eeprom = match self.header.eeprom() & 0b1000_0111 {
            0x01 => Some(Eeprom::new(eeprom::EEpromType::Ee93c46x8)),
            0x02 => Some(Eeprom::new(eeprom::EEpromType::Ee93c56x8)),
            0x03 => Some(Eeprom::new(eeprom::EEpromType::Ee93c66x8)),
            0x04 => Some(Eeprom::new(eeprom::EEpromType::Ee93c76x8)),
            0x05 => Some(Eeprom::new(eeprom::EEpromType::Ee93c86x8)),
            0x81 => Some(Eeprom::new(eeprom::EEpromType::Ee93c46x16)),
            0x82 => Some(Eeprom::new(eeprom::EEpromType::Ee93c56x16)),
            0x83 => Some(Eeprom::new(eeprom::EEpromType::Ee93c66x16)),
            0x84 => Some(Eeprom::new(eeprom::EEpromType::Ee93c76x16)),
            0x85 => Some(Eeprom::new(eeprom::EEpromType::Ee93c86x16)),
            _ => None,
         }
    }

    fn load_lnx_header(&mut self, file_content: &[u8]) {
        self.header.set_bank0_size(TO_U16!(file_content[4], file_content[5]));
        self.header.set_bank1_size(TO_U16!(file_content[6], file_content[7]));
        self.header.set_version(TO_U16!(file_content[8], file_content[9]));
        self.header.set_title(match String::from_utf8(file_content[10..=41].to_vec()) {
            Ok(t) => t,
            Err(_) => "Error".to_string()
        });
        self.header.set_manufacturer(match String::from_utf8(file_content[42..=58].to_vec()) {
            Ok(m) => m,
            Err(_) => "Error".to_string()
        });
        self.header.set_rotation(match file_content[58] {
            1 => LNXRotation::_270,
            2 => LNXRotation::_90,
            _ => LNXRotation::None,
        });

        self.header.set_spare(file_content[59..=63].to_vec());
    }

    pub fn write_address_to_pins(&mut self, shifter: u8, ripple: u16, audin: u16) {
        let mut pins = self.cart_pins();

        pins = write_pins(pins, shifter as u16, &SHIFTER_PINS);
        pins = write_pins(pins, ripple, &RIPPLE_PINS);
        pins = write_pins(pins, audin, &[CART_PIN_AUDIN]);

        self.set_cart_pins(pins);
    }

    fn cart_pins(&self) -> u32 {
        match &self.cart {
            CartType::Generic(c) => c.pins(),
            _ => panic!("Trying to write to inexistant cart."),
        }
    }

    pub fn audin(&self) -> bool {
        match &self.eeprom {
            None => false,
            Some(ee) => ee.audin()
        }        
    }

    fn set_cart_pins(&mut self, pins: u32) {
        match &mut self.cart {
            CartType::Generic(c) => c.set_pins(pins),
            _ => panic!("Trying to write to inexistant cart."),
        };
        if let Some(ee) = &mut self.eeprom {
            ee.tick(pins);
        }
    }

    fn set_pin(&mut self, pin: u32) {
        let mut pins = self.cart_pins();
        pins |= pin;
        self.set_cart_pins(pins);
    }

    fn clear_pin(&mut self, pin: u32) {
        let mut pins = self.cart_pins();
        pins &= !pin;
        self.set_cart_pins(pins);
    }

    pub fn rotation(&self) -> LNXRotation {
        self.header.rotation()
    }

    pub fn tick(&mut self, bus: &mut Bus, mikey_regs: &mut MikeyRegisters, switches: &mut Switches) {
        let buss = bus.status();

        match self.ticks_to_done {
            0 => match buss {
                    BusStatus::PeekCart0 => {
                        self.ticks_to_done = CART_READ_TICKS;
                        switches.set(Switches::cart0_inactive, false);
                    }
                    BusStatus::PeekCart1 => {
                        self.ticks_to_done = CART_READ_TICKS;
                        switches.set(Switches::cart1_inactive, false);
                    }
                    BusStatus::PokeCart0 => {
                        self.ticks_to_done = CART_WRITE_TICKS;
                        switches.set(Switches::cart0_inactive, false);
                    }
                    BusStatus::PokeCart1 => {
                        self.ticks_to_done = CART_WRITE_TICKS;
                        switches.set(Switches::cart1_inactive, false);
                    }
                    _ => (),
                }
            1 => {
                 match buss {
                    BusStatus::PeekCart0 => { 
                        if mikey_regs.data(SYSCTL1) & SYSCTL1_POWER != 0 {
                            self.set_pin(CART_PIN_CE); 
                            let data = read_pins_u8(self.cart_pins(), &DATA_PINS);
                            bus.set_data(data);
                            self.clear_pin(CART_PIN_CE); 
                        }
                        else {
                            bus.set_data(0xff);
                        }
                        bus.set_status(BusStatus::PeekIncCartRipple);
                        switches.set(Switches::cart0_inactive, true);
                    },
                    BusStatus::PeekCart1 => { 
                        bus.set_data(0xff);
                        bus.set_status(BusStatus::PeekIncCartRipple);
                        switches.set(Switches::cart1_inactive, true);
                    }
                    BusStatus::PokeCart0 => { 
                        bus.set_status(BusStatus::PokeIncCartRipple);
                        switches.set(Switches::cart0_inactive, true);
                    },
                    BusStatus::PokeCart1 => { 
                        bus.set_status(BusStatus::PokeIncCartRipple);
                        switches.set(Switches::cart1_inactive, true);
                    }
                    _ => ()
                }
                self.ticks_to_done = 0;
            }
            _ => self.ticks_to_done -= 1,
        }
    }

    pub fn copy_from(&mut self, other: &Cartridge) {
        self.header = other.header.clone();
        match &other.cart {
            CartType::Generic(from) => match &mut self.cart {
                CartType::Generic(to) => to.copy_from(from),
                _ => panic!("Trying to write to inexistant cart."),
            }
            _ => panic!("Trying to read from an inexistant cart."),
          };
    }
}
