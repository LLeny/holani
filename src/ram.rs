use log::trace;
use crate::{consts::*, shared_memory::SharedMemory};
use serde::{Serialize, Deserialize};
use super::bus::*;

pub const RAM_MAX: u16 = 0xffff;

#[derive(Serialize, Deserialize)]
pub struct Ram {
    data: SharedMemory,
    addr_r: u16,
    data_r: u8,
    ticks_to_done: i8,
    write: bool,
    ticks: u64,
}

impl Ram {
    pub fn new() -> Ram {
        let mut r = Ram {
            data: SharedMemory::new((RAM_MAX as usize) + 1, 0xFF),
            ticks_to_done: -1,
            addr_r: 0,
            data_r: 0,
            write: false,
            ticks: 0,
        };
        r.data[MMC_ADDR as usize] = 0;
        r
    }

    #[inline]
    pub fn get(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }

    #[inline]
    pub fn set(&mut self, addr: u16, data: u8) {
        self.data[addr as usize] = data;
    }

    #[inline]
    pub fn fill(&mut self, v: u8) {
        self.data.fill(v);
    }

    pub fn copy(&mut self, dest: u16, buf: &[u8]) {
        assert!(dest as usize + buf.len() <= RAM_MAX as usize);
        self.data.copy(dest, buf);
    }

    pub fn peek(&mut self, bus: &Bus) {
        if bus.addr() & 0xff00 == self.addr_r & 0xff00 {
            self.ticks_to_done = RAM_PAGE_READ_TICKS;
            trace!("[{}] > Peek 0x{:04x} (page mode)", self.ticks, bus.addr());
        } else {
            self.ticks_to_done = RAM_NORMAL_READ_TICKS;
            trace!("[{}] > Peek 0x{:04x} (normal mode)", self.ticks, bus.addr());
        }
        self.addr_r = bus.addr();
        self.write = false;
    }

    pub fn poke(&mut self, bus: &Bus) {
        self.ticks_to_done = RAM_NORMAL_WRITE_TICKS;
        self.addr_r = bus.addr();
        self.write = true;
        self.data_r = bus.data();
        trace!("[{}] > Poke 0x{:04x} = 0x{:02x}", self.ticks, self.addr_r, self.data_r);
    }

    pub fn tick(&mut self, bus: &mut Bus) {
        match self.ticks_to_done {
            -1 => (),
            0 => {
                if self.write {
                    self.data[self.addr_r as usize] = self.data_r;
                    bus.set_status(BusStatus::PokeDone);
                    trace!("[{}] < Poke 0x{:02x}", self.ticks, self.data_r);
                } else {
                    bus.set_data(self.data[self.addr_r as usize]);
                    bus.set_status(BusStatus::PeekDone);
                    trace!("[{}] < Peek 0x{:04x} -> 0x{:02x}", self.ticks, self.addr_r, bus.data());
                }
                self.ticks_to_done = -1;
            }
            _ => self.ticks_to_done -= 1,
        };
        self.ticks += 1;
    }

    #[inline]
    pub fn mmapctl(&self) -> u8 {
        self.data[MMC_ADDR as usize]
    }

    #[inline]
    pub fn set_mmapctl(&mut self, data: u8) {
        self.data[MMC_ADDR as usize] = data;
    }

    #[inline]
    pub fn write(&self) -> bool {
        self.write
    }

    #[inline]
    pub fn data(&self) -> &SharedMemory {
        &self.data
    }
}

impl Default for Ram {
    fn default() -> Self {
        Ram::new()
    }
}
