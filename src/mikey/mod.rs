pub mod cpu;
pub mod display;
pub mod registers;
pub mod timers;
pub mod uart;

use crate::{
    alloc, bus, cartridge,
    consts::{
        self, LYNX_SCREEN_HEIGHT, SINGLE_REFRESH_PERIOD, SINGLE_REFRESH_TICKS, VIDEO_DMA_PER_ROW,
        VIDEO_DMA_TICK_PERIOD,
    },
    ram, rom,
};
use alloc::vec::Vec;
use bitflags::bitflags;
use bus::{Bus, BusStatus};
use cartridge::Cartridge;
use consts::{
    ATTEN_A, ATTEN_B, ATTEN_C, ATTEN_D, AUD0VOL, AUD3MISC, BLUERED0, BLUEREDF, CPUSLEEP,
    CRYSTAL_TICK_LENGTH, DISPADR, DISPCTL, GREEN0, GREENF, INTRST, INTSET, INT_TIMER4, IODAT,
    IODAT_AUDIN, IODAT_CAD, IODAT_EXTPW, IODAT_NOEXP, IODAT_REST, IODIR, M6502_IRQ, M6502_NMI,
    M6502_RDY, M6502_RES, M6502_RW, M6502_SYNC, MIKEY_READ_TICKS, MIKEY_TIMER_READ_TICKS,
    MIKEY_TIMER_WRITE_TICKS, MIKEY_WRITE_TICKS, MIK_ADDR, MPAN, MSTEREO, PBKUP,
    REFRESH_AND_VIDEO_DMA_TICKS, SDONEACK, SERCTL, SERDAT, SYSCTL1, SYSCTL1_CAS, SYSCTL1_POWER,
    TIM0BKUP, TIM4CTLA, VIDEO_DMA_BUFFER_LENGTH,
};
use cpu::{CPUPins, M6502Flags, M6502};
use display::Display;
use log::trace;
use ram::Ram;
use registers::{DispCtl, MikeyRegisters, SerCtlR, SerCtlW};
use rom::Rom;
use serde::{Deserialize, Serialize};
use timers::Timers;
#[cfg(not(feature = "comlynx_shared_memory"))]
use uart::{comlynx_cable_mutex::ComlynxCable, Uart};
#[cfg(feature = "comlynx_shared_memory")]
use uart::{comlynx_cable_shared_memory::ComlynxCable, Uart};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MikeyInstruction {
    None,
    TimersPeek,
    TimersPoke,
    Poke,
    Peek,
    PeekNothing,
    PokeSysctl1,
    PeekIntRst,
    PeekIodat,
    PokeIodat,
    PokeOk,
    PeekIncCartRipple,
    PokeIncCartRipple,
    CpuSleep,
    PeekDispCtl,
    PokeDispCtl,
    PeekSerCtl,
    PokeSerCtl,
    PeekSerDat,
    PokeSerDat,
    PokePbkup,
    PokeChangeAttenuation,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MikeyBusOwner {
    RefreshAndVideo,
    Refresh,
    Cpu,
}

#[derive(Serialize, Deserialize)]
pub struct VideoDMA {
    pub ptr: usize,
    pub delay: isize,
    pub row_dma_count: usize,
    pub flipped_display: isize,
}

impl Default for VideoDMA {
    fn default() -> Self {
        Self {
            ptr: 0,
            delay: 0,
            row_dma_count: 0,
            flipped_display: 1,
        }
    }
}

impl VideoDMA {
    pub fn reset(&mut self) {
        self.ptr = 0;
        self.delay = 0;
        self.row_dma_count = 0;
        self.flipped_display = 1;
    }

    pub fn hsync(&mut self) {
        self.row_dma_count = 0;
        self.delay = 0;
    }

    pub fn vsync(&mut self, regs: &MikeyRegisters) {
        self.ptr = usize::from(regs.disp_addr());
        self.flipped_display = if regs.is_flipped() { -1 } else { 1 };
    }

    pub fn request(&mut self) {
        self.delay = VIDEO_DMA_TICK_PERIOD;
    }

    pub fn tick(&mut self) {
        self.delay -= 1;
    }

    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.row_dma_count < VIDEO_DMA_PER_ROW && self.delay <= 0
    }

    #[must_use]
    pub fn is_flipped(&self) -> bool {
        self.flipped_display < 0
    }

    #[must_use]
    pub fn get_buffer_offset(&self, offset: usize) -> u16 {
        (offset as isize * self.flipped_display + self.ptr as isize) as u16
    }

    pub fn consume(&mut self) {
        self.row_dma_count += 1;
        self.ptr = if self.is_flipped() {
            self.ptr.wrapping_sub(VIDEO_DMA_BUFFER_LENGTH)
        } else {
            self.ptr.wrapping_add(VIDEO_DMA_BUFFER_LENGTH)
        };
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug, Serialize, Deserialize)]
    struct SuzyStatus: u8 {
        const busy         = 0b0000_0001;
        const bus_enabled  = 0b0000_0010;
        const done_pending = 0b0000_0100;
    }
}

