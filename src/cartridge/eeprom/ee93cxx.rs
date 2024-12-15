use alloc::vec::Vec;
use log::trace;
use serde::{Deserialize, Serialize};
use bitflags::bitflags;
use crate::consts::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum Ee93cxxType {
    C46x8,
    C56x8,
    C66x8,
    C76x8,
    C86x8,
    C46x16,
    C56x16,
    C66x16,
    C76x16,
    C86x16,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub enum Ee93cxxState {
    WaitForStartBit,
    WaitForCommand,
    SendingData,
    WaitForWrite,
    WaitForWriteAll,
}

const EE93CXX_CMD_ERASE:u16 = 0b11;
const EE93CXX_CMD_READ: u16 = 0b10;
const EE93CXX_CMD_WRITE:u16 = 0b01;

const EE93CXX_ADR_WRAL: u16 = 0b01;
const EE93CXX_ADR_ERAL: u16 = 0b10;
const EE93CXX_ADR_EWDS: u16 = 0b00;
const EE93CXX_ADR_EWEN: u16 = 0b11;


bitflags! {
    #[derive(Copy, Clone, Debug, Serialize, Deserialize)]
    pub struct Ee93cxxPins:u8 {
        const DO = 0b00001000;
        const DI = 0b00000100;
        const CLK = 0b00000010;
        const CS = 0b00000001;
    }
}

#[derive(Serialize, Deserialize)]
pub struct Ee93cxxConf {
    size: usize,
    address_bits: u8,
    data_len: u8,
    cart_pins: u32,
    clk_pin_mask: u32,
    cs_pin_mask: u32,
    di_pin_mask: u32,
    do_pin_mask: u32,
}

impl Ee93cxxConf {
    fn new(size: usize, address_bits: u8, data_len: u8) -> Self {
        Self {
            size,
            address_bits,
            data_len,
            cart_pins: 0,
            clk_pin_mask: 1 << (CART_PIN_A1-1),
            cs_pin_mask: 1 << (CART_PIN_A7-1),
            di_pin_mask: 1 << (CART_PIN_AUDIN-1),
            do_pin_mask: 1 << (CART_PIN_AUDIN-1),
        }
    }

    fn address_mask(&self) -> u16 {
        u16::pow(2, self.address_bits.into()) - 1
    }

    fn command_len(&self) -> u8 {
        self.address_bits + 2
    }

    fn load_cart_pins(&mut self, cart_pins: u32) -> Ee93cxxPins {
        self.cart_pins = cart_pins;
        let mut pins = Ee93cxxPins::empty();

        pins.set(Ee93cxxPins::CLK, cart_pins & self.clk_pin_mask != 0);
        pins.set(Ee93cxxPins::CS, cart_pins & self.cs_pin_mask != 0);
        pins.set(Ee93cxxPins::DI, cart_pins & self.di_pin_mask != 0);

        pins
    }
}

#[derive(Serialize, Deserialize)]
pub struct Ee93cxx {
    data: Vec<u16>,
    state: Ee93cxxState,
    prev_clk: bool,
    typ: Ee93cxxType,
    config: Ee93cxxConf,
    shifter: u16,
    shifter_in: u8,
    data_buffer: u32,
    data_buffer_in: u8,
    command: u16,
    ewds: bool,
    last_output: bool,
}

fn config(t: Ee93cxxType) -> Ee93cxxConf {
    match t {
        Ee93cxxType::C46x8 => Ee93cxxConf::new(128, 6, 8),
        Ee93cxxType::C56x8 =>  Ee93cxxConf::new(256, 8, 8),
        Ee93cxxType::C66x8 =>  Ee93cxxConf::new(512, 8, 8),
        Ee93cxxType::C76x8 =>  Ee93cxxConf::new(1024, 10, 8),
        Ee93cxxType::C86x8 =>  Ee93cxxConf::new(2048, 10, 8),
        Ee93cxxType::C46x16 => Ee93cxxConf::new(64, 5, 16),
        Ee93cxxType::C56x16 => Ee93cxxConf::new(128, 7, 16),
        Ee93cxxType::C66x16 => Ee93cxxConf::new(256, 7, 16),
        Ee93cxxType::C76x16 => Ee93cxxConf::new(512, 9, 16),
        Ee93cxxType::C86x16 => Ee93cxxConf::new(1024, 9, 16), 
    }
}

impl Ee93cxx {
    pub fn new(typ: Ee93cxxType) -> Self {
        let config = config(typ);
        Self {
            data: vec![0xFF; config.size],
            config,
            shifter: 0,
            shifter_in: 0,
            data_buffer: 0,
            data_buffer_in: 0,            
            state: Ee93cxxState::WaitForStartBit,
            prev_clk: false,
            command: 0,
            ewds: true,
            last_output: false,
            typ,
        }
    }

    pub fn tick(&mut self, cart_pins: u32) {
        let mut p = self.config.load_cart_pins(cart_pins);

        if p.contains(Ee93cxxPins::CS) {
            if !self.prev_clk && p.contains(Ee93cxxPins::CLK) {
               self.clock(&mut p);
            }            
        } else {
            self.reset();
        }

        self.prev_clk = p.contains(Ee93cxxPins::CLK);
    }

    fn clock(&mut self, p: &mut Ee93cxxPins) {
        match self.state {
            Ee93cxxState::WaitForStartBit => {
                if p.contains(Ee93cxxPins::DI) {
                    self.state = Ee93cxxState::WaitForCommand;
                }
            }
            Ee93cxxState::WaitForCommand => {
                self.shifter <<= 1;
                self.shifter |= if p.contains(Ee93cxxPins::DI) {1} else {0};
                self.shifter_in += 1;
                if self.shifter_in == self.config.command_len() {
                    trace!("clock cmd:{:02b} {:016b}", (self.shifter >> self.config.address_bits) & 0b11, self.shifter);
                    match (self.shifter >> self.config.address_bits) & 0b11 {
                        0 => match (self.shifter >> (self.config.address_bits - 2)) & 0b11 {
                            EE93CXX_ADR_ERAL => self.eral(),
                            EE93CXX_ADR_EWDS => self.ewds(),
                            EE93CXX_ADR_EWEN => self.ewen(),
                            EE93CXX_ADR_WRAL => self.state = Ee93cxxState::WaitForWriteAll,
                            _ => self.reset(),  
                        } 
                        EE93CXX_CMD_ERASE => self.erase(),
                        EE93CXX_CMD_READ => self.read(),
                        EE93CXX_CMD_WRITE => self.state = Ee93cxxState::WaitForWrite,
                        _ => self.reset(),
                    }
                }                
            }
            Ee93cxxState::SendingData => {
                self.last_output = self.data_buffer & (1 << self.data_buffer_in) != 0;
                if self.data_buffer_in == 0 {
                    self.reset();
                } else {
                    self.data_buffer_in -= 1;            
                }
            }
            Ee93cxxState::WaitForWrite => if self.data_buffer_in == self.config.data_len {
                self.write();
            } else {
                self.data_buffer |= if p.contains(Ee93cxxPins::DI) {1} else {0};
                self.data_buffer <<= 1;   
                self.data_buffer_in += 1;
            },
            Ee93cxxState::WaitForWriteAll => if self.data_buffer_in == self.config.data_len {
                self.wral();
            } else {
                self.data_buffer |= if p.contains(Ee93cxxPins::DI) {1} else {0};
                self.data_buffer <<= 1;   
                self.data_buffer_in += 1;
            },
        }
    }

    fn address(&self) -> usize {
        (self.shifter & self.config.address_mask()) as usize
    }

    fn write(&mut self) {
        if !self.ewds {
            let addr = self.address();
            self.data[addr] = self.data_buffer as u16;
            trace!("write 0x{:04X} with 0x{:04X}", self.address(), self.data_buffer);
        } else {
            trace!("write disabled");
        }

        self.last_output = true;
        self.reset();
    }

    fn wral(&mut self) {
        if !self.ewds {
            self.data.fill(self.data_buffer as u16);
            trace!("wral with 0x{:02X}", self.data_buffer);
        } else {
            trace!("wral disabled");
        }
        self.last_output = true;
        self.reset();        
    }

    fn reset(&mut self) {
        self.shifter = 0;
        self.shifter_in = 0;
        self.data_buffer = 0;
        self.data_buffer_in = 0;
        self.state = Ee93cxxState::WaitForStartBit;
    }

    fn erase(&mut self) {
        if !self.ewds {
            let addr = self.address();
            self.data[addr] = 0xFFFF;
            trace!("erase 0x{:04X} with 0x{:04X}", self.address(), 0xFFFF);
        } else {
            trace!("erase disabled");
        }
        self.last_output = true;
        self.reset();        
    }
    
    fn eral(&mut self) {
        if !self.ewds {
            self.data.fill(0xFFFF);
            trace!("eral");
        } else {
            trace!("eral disabled");
        }
        self.last_output = true;
        self.reset();
    }
    
    fn ewds(&mut self) {
        self.ewds = true;
        trace!("ewds");
        self.reset();
    }
    
    fn ewen(&mut self) {
        self.ewds = false;
        trace!("ewen");
        self.reset();
    } 

    fn read(&mut self) {
        let addr = self.address();
        self.data_buffer = self.data[addr] as u32;
        self.data_buffer_in = self.config.data_len - 1;
        trace!("read 0x{:04X}: 0x{:04X}", self.address(), self.data_buffer);
        self.state = Ee93cxxState::SendingData;
    }

    pub fn audin(&self) -> bool {
        self.last_output
    }
}