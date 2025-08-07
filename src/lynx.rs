use crate::bus::{Bus, BusStatus};
use crate::cartridge::lnx_header::LNXRotation;
use crate::cartridge::Cartridge;
use crate::consts::{
    INTV_ADDR_A, M6502_RDY, MAPCTL_MIK_BIT, MAPCTL_ROM_BIT, MAPCTL_SUZ_BIT, MAPCTL_VEC_BIT,
    MIK_ADDR, MIK_ADDR_B, MMC_ADDR, MMC_ADDR_B, NMIV_ADDR, ROM_ADDR, ROM_ADDR_B, SUZ_ADDR,
    SUZ_ADDR_B, TIM0BKUP,
};
#[cfg(not(feature = "comlynx_shared_memory"))]
use crate::mikey::uart::comlynx_cable_mutex::ComlynxCable;
#[cfg(feature = "comlynx_shared_memory")]
use crate::mikey::uart::comlynx_cable_shared_memory::ComlynxCable;
use crate::mikey::{
    video::{LYNX_SCREEN_HEIGHT, LYNX_SCREEN_WIDTH},
    Mikey,
};
use crate::ram::{Ram, RAM_MAX};
use crate::rom::Rom;
use crate::shared_memory::SharedMemory;
use crate::suzy::{
    registers::{joystick_swap, Joystick, Switches},
    Suzy,
};
use crate::vectors::Vectors;
use alloc::vec::Vec;
use log::trace;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Lynx {
    ram: Ram,
    rom: Rom,
    suzy: Suzy,
    mikey: Mikey,
    vectors: Vectors,
    cart: Cartridge,
    bus: Bus,
    last_ir_pc: u16,
    switches_cache: Switches,
    #[cfg(feature = "comlynx_external")]
    #[serde(skip)]
    comlynx_ext_tx: Option<kanal::Receiver<u8>>,
    #[cfg(feature = "comlynx_external")]
    #[serde(skip)]
    comlynx_ext_rx: Option<kanal::Sender<u8>>,
}

impl Lynx {
    #[must_use]
    pub fn new() -> Self {
        #[cfg(feature = "comlynx_external")]
        let (comlynx_ext_tx_tx, comlynx_ext_tx_rx) = kanal::unbounded::<u8>();
        #[cfg(feature = "comlynx_external")]
        let (comlynx_ext_rx_tx, comlynx_ext_rx_rx) = kanal::unbounded::<u8>();

        let mut slf = Self {
            vectors: Vectors::default(),
            ram: Ram::default(),
            rom: Rom::default(),
            suzy: Suzy::default(),
            mikey: Mikey::default(),
            cart: Cartridge::default(),
            bus: Bus::default(),
            last_ir_pc: 0,
            switches_cache: Switches::empty(),
            #[cfg(feature = "comlynx_external")]
            comlynx_ext_tx: Some(comlynx_ext_tx_rx),
            #[cfg(feature = "comlynx_external")]
            comlynx_ext_rx: Some(comlynx_ext_rx_tx),
        };

        #[cfg(feature = "comlynx_external")]
        slf.mikey_mut()
            .uart_mut()
            .set_external_comlynx(comlynx_ext_tx_tx, comlynx_ext_rx_rx);

        slf.initialize();
        slf
    }

    fn initialize(&mut self) {
        self.vectors.from_slice(&self.rom.as_slice()[0x1FA..]);
        self.ram.set_mmapctl(self.rom.as_slice()[0x1F9]);
        let reset_vec = self.vectors.reset();
        self.mikey.cpu_prefetch(reset_vec, &mut self.rom);
    }