#[derive(Serialize, Deserialize)]
pub struct Mikey {
    cpu: M6502,
    cpu_pins: CPUPins,
    timers: Timers,
    uart: Uart,
    ticks: u64,
    registers: MikeyRegisters,
    bus_owner: MikeyBusOwner,
    bus_request: Option<MikeyBusOwner>,
    display: Display,
    video_dma: VideoDMA,
    refresh_delay: isize,
    bus_grant_bkup: Option<bool>,
    comlynx_cable_present: bool,
    suzy_status: SuzyStatus,
}

impl Mikey {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cpu: M6502::new(),
            cpu_pins: CPUPins::default(),
            uart: Uart::new(),
            ticks: 0,
            timers: Timers::new(),
            registers: MikeyRegisters::new(),
            display: Display::new(),
            video_dma: VideoDMA::default(),
            refresh_delay: SINGLE_REFRESH_PERIOD,
            bus_owner: MikeyBusOwner::Cpu,
            bus_request: None,
            bus_grant_bkup: None,
            comlynx_cable_present: false,
            suzy_status: SuzyStatus::bus_enabled,
        }
    }

    pub fn reset(&mut self) {
        self.cpu = M6502::new();
        self.cpu_pins = CPUPins::default();
        self.ticks = 0;
        self.timers = Timers::new();
        self.registers = MikeyRegisters::new();
        self.display = Display::new();
        self.video_dma.reset();
        self.bus_owner = MikeyBusOwner::Cpu;
        self.uart.reset();
        self.suzy_status = SuzyStatus::bus_enabled;
    }

    pub fn set_suzy_state(&mut self, busy: bool, bus_enabled: bool) {
        self.suzy_status.set(SuzyStatus::busy, busy);
        self.suzy_status.set(SuzyStatus::bus_enabled, bus_enabled);
    }

    pub fn on_suzy_done(&mut self) {
        self.suzy_status.insert(SuzyStatus::done_pending);
    }

    pub fn cpu_prefetch(&mut self, pc: u16, rom: &mut Rom) {
        trace!("- CPU prefetch 0x{pc:04x}");
        self.cpu_pins.set(M6502_SYNC);
        self.cpu_pins.sa(pc);
        self.cpu_pins.sd(rom.get(pc));
        self.cpu.set_pc(pc);
        trace!("- CPU:{:?}", self.cpu);
    }

    pub fn cpu_tick(&mut self, bus: &mut Bus) {
        self.cpu_pins = self.cpu.tick(self.cpu_pins);
        let addr = self.cpu_pins.ga();

        if self.cpu_pins.is_set(M6502_RW) {
            bus.set_addr(addr);
            bus.set_status(BusStatus::PeekCore);
        } else {
            bus.set_addr(addr);
            bus.set_data(self.cpu_pins.gd());
            bus.set_status(BusStatus::PokeCore);
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn tick(&mut self, bus: &mut Bus, cart: &mut Cartridge, dma_ram: &Ram) {
        self.ticks += 1;

        let (mut int, int4_done) = self.timers.tick_all();

        if int4_done {
            // "The interrupt bit for timer 4 (UART baud rate) is driven by receiver or transmitter ready bit of the UART."
            if self.uart.tick(&mut self.registers) {
                int |= INT_TIMER4;
            }
        }

        if self.registers.serctl_w_is_flag_set(SerCtlW::tx_int_en)
            && self.registers.serctl_r_is_flag_set(SerCtlR::tx_rdy)
        {
            int |= INT_TIMER4;
        }

        if let Some(hsync_count) = self.timers.hsync() {
            trace!("hsync {}", self.ticks);
            self.display.hsync(hsync_count as usize);
            self.video_dma.hsync();
            if hsync_count == LYNX_SCREEN_HEIGHT as u8 {
                self.video_dma.vsync(&self.registers);
            }
        }

        self.display.tick(&self.registers);
        self.video_dma.tick();

        if self.display.is_dma_active()
            && self.registers.dispctl_is_flag_set(DispCtl::dma_enable)
        {
            if self.video_dma.is_ready() {
                self.bus_request = Some(MikeyBusOwner::RefreshAndVideo);
                self.video_dma.request();
            }
        } else {
            self.refresh_delay -= 1;
            if self.refresh_delay <= 0 && self.cpu_pins.is_set(M6502_SYNC) {
                self.bus_request = Some(MikeyBusOwner::Refresh);
                self.refresh_delay = SINGLE_REFRESH_PERIOD;
            }
        }

        if int != 0 {
            int |= self.registers.data(INTSET);
            self.registers.set_data(INTSET, int);
            trace!("INTSET -> {int:02X}");
            if !bus.grant() {
                // wake up the cpu
                bus.set_request(true);
            }
        }

        if self.registers.ticks_delay() > 0 {
            self.registers.dec_ticks_delay();
            return;
        }

        if self.bus_owner == MikeyBusOwner::Cpu {
            self.handle_mikey_bus_owner(bus);
        }

        if bus.status() == BusStatus::None {
            match self.bus_request {
                Some(MikeyBusOwner::RefreshAndVideo) => {
                    if bus.grant() {
                        self.bus_request = None;
                        bus.set_requester(Some(MikeyBusOwner::RefreshAndVideo));
                    } else if !bus.request() {
                        bus.set_request(true);
                        trace!("Bus requested by Video");
                        self.bus_grant_bkup = Some(false);
                    }
                }
                Some(MikeyBusOwner::Refresh) => {
                    if bus.grant() {
                        self.bus_request = None;
                        bus.set_requester(Some(MikeyBusOwner::Refresh));
                    } else if !bus.request() {
                        bus.set_request(true);
                        trace!("Bus requested by refresh");
                        self.bus_grant_bkup = Some(false);
                    }
                }
                _ => (),
            }
        }

        match bus.requester() {
            Some(MikeyBusOwner::Refresh) => {
                self.bus_owner = MikeyBusOwner::Refresh;
                self.registers.set_ticks_delay(SINGLE_REFRESH_TICKS);
                bus.set_requester(None);
                trace!("[{}] Refresh taking bus.", self.ticks);
            }
            Some(MikeyBusOwner::RefreshAndVideo) => {
                self.bus_owner = MikeyBusOwner::RefreshAndVideo;
                self.registers.set_ticks_delay(REFRESH_AND_VIDEO_DMA_TICKS);
                bus.set_requester(None);
                trace!("[{}] Video/Refresh taking bus.", self.ticks);
                return;
            }
            _ => (),
        }

        match self.bus_owner {
            MikeyBusOwner::Cpu => {
                if self.cpu.flags().contains(M6502Flags::I) || self.registers.data(INTSET) == 0 {
                    self.cpu_pins.pin_off(M6502_IRQ);
                } else {
                    self.cpu_pins.pin_on(M6502_IRQ);
                }

                if bus.grant() {
                    self.cpu_pins.pin_off(M6502_RDY);
                } else {
                    self.cpu_pins.pin_on(M6502_RDY);
                }

                if self.registers.ir() != MikeyInstruction::None {
                    self.process_ir_step(bus, cart);
                }

                if bus.status() == BusStatus::None {
                    self.cpu_tick(bus);
                }
            }
            MikeyBusOwner::Refresh => {
                self.bus_owner = MikeyBusOwner::Cpu;
                bus.set_status(BusStatus::None);
                if let Some(grant) = self.bus_grant_bkup.take() {
                    trace!("Refresh set bus grant: {grant}");
                    bus.set_grant(grant);
                }
                trace!("[{}] Refresh done.", self.ticks);
            }
            MikeyBusOwner::RefreshAndVideo => {
                let mut pixs: Vec<u8> = (0..VIDEO_DMA_BUFFER_LENGTH)
                    .map(|i| dma_ram.get(self.video_dma.get_buffer_offset(i)))
                    .collect();

                if self.video_dma.is_flipped() {
                    pixs = pixs.iter().map(|b| b.rotate_right(4)).collect();
                }

                self.display.push_video_dma_burst(&pixs);
                self.video_dma.consume();

                self.bus_owner = MikeyBusOwner::Cpu;
                bus.set_status(BusStatus::None);
                if let Some(grant) = self.bus_grant_bkup.take() {
                    trace!("Refresh/Video set bus grant: {grant}");
                    bus.set_grant(grant);
                }
                trace!("[{}] Refresh/Video done.", self.ticks);
            }
        }
    }

    fn handle_mikey_bus_owner(&mut self, bus: &mut Bus) {
        match bus.status() {
            BusStatus::PeekDone => {
                self.cpu_pins.sd(bus.data());
                bus.set_status(BusStatus::None);
                self.cpu_pins.pin_off(M6502_RDY);
                trace!(
                    "[{}] < Peek 0x{:02x}, bus:{:?}",
                    self.ticks,
                    bus.data(),
                    bus
                );
            }
            BusStatus::PokeDone => {
                bus.set_status(BusStatus::None);
                self.cpu_pins.pin_off(M6502_RDY);
                trace!("[{}] < Poke, bus:{:?}", self.ticks, bus);
            }
            BusStatus::PokeIncCartRipple => {
                self.registers.set_ir(MikeyInstruction::PokeIncCartRipple);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
            BusStatus::PeekIncCartRipple => {
                self.registers.set_ir(MikeyInstruction::PeekIncCartRipple);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
            _ => (),
        }
    }

    #[must_use]
    pub fn get(&self, addr: u16) -> u8 {
        self.registers.data(addr)
    }

    pub fn peek(&mut self, bus: &Bus) {
        assert!(bus.addr() >= MIK_ADDR && bus.addr() <= MIK_ADDR | 0xff);
        let addr = bus.addr();
        match addr {
            TIM0BKUP..=AUD3MISC => {
                self.registers.set_addr_r(addr);
                self.registers.set_ir(MikeyInstruction::TimersPeek);
                self.registers.set_ticks_delay(MIKEY_TIMER_READ_TICKS);
            }
            IODAT => {
                self.registers.set_ir(MikeyInstruction::PeekIodat);
                self.registers.set_ticks_delay(MIKEY_READ_TICKS);
            }
            INTRST => {
                self.registers.set_addr_r(addr);
                self.registers.set_ir(MikeyInstruction::PeekIntRst);
                self.registers.set_ticks_delay(MIKEY_READ_TICKS);
            }
            SYSCTL1 => {
                self.registers.set_addr_r(addr);
                self.registers.set_ir(MikeyInstruction::PeekNothing);
                self.registers.set_ticks_delay(MIKEY_READ_TICKS);
            }
            DISPCTL => {
                self.registers.set_ir(MikeyInstruction::PeekDispCtl);
                self.registers.set_ticks_delay(MIKEY_READ_TICKS);
            }
            SERCTL => {
                self.registers.set_ir(MikeyInstruction::PeekSerCtl);
                self.registers.set_ticks_delay(MIKEY_READ_TICKS);
            }
            SERDAT => {
                self.registers.set_ir(MikeyInstruction::PeekSerDat);
                self.registers.set_ticks_delay(MIKEY_READ_TICKS);
            }
            _ => {
                self.registers.set_addr_r(addr);
                self.registers.set_ir(MikeyInstruction::Peek);
                self.registers.set_ticks_delay(MIKEY_READ_TICKS);
            }
        }
        trace!("[{}] > Peek 0x{:04x}", self.ticks, bus.addr());
    }

    pub fn poke(&mut self, bus: &Bus) {
        assert!(bus.addr() >= MIK_ADDR && bus.addr() <= (MIK_ADDR | 0xff));
        let addr = bus.addr();
        match addr {
            TIM0BKUP..=AUD3MISC => {
                self.registers.set_addr_r(addr);
                self.registers.set_data_r(u16::from(bus.data()));
                self.registers.set_ir(MikeyInstruction::TimersPoke);
                self.registers.set_ticks_delay(MIKEY_TIMER_WRITE_TICKS);
            }
            INTSET => {
                self.registers.set_addr_r(addr);
                self.registers.set_data_r(u16::from(bus.data()));
                self.registers.set_ir(MikeyInstruction::Poke);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
            INTRST => {
                self.set_intrst(bus.data());
                self.registers.set_ir(MikeyInstruction::PokeOk);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
            IODAT => {
                self.registers.set_addr_r(addr);
                self.registers.set_data_r(u16::from(bus.data()));
                self.registers.set_ir(MikeyInstruction::PokeIodat);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
            SYSCTL1 => {
                self.registers.set_addr_r(addr);
                self.registers.set_data_r(u16::from(bus.data()));
                self.registers.set_ir(MikeyInstruction::PokeSysctl1);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
            SDONEACK => {
                self.suzy_status.remove(SuzyStatus::done_pending);
                self.registers.set_ir(MikeyInstruction::PokeOk);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
            CPUSLEEP => {
                self.registers.set_addr_r(addr);
                self.registers.set_data_r(u16::from(bus.data()));
                self.registers.set_ir(MikeyInstruction::CpuSleep);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
            DISPCTL => {
                self.registers.set_data_r(u16::from(bus.data()));
                self.registers.set_ir(MikeyInstruction::PokeDispCtl);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
            SERCTL => {
                self.registers.set_data_r(u16::from(bus.data()));
                self.registers.set_ir(MikeyInstruction::PokeSerCtl);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
            SERDAT => {
                self.registers.set_data_r(u16::from(bus.data()));
                self.registers.set_ir(MikeyInstruction::PokeSerDat);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
            PBKUP => {
                let data = bus.data();
                self.display.set_pbkup(data);
                self.registers.set_data_r(u16::from(data));
                self.registers.set_ir(MikeyInstruction::PokePbkup);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
            ATTEN_A..=MSTEREO => {
                self.registers.set_addr_r(addr);
                self.registers.set_data_r(u16::from(bus.data()));
                self.registers
                    .set_ir(MikeyInstruction::PokeChangeAttenuation);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
            _ => {
                self.registers.set_addr_r(addr);
                self.registers.set_data_r(u16::from(bus.data()));
                self.registers.set_ir(MikeyInstruction::Poke);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
        }
        trace!(
            "[{}] > Poke 0x{:04x} 0x{:02x}",
            self.ticks,
            bus.addr(),
            bus.data()
        );
    }

    #[allow(clippy::too_many_lines)]
    fn process_ir_step(&mut self, bus: &mut Bus, cart: &mut Cartridge) {
        match self.registers.ir() {
            MikeyInstruction::Peek => {
                bus.set_data(self.registers.data(self.registers.addr_r()));
                bus.set_status(BusStatus::PeekDone);
                self.registers.reset_ir();
            }
            MikeyInstruction::Poke => {
                self.registers
                    .set_data(self.registers.addr_r(), self.registers.data_r() as u8);
                bus.set_status(BusStatus::PokeDone);
                self.registers.reset_ir();
            }
            MikeyInstruction::PokeChangeAttenuation => {
                self.registers
                    .set_data(self.registers.addr_r(), self.registers.data_r() as u8);
                self.registers.update_attenuations();
                bus.set_status(BusStatus::PokeDone);
                self.registers.reset_ir();
            }
            MikeyInstruction::TimersPeek => {
                bus.set_data(self.timers.peek(self.registers.addr_r()));
                bus.set_status(BusStatus::PeekDone);
                self.registers.reset_ir();
            }
            MikeyInstruction::TimersPoke => {
                self.timers
                    .poke(self.registers.addr_r(), self.registers.data_r() as u8);
                bus.set_status(BusStatus::PokeDone);
                self.registers.reset_ir();
            }
            MikeyInstruction::PeekIntRst => {
                bus.set_data(self.registers.data(INTSET));
                bus.set_status(BusStatus::PeekDone);
                self.registers.reset_ir();
            }
            MikeyInstruction::PeekNothing => {
                bus.set_data(0xff);
                bus.set_status(BusStatus::PeekDone);
                self.registers.reset_ir();
            }
            MikeyInstruction::PokeSysctl1 => {
                self.sysctl1_updated(bus, cart);
                self.registers.reset_ir();
                bus.set_status(BusStatus::PokeDone);
            }
            MikeyInstruction::PokeIodat => {
                self.iodat_updated(cart);
                self.registers.reset_ir();
                bus.set_status(BusStatus::PokeDone);
            }
            MikeyInstruction::PeekIodat => {
                let iodat = self.registers.data(IODAT);
                let iodir = self.registers.data(IODIR);
                let mut v: u8 = 0;
                if cart.audin() {
                    v |= IODAT_AUDIN;
                } else {
                    v &= !IODAT_AUDIN;
                }
                if iodir & IODAT_EXTPW != 0 {
                    v |= iodat & IODAT_EXTPW;
                } else {
                    v |= IODAT_EXTPW;
                }
                if iodir & IODAT_CAD != 0 {
                    v |= iodat & IODAT_CAD;
                }
                if iodir & IODAT_NOEXP != 0 {
                    v |= iodat & IODAT_NOEXP;
                } else if !self.comlynx_cable_present {
                    v |= IODAT_NOEXP;
                }
                if iodir & IODAT_REST != 0 {
                    v |= iodat & IODAT_REST;
                }
                bus.set_data(v);
                bus.set_status(BusStatus::PeekDone);
                self.registers.reset_ir();
            }
            MikeyInstruction::PokeOk => {
                self.registers.reset_ir();
                bus.set_status(BusStatus::PokeDone);
            }
            MikeyInstruction::PokeIncCartRipple => {
                self.registers.inc_cart_position();
                cart.write_address_to_pins(
                    self.registers.cart_shift(),
                    self.registers.cart_position(),
                    self.registers.audin(),
                );
                self.registers.reset_ir();
                bus.set_status(BusStatus::PokeDone);
            }
            MikeyInstruction::PeekIncCartRipple => {
                self.registers.inc_cart_position();
                cart.write_address_to_pins(
                    self.registers.cart_shift(),
                    self.registers.cart_position(),
                    self.registers.audin(),
                );
                self.registers.reset_ir();
                bus.set_status(BusStatus::PeekDone);
            }
            MikeyInstruction::CpuSleep => {
                self.registers.reset_ir();
                if self.registers.data_r() == 0
                    && self
                        .suzy_status
                        .contains(SuzyStatus::busy | SuzyStatus::bus_enabled)
                    && !self.suzy_status.contains(SuzyStatus::done_pending)
                    && self.registers.data(INTSET) == 0
                {
                    bus.set_grant(false);
                }
                bus.set_status(BusStatus::PokeDone);
            }
            MikeyInstruction::PeekDispCtl => {
                bus.set_data(self.registers.dispctl());
                self.registers.reset_ir();
                trace!("< Peek");
                bus.set_status(BusStatus::PeekDone);
            }
            MikeyInstruction::PokeDispCtl => {
                self.registers.set_dispctl(self.registers.data_r() as u8);
                self.registers.reset_ir();
                trace!("< Poke");
                bus.set_status(BusStatus::PokeDone);
            }
            MikeyInstruction::PeekSerCtl => {
                bus.set_data(self.registers.serctl());
                self.registers.reset_ir();
                trace!("< Peek");
                bus.set_status(BusStatus::PeekDone);
            }
            MikeyInstruction::PokeSerCtl => {
                self.registers
                    .set_serctl(&mut self.uart, self.registers.data_r() as u8);
                self.registers.reset_ir();
                trace!("< Poke");
                bus.set_status(BusStatus::PokeDone);
            }
            MikeyInstruction::PeekSerDat => {
                bus.set_data(self.uart.get_data(&mut self.registers));
                self.registers.reset_ir();
                trace!("< Peek");
                bus.set_status(BusStatus::PeekDone);
            }
            MikeyInstruction::PokeSerDat => {
                let data = self.registers.data_r() as u8;
                self.uart
                    .set_transmit_holding_buffer(&mut self.registers, data);
                self.registers.reset_ir();
                trace!("< Poke");
                bus.set_status(BusStatus::PokeDone);
            }
            MikeyInstruction::PokePbkup => {
                self.registers
                    .set_data(PBKUP, self.registers.data_r() as u8);
                self.registers.reset_ir();
                bus.set_status(BusStatus::PokeDone);
            }
            MikeyInstruction::None => (),
        }
    }

    fn set_intrst(&mut self, v: u8) {
        trace!("INTRST -> {v:02X}");
        self.registers.set_data(INTRST, v);
        self.registers
            .set_data(INTSET, self.registers.data(INTSET) & !v);
    }

    fn sysctl1_updated(&mut self, _bus: &mut Bus, cart: &mut Cartridge) {
        let prev = self.registers.data(SYSCTL1);
        let new = self.registers.data_r() as u8;
        self.registers.set_data(SYSCTL1, new);

        if prev & SYSCTL1_POWER == 0 && new & SYSCTL1_POWER != 0 {
            self.registers.reset_cart_position();
            self.registers.reset_cart_shift();
        }

        let b = u8::from(self.registers.data(IODAT) & IODAT_CAD != 0);
        if b > 0 {
            self.registers.reset_cart_position();
        }

        if new & SYSCTL1_POWER != 0 && prev & SYSCTL1_CAS == 0 && new & SYSCTL1_CAS != 0 {
            self.registers.shift_cart_shift(b);
            cart.write_address_to_pins(
                self.registers.cart_shift(),
                self.registers.cart_position(),
                self.registers.audin(),
            );
        }
    }

    fn iodat_updated(&mut self, cart: &mut Cartridge) {
        let new = self.registers.data_r() as u8;
        self.registers.set_data(IODAT, new);

        let b = u16::from(self.registers.data(IODAT) & IODAT_AUDIN != 0);
        self.registers.set_audin(b);
        cart.write_address_to_pins(
            self.registers.cart_shift(),
            self.registers.cart_position(),
            self.registers.audin(),
        );
    }

    #[must_use]
    pub fn cpu_pins(&self) -> CPUPins {
        self.cpu_pins
    }

    #[must_use]
    pub fn cpu(&self) -> &M6502 {
        &self.cpu
    }

    #[must_use]
    pub fn registers(&self) -> &MikeyRegisters {
        &self.registers
    }

    pub fn registers_mut(&mut self) -> &mut MikeyRegisters {
        &mut self.registers
    }

    #[must_use]
    pub fn timers(&self) -> &Timers {
        &self.timers
    }

    #[must_use]
    pub fn audio_sample(&self) -> (i16, i16) {
        let audio0 = f32::from(self.timers.audio_out(0));
        let audio1 = f32::from(self.timers.audio_out(1));
        let audio2 = f32::from(self.timers.audio_out(2));
        let audio3 = f32::from(self.timers.audio_out(3));

        let left = ((audio0 * self.registers.attenuation_left(0)
            + audio1 * self.registers.attenuation_left(1)
            + audio2 * self.registers.attenuation_left(2)
            + audio3 * self.registers.attenuation_left(3)) as i32)
            << 5;

        let right = ((audio0 * self.registers.attenuation_right(0)
            + audio1 * self.registers.attenuation_right(1)
            + audio2 * self.registers.attenuation_right(2)
            + audio3 * self.registers.attenuation_right(3)) as i32)
            << 5;

        (left as i16, right as i16)
    }

    pub fn video_mut(&mut self) -> &mut Display {
        &mut self.display
    }

    #[must_use]
    pub fn video(&self) -> &Display {
        &self.display
    }

    #[cfg(not(feature = "comlynx_shared_memory"))]
    pub fn set_comlynx_cable(&mut self, cable: &ComlynxCable) {
        self.uart.set_cable(cable);
    }

    #[allow(dead_code)]
    pub(crate) fn uart_mut(&mut self) -> &mut Uart {
        &mut self.uart
    }

    pub(crate) fn comlynx_cable(&self) -> &ComlynxCable {
        self.uart.cable()
    }

    #[must_use]
    pub fn bus_owner(&self) -> MikeyBusOwner {
        self.bus_owner
    }

    pub fn set_comlynx_cable_present(&mut self, comlynx_cable_present: bool) {
        self.comlynx_cable_present = comlynx_cable_present;
    }
}

impl Default for Mikey {
    fn default() -> Self {
        Mikey::new()
    }
}
