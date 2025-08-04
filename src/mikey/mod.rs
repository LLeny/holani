pub mod cpu;
pub mod registers;
pub mod timers;
pub mod uart;
pub mod video;

use crate::{alloc, bus, cartridge, consts, ram, rom};
use bus::{Bus, BusStatus};
use cartridge::Cartridge;
use consts::{
    ATTEN_A, ATTEN_B, ATTEN_C, ATTEN_D, AUD0VOL, AUD3MISC, BLUERED0, BLUEREDF, CPUSLEEP,
    CRYSTAL_TICK_LENGTH, DISPADR, DISPCTL, GREEN0, GREENF, INTRST, INTSET, INT_TIMER4, IODAT,
    IODAT_AUDIN, IODAT_CAD, IODAT_EXTPW, IODAT_NOEXP, IODAT_REST, IODIR, M6502_IRQ, M6502_NMI,
    M6502_RDY, M6502_RES, M6502_RW, M6502_SYNC, MIKEY_READ_TICKS, MIKEY_TIMER_READ_TICKS,
    MIKEY_TIMER_WRITE_TICKS, MIKEY_WRITE_TICKS, MIK_ADDR, MPAN, MSTEREO, PBKUP,
    REFRESH_AND_VIDEO_DMA_TICKS, SERCTL, SERDAT, SYSCTL1, SYSCTL1_CAS, SYSCTL1_POWER, TIM0BKUP,
    TIM4CTLA, VIDEO_DMA_BUFFER_LENGTH,
};
use cpu::{CPUPins, M6502Flags, M6502};
use log::{info, trace};
use ram::Ram;
use registers::{MikeyRegisters, SerCtlR, SerCtlW};
use rom::Rom;
use serde::{Deserialize, Serialize};
use timers::Timers;
#[cfg(not(feature = "comlynx_shared_memory"))]
use uart::{comlynx_cable_mutex::ComlynxCable, Uart};
#[cfg(feature = "comlynx_shared_memory")]
use uart::{comlynx_cable_shared_memory::ComlynxCable, Uart};
use video::Video;

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
    Cpu,
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
    video: Video,
    video_buffer_curr_addr: u16,
    disp_addr: u16,
    is_flipped: bool,
    bus_grant_bkup: Option<bool>,
    comlynx_cable_present: bool,
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
            video: Video::new(),
            video_buffer_curr_addr: 0,
            disp_addr: 0,
            is_flipped: false,
            bus_owner: MikeyBusOwner::Cpu,
            bus_grant_bkup: None,
            comlynx_cable_present: false,
        }
    }

    pub fn reset(&mut self) {
        self.cpu = M6502::new();
        self.cpu_pins = CPUPins::default();
        self.ticks = 0;
        self.timers = Timers::new();
        self.registers = MikeyRegisters::new();
        self.video = Video::new();
        self.video_buffer_curr_addr = 0;
        self.bus_owner = MikeyBusOwner::Cpu;
        self.uart.reset();
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

    pub fn tick(&mut self, bus: &mut Bus, cart: &mut Cartridge, dma_ram: &Ram) {
        self.ticks += 1;

        let (mut int, int4_triggered) = self.timers.tick_all(self.ticks as i64);

        if int4_triggered {
            // "The interrupt bit for timer 4 (UART baud rate) is driven by receiver or transmitter ready bit of the UART."
            if self.uart.tick(&mut self.registers) {
                int |= INT_TIMER4;
            }
        }

        if let Some(hsync_count) = self.timers.hsync() {
            trace!("hsync {}", self.ticks);
            self.video.hsync(hsync_count, &self.registers);
        }

        self.video.tick();

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

        match self.bus_owner {
            MikeyBusOwner::Cpu => {
                if self.cpu.flags().contains(M6502Flags::I) || self.registers.data(INTSET) == 0 {
                    self.cpu_pins.pin_off(M6502_IRQ);
                } else {
                    self.cpu_pins.pin_on(M6502_IRQ);
                }

                if bus.grant() { self.cpu_pins.pin_off(M6502_RDY) } else { self.cpu_pins.pin_on(M6502_RDY) }

                if self.registers.ir() != MikeyInstruction::None {
                    self.process_ir_step(bus, cart);
                }

                if bus.status() == BusStatus::None {
                    self.cpu_tick(bus);
                }
            }
            MikeyBusOwner::RefreshAndVideo => {
                let mut base_addr = i32::from(self.video_buffer_curr_addr);

                let addr_move_dir = if self.is_flipped { -1i32 } else { 1i32 };

                let mut b = vec![];
                for _ in 0..VIDEO_DMA_BUFFER_LENGTH {
                    b.push(if self.is_flipped { dma_ram.get(base_addr as u16).rotate_left(4) } else { dma_ram.get(base_addr as u16) });
                    base_addr += addr_move_dir;
                }

                self.video.push_pix_buffer(&b);

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
    
        if bus.status() == BusStatus::None {
            if let Some(screen_pixel_base) = self.video.required_bytes() {
                if bus.grant() {
                    if screen_pixel_base == 0 {
                        self.disp_addr = self.registers.disp_addr();
                        self.is_flipped = self.registers.is_flipped();
                    }
                    self.bus_owner = MikeyBusOwner::RefreshAndVideo;
                    self.registers.set_ticks_delay(REFRESH_AND_VIDEO_DMA_TICKS);
                    self.video_buffer_curr_addr = if self.is_flipped { self.disp_addr - screen_pixel_base } else { self.disp_addr + screen_pixel_base };
                    trace!(
                        "[{}] Need pixels @ 0x{:04X} (0x{:04X}+0x{:04X})",
                        self.ticks,
                        self.video_buffer_curr_addr,
                        self.registers.disp_addr(),
                        screen_pixel_base
                    );
                } else if !bus.request() {
                    bus.set_request(true);
                    trace!("Bus requested by Video");
                    self.bus_grant_bkup = Some(false);
                }
            }
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
                self.registers.set_data_r(u16::from(bus.data()));
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
                bus.set_grant(false);
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
            info!(
                "{:04X};{:04X};{:02X}",
                self.registers.cart_shift(),
                self.registers.cart_position(),
                b
            );
            cart.write_address_to_pins(
                self.registers.cart_shift(),
                self.registers.cart_position(),
                self.registers.audin(),
            );
        }
    }

    fn iodat_updated(&mut self, cart: &mut Cartridge) {
        let new = self.registers.data_r() as u8;
        info!("o{:02X};{:04X};{}", new, self.cpu().last_ir_pc, self.ticks);
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

    pub fn video_mut(&mut self) -> &mut Video {
        &mut self.video
    }

    #[must_use]
    pub fn video(&self) -> &Video {
        &self.video
    }

    #[cfg(not(feature = "comlynx_shared_memory"))]
    pub fn set_comlynx_cable(&mut self, cable: &ComlynxCable) {
        self.uart.set_cable(cable);
    }

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