    /// Loads a cartridge from a byte slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the cartridge data is invalid or cannot be parsed.
    pub fn load_cart_from_slice(&mut self, data: &[u8]) -> Result<(), &'static str> {
        trace!("Load cart");
        match Cartridge::from_slice(data) {
            Err(e) => Err(e),
            Ok(c) => {
                self.cart = c;
                Ok(())
            }
        }
    }

    /// Loads a ROM from a byte slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the ROM data is invalid or cannot be parsed.
    pub fn load_rom_from_slice(&mut self, data: &[u8]) -> Result<(), &'static str> {
        trace!("Load rom");
        match Rom::from_slice(data) {
            Err(e) => Err(e),
            Ok(r) => {
                self.rom = r;
                self.initialize();
                Ok(())
            }
        }
    }

    fn mmap_ram(&self, bit: u8) -> bool {
        self.ram.mmapctl() & bit != 0
    }

    pub fn poke(&mut self) {
        self.bus.set_status(BusStatus::Poke);
        self.mikey().cpu_pins().pin_on(M6502_RDY);
        trace!(
            "> Poke 0x{:04x} = 0x{:02x}, bus:{:?}",
            self.bus.addr(),
            self.bus.data(),
            self.bus
        );
        match self.bus.addr() {
            0..=SUZ_ADDR_B => self.ram.poke(&self.bus),
            SUZ_ADDR..=MIK_ADDR_B => {
                if self.mmap_ram(MAPCTL_SUZ_BIT) {
                    self.ram.poke(&self.bus);
                } else {
                    self.suzy.poke(&mut self.bus);
                }
            }
            MIK_ADDR..=ROM_ADDR_B => {
                if self.mmap_ram(MAPCTL_MIK_BIT) {
                    self.ram.poke(&self.bus);
                } else {
                    self.mikey.poke(&self.bus);
                }
            }
            MMC_ADDR | ROM_ADDR..=MMC_ADDR_B => self.ram.poke(&self.bus),
            NMIV_ADDR..=INTV_ADDR_A => {
                if self.mmap_ram(MAPCTL_VEC_BIT) {
                    self.ram.poke(&self.bus);
                } else {
                    self.vectors.poke(&self.bus);
                }
            }
        }
    }

    pub fn peek(&mut self) {
        self.bus.set_status(BusStatus::Peek);
        self.mikey().cpu_pins().pin_on(M6502_RDY);
        trace!(
            "> Peek 0x{:04x}, bus:{:?}",
            self.bus.addr(),
            self.bus
        );
        match self.bus.addr() {
            0..=SUZ_ADDR_B => self.ram.peek(&self.bus),
            SUZ_ADDR..=MIK_ADDR_B => {
                if self.mmap_ram(MAPCTL_SUZ_BIT) {
                    self.ram.peek(&self.bus);
                } else {
                    self.suzy.peek(&mut self.bus);
                }
            }
            MIK_ADDR..=ROM_ADDR_B => {
                if self.mmap_ram(MAPCTL_MIK_BIT) {
                    self.ram.peek(&self.bus);
                } else {
                    self.mikey.peek(&self.bus);
                }
            }
            ROM_ADDR..=MMC_ADDR_B => {
                if self.mmap_ram(MAPCTL_ROM_BIT) {
                    self.ram.peek(&self.bus);
                } else {
                    self.rom.peek(&self.bus);
                }
            }
            MMC_ADDR => self.ram.peek(&self.bus),
            NMIV_ADDR..=INTV_ADDR_A => {
                if self.mmap_ram(MAPCTL_VEC_BIT) {
                    self.ram.peek(&self.bus);
                } else {
                    self.vectors.peek(&self.bus);
                }
            }
        }
    }

    pub fn cpu_mem(&self, addr: u16) -> u8 {
        match addr {
            0..=SUZ_ADDR_B => self.ram.get(addr),
            SUZ_ADDR..=MIK_ADDR_B => {
                if self.mmap_ram(MAPCTL_SUZ_BIT) {
                    self.ram.get(addr)
                } else {
                    self.suzy.get(addr)
                }
            }
            MIK_ADDR..=ROM_ADDR_B => {
                if self.mmap_ram(MAPCTL_MIK_BIT) {
                    self.ram.get(addr)
                } else {
                    self.mikey.get(addr)
                }
            }
            ROM_ADDR..=MMC_ADDR_B => {
                if self.mmap_ram(MAPCTL_ROM_BIT) {
                    self.ram.get(addr)
                } else {
                    self.rom.get(addr)
                }
            }
            MMC_ADDR => self.ram.get(addr),
            NMIV_ADDR..=INTV_ADDR_A => {
                if self.mmap_ram(MAPCTL_VEC_BIT) {
                    self.ram.get(addr)
                } else {
                    self.vectors.get(addr)
                }
            }
        }
    }

    pub fn peek_ram(&mut self) {
        self.bus.set_status(BusStatus::Peek);
        self.mikey().cpu_pins().pin_on(M6502_RDY);
        trace!(
            "> Peek RAM 0x{:04x}, bus:{:?}",
            self.bus.addr(),
            self.bus
        );
        self.ram.peek(&self.bus);
    }

    pub fn step_instruction(&mut self) {
        loop {
            self.tick();
            let pc = self.mikey.cpu().last_ir_pc;
            if self.last_ir_pc != pc {
                self.last_ir_pc = pc;
                break;
            }
        }
    }

    pub fn tick(&mut self) {
        match self.bus.status() {
            BusStatus::PokeCore => self.poke(),
            BusStatus::PeekCore => self.peek(),
            BusStatus::PeekRAM => self.peek_ram(),
            _ => (),
        }

        self.ram.tick(&mut self.bus);
        self.rom.tick(&mut self.bus);
        self.vectors.tick(&mut self.bus);
        self.suzy.tick(&mut self.bus, &mut self.ram);
        let mut switches = self.switches_cache;
        self.cart
            .tick(&mut self.bus, self.mikey.registers_mut(), &mut switches);
        if self.switches_cache != switches {
            self.switches_cache = switches;
            self.suzy.set_switches(switches.bits());
        }
        self.mikey.tick(&mut self.bus, &mut self.cart, &self.ram);

        // #[cfg(debug_assertions)]
        // if self.last_ir_pc != self.mikey.cpu().last_ir_pc {
        //     self.last_ir_pc = self.mikey().cpu().last_ir_pc;
        //     let (dis, _) = disassemble(&self.ram, self.last_ir_pc);
        //     debug!("[{:04X}] -> {}", self.last_ir_pc,  dis);

        //     if self.mikey.cpu().last_ir_pc == 0x7040 {
        //         println!("A:{:02X} X:{:02X} Y:{:02X}", self.mikey().cpu().a(), self.mikey().cpu().x(), self.mikey().cpu().y());
        //         println!("X:{}", self.mikey().cpu().x());
        //     }
        // }
    }

    pub fn bus(&self) -> &Bus {
        &self.bus
    }

    pub fn ram(&self) -> &Ram {
        &self.ram
    }

    pub fn rom(&self) -> &Rom {
        &self.rom
    }

    pub fn suzy(&self) -> &Suzy {
        &self.suzy
    }

    pub fn mikey_mut(&mut self) -> &mut Mikey {
        &mut self.mikey
    }

    pub fn mikey(&self) -> &Mikey {
        &self.mikey
    }

    pub fn vectors(&self) -> &Vectors {
        &self.vectors
    }

    pub fn cart(&self) -> &Cartridge {
        &self.cart
    }

    pub fn set_joystick_u8(&mut self, joy: u8) {
        trace!("Joystick: {joy:08b}");

        let mut j = Joystick::from_bits(joy).unwrap();

        match self.rotation() {
            LNXRotation::_270 => {
                j = joystick_swap(j, Joystick::down, Joystick::right);
                j = joystick_swap(j, Joystick::up, Joystick::left);
                j = joystick_swap(j, Joystick::up, Joystick::down);
            }
            LNXRotation::_90 => {
                j = joystick_swap(j, Joystick::up, Joystick::left);
                j = joystick_swap(j, Joystick::down, Joystick::right);
            }
            LNXRotation::None => (),
        }

        if !self.left_handed() {
            j = joystick_swap(j, Joystick::up, Joystick::down);
            j = joystick_swap(j, Joystick::left, Joystick::right);
        }

        self.suzy.set_joystick(j.bits());
    }

    pub fn set_switches_u8(&mut self, sw: u8) {
        trace!("Switches: {sw:08b}");
        self.switches_cache = Switches::from_bits_truncate(sw);
        self.suzy.set_switches(sw);
    }

    pub fn joystick(&self) -> Joystick {
        self.suzy.joystick()
    }

    pub fn switches(&mut self) -> Switches {
        self.switches_cache = self.suzy.switches();
        self.switches_cache
    }

    pub fn screen_size(&self) -> (u32, u32) {
        match self.rotation() {
            LNXRotation::_270 | LNXRotation::_90 => (LYNX_SCREEN_HEIGHT, LYNX_SCREEN_WIDTH),
            LNXRotation::None => (LYNX_SCREEN_WIDTH, LYNX_SCREEN_HEIGHT),
        }
    }

    pub fn screen_rgba(&self) -> &Vec<u8> {
        self.mikey.video().rgba_screen()
    }

    pub fn rotation(&self) -> LNXRotation {
        self.cart.rotation()
    }

    pub fn left_handed(&self) -> bool {
        self.suzy.left_handed()
    }

    pub fn reset(&mut self) {
        self.bus = Bus::new();
        self.ram = Ram::new();
        self.vectors = Vectors::new();
        self.suzy = Suzy::new();
        self.mikey.reset();
        self.cart.reset();
        self.last_ir_pc = 0;
        self.initialize();
    }

    pub fn serialize_size(&self) -> usize {
        postcard::experimental::serialized_size(&self).unwrap()
    }

    pub fn audio_sample(&self) -> (i16, i16) {
        self.mikey.audio_sample()
    }

    pub fn redraw_requested(&mut self) -> bool {
        self.mikey.video_mut().redraw_requested()
    }

    pub fn display_refresh_rate(&self) -> f64 {
        1_000_000. / // to sec.
        (
            f64::from(self.mikey.timers().peek(TIM0BKUP) + 1) // us per line
            * 105. // 105 lines
        )
    }

    pub fn ram_size(&self) -> usize {
        RAM_MAX as usize
    }

    pub fn ram_data(&self) -> &SharedMemory {
        self.ram.data()
    }

    pub fn set_comlynx_cable_present(&mut self, present: bool) {
        self.mikey.set_comlynx_cable_present(present);
    }

    #[cfg(not(feature = "comlynx_shared_memory"))]
    pub fn set_comlynx_cable(&mut self, cable: &ComlynxCable) {
        self.mikey.set_comlynx_cable(cable);
        self.mikey.set_comlynx_cable_present(true);
    }

    pub fn comlynx_cable(&self) -> &ComlynxCable {
        self.mikey.comlynx_cable()
    }

    pub fn cart_mut(&mut self) -> &mut Cartridge {
        &mut self.cart
    }

    #[cfg(feature = "comlynx_external")]
    pub fn comlynx_ext_rx(&mut self, data: u8) {
        let _ = self.comlynx_ext_rx.as_ref().unwrap().send(data);
    }

    #[cfg(feature = "comlynx_external")]
    pub fn comlynx_ext_tx(&mut self) -> Option<u8> {
        self.comlynx_ext_tx
            .as_ref()
            .unwrap()
            .try_recv()
            .unwrap_or_default()
    }
}

impl Default for Lynx {
    fn default() -> Self {
        Self::new()
    }
}
