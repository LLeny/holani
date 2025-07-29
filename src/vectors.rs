use crate::{
    bus::{Bus, BusStatus},
    consts::{INTV_ADDR, NMIV_ADDR, RESV_ADDR},
};
use log::trace;
use serde::{Deserialize, Serialize};

const VECTOR_NORMAL_READ_TICKS: i8 = 5;
const VECTOR_NORMAL_WRITE_TICKS: i8 = 5;

#[derive(Serialize, Deserialize)]
pub struct Vectors {
    data: [u8; 6],
    addr_r: u16,
    data_r: u8,
    ticks_to_done: i8,
    write: bool,
    ticks: u64,
}

impl Vectors {
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: [0, 0, 0x80, 0xff, 0, 0],
            addr_r: 0,
            data_r: 0,
            ticks_to_done: -1,
            write: false,
            ticks: 0,
        }
    }

    pub fn from_slice(&mut self, data: &[u8]) {
        self.data.clone_from_slice(data);
    }

    #[must_use]
    pub fn get(&self, addr: u16) -> u8 {
        self.data[(addr - NMIV_ADDR) as usize]
    }

    pub fn peek(&mut self, bus: &Bus) {
        self.ticks_to_done = VECTOR_NORMAL_READ_TICKS;
        self.addr_r = bus.addr();
        self.write = false;
        trace!("[{}] > Peek 0x{:04x}", self.ticks, bus.addr());
    }

    pub fn poke(&mut self, bus: &Bus) {
        self.ticks_to_done = VECTOR_NORMAL_WRITE_TICKS;
        self.addr_r = bus.addr();
        self.write = true;
        self.data_r = bus.data();
        trace!(
            "[{}] > Poke 0x{:04x} = 0x{:02x}",
            self.ticks,
            bus.addr(),
            bus.data()
        );
    }

    pub fn tick(&mut self, bus: &mut Bus) {
        match self.ticks_to_done {
            -1 => (),
            0 => {
                if self.write {
                    self.data[(self.addr_r - NMIV_ADDR) as usize] = self.data_r;
                    bus.set_status(BusStatus::PokeDone);
                    trace!("[{}] < Poke 0x{:02x}", self.ticks, self.data_r);
                } else {
                    bus.set_data(self.data[(self.addr_r - NMIV_ADDR) as usize]);
                    bus.set_status(BusStatus::PeekDone);
                    trace!("[{}] < Peek", self.ticks);
                }
                self.ticks_to_done = -1;
            }
            _ => self.ticks_to_done -= 1,
        }
        self.ticks += 1;
    }
    #[must_use]
    pub fn write(&self) -> bool {
        self.write
    }

    #[must_use]
    pub fn ready(&self) -> bool {
        self.ticks_to_done == -1
    }

    #[must_use]
    pub fn data(&self, addr: u16) -> u8 {
        self.data[(addr - NMIV_ADDR) as usize]
    }

    #[must_use]
    pub fn u16(&self, addr: u16) -> u16 {
        u16::from(self.data(addr)) | (u16::from(self.data(addr + 1)) << 8)
    }

    #[must_use]
    pub fn interrupt(&self) -> u16 {
        self.u16(INTV_ADDR)
    }

    #[must_use]
    pub fn nmi(&self) -> u16 {
        self.u16(NMIV_ADDR)
    }

    #[must_use]
    pub fn reset(&self) -> u16 {
        self.u16(RESV_ADDR)
    }
}

impl Default for Vectors {
    fn default() -> Self {
        Vectors::new()
    }
}
