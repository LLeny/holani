use log::trace;

use super::*;

pub const _1024KAUDIN_PINS: [u32; 16] = [CART_PIN_A0,CART_PIN_A1,CART_PIN_A2,CART_PIN_A3,CART_PIN_A4,CART_PIN_A5,CART_PIN_A6,CART_PIN_A7,CART_PIN_A8,CART_PIN_A9,CART_PIN_A10,CART_PIN_AUDIN,0,0,0,0];
pub const _512K_PINS: [u32; 16] = [CART_PIN_A0,CART_PIN_A1,CART_PIN_A2,CART_PIN_A3,CART_PIN_A4,CART_PIN_A5,CART_PIN_A6,CART_PIN_A7,CART_PIN_A8,CART_PIN_A9,CART_PIN_A10,0,0,0,0,0];
pub const _256K_PINS: [u32; 16] = [CART_PIN_A0,CART_PIN_A1,CART_PIN_A2,CART_PIN_A3,CART_PIN_A4,CART_PIN_A5,CART_PIN_A6,CART_PIN_A7,CART_PIN_A8,CART_PIN_A9,0,0,0,0,0,0];
pub const _128K_PINS: [u32; 16] = [CART_PIN_A0,CART_PIN_A1,CART_PIN_A2,CART_PIN_A3,CART_PIN_A4,CART_PIN_A5,CART_PIN_A6,CART_PIN_A7,CART_PIN_A8,0,0,0,0,0,0,0];
pub const BLOCK_PINS: [u32; 8] = [CART_PIN_A12,CART_PIN_A13,CART_PIN_A14,CART_PIN_A15,CART_PIN_A16,CART_PIN_A17,CART_PIN_A18,CART_PIN_A19];

#[derive(Serialize, Deserialize)]
pub struct CartridgeGeneric {
    pins: u32,
    banks: Vec<Vec<u8>>,
    addr_pins: Vec<u32>,
    block_pins: Vec<u32>,
    bank_size: u32,
}

impl CartridgeGeneric {
    pub fn new(bank_size: u32, data_pins: &[u32]) -> Self {
        Self {
            banks: Vec::new(),
            pins: 0,
            addr_pins: data_pins.to_vec(),
            block_pins: BLOCK_PINS.to_vec(),
            bank_size,
        }
    }

    fn block(&self, pins: u32) -> u16 {
        read_pins_u16(pins, &self.block_pins)
    }

    fn addr(&self, pins: u32) -> u16 {
        read_pins_u16(pins, &self.addr_pins)
    }

    fn data_address(&self, pins: u32) -> usize {
        let block = self.block(pins) as u32;
        let addr = self.addr(pins) as u32;
        trace!("block:0x{:08X} addr:0x{:08X}", block, addr);
        (block * self.bank_size + addr) as usize
    }

    fn read(&mut self, pins: u32) -> u32 {
        let addr = self.data_address(pins);
        let data = self.banks[0][addr];
        trace!("Read 0x{:06x} data:0x{:02x}", addr, data);
        write_data_pins(pins, data) 
    }

    fn write(&mut self, pins: u32) -> u32 {
        let addr = self.data_address(pins);
        let data = read_pins_u8(pins, DATA_PINS.as_ref());
        self.banks[0][addr] = data;
        trace!("Write 0x{:06x} data:0x{:02x}", addr, data);
        pins
    }
}

impl CartridgeI for CartridgeGeneric {
    fn load(&mut self, file_content: &[u8]){
        self.banks.clear();
        self.banks.push(file_content.to_vec());
    }

    fn set_pins(&mut self, mut pins: u32) {
        if self.pins & CART_PIN_CE == 0 && pins & CART_PIN_CE != 0 {
            pins = self.read(pins);
        }
        else if self.pins & CART_PIN_WE == 0 && pins & CART_PIN_WE != 0 {
            pins = self.write(pins);
        }

        self.pins = pins;
    }

    fn pins(&self) -> u32 {
        self.pins
    }    
}