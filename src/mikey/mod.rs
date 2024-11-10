pub mod cpu;
pub mod registers;
pub mod timers;
pub mod uart;
pub mod video;

use crate::*;
use cpu::*;
use log::trace;
use timers::*;
use registers::*;
#[cfg(not(feature = "comlynx_shared_memory"))]
use uart::{comlynx_cable_mutex::ComlynxCable, Uart};
#[cfg(feature = "comlynx_shared_memory")]
use uart::{comlynx_cable_shared_memory::ComlynxCable, Uart};
use video::*;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
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
    Video,
    Refresh,
    Cpu,
}

#[derive(Serialize, Deserialize)]
pub struct Mikey {
    cpu: M6502,
    #[serde(skip)]
    cpu_stepper: M6502Stepper,
    cpu_pins: CPUPins,
    timers: Timers,
    uart: Uart,
    ticks: u64,    
    registers: MikeyRegisters,
    bus_owner: MikeyBusOwner,
    video: Video,
    video_buffer_buffer: Vec<u8>,
    video_buffer_curr_addr: u16,
    bus_grant_bkp: Option<bool>,
}

impl Mikey {
    pub fn new() -> Self {
        Self {
            cpu: M6502::new(),
            cpu_stepper: M6502Stepper::default(),
            cpu_pins: CPUPins::default(),
            uart: Uart::new(),
            ticks: 0,
            timers: Timers::new(),
            registers: MikeyRegisters::new(),
            video: Video::new(),
            video_buffer_buffer: vec![],
            video_buffer_curr_addr: 0,
            bus_grant_bkp: None,
            bus_owner: MikeyBusOwner::Cpu,
        }
    }

    pub fn reset(&mut self) {
        self.cpu = M6502::new();
        self.cpu_stepper = M6502Stepper::default();
        self.cpu_pins = CPUPins::default();
        self.ticks = 0;
        self.timers = Timers::new();
        self.registers = MikeyRegisters::new();
        self.video = Video::new();
        self.video_buffer_buffer.clear();
        self.video_buffer_curr_addr = 0;
        self.bus_grant_bkp = None;
        self.bus_owner = MikeyBusOwner::Cpu;
        self.uart.reset();
    }

    pub fn cpu_prefetch(&mut self, pc: u16, rom: &mut Rom) {
        trace!("- CPU prefetch 0x{:04x}", pc);
        self.cpu_pins.set(M6502_SYNC);
        self.cpu_pins.sa(pc);
        self.cpu_pins.sd(rom.get(pc));
        self.cpu.set_pc(pc);
        trace!("- CPU:{:?}", self.cpu);
    }

    pub fn cpu_tick(&mut self, bus: &mut Bus) {
        self.cpu_pins = self.cpu_stepper.tick(&mut self.cpu, self.cpu_pins);
        let addr = self.cpu_pins.ga();

        if self.cpu_pins.is_set(M6502_RW) {
            bus.set_addr(addr);
            bus.set_data(1);//always force page mode if possible. // if self.cpu_pins.is_set(M6502_SYNC) {1} else {0});
            bus.set_status(BusStatus::PeekCore);
        } else {
            bus.set_addr(addr);
            bus.set_data(self.cpu_pins.gd());
            bus.set_status(BusStatus::PokeCore);
        }
    }

    pub fn tick(&mut self, bus: &mut Bus, cart: &mut Cartridge) {
        self.ticks += 1;

        let (mut int, int4_triggered) = self.timers.tick_all(self.ticks);

        if int4_triggered { // "The interrupt bit for timer 4 (UART baud rate) is driven by receiver or transmitter ready bit of the UART."
            if self.uart.tick(&mut self.registers) {
                int |= INT_TIMER4;
            }
        }
        
        let vsync = self.timers.vsync();
        let hsync = self.timers.hsync();

        if vsync {
            self.video.vsync();
        } else if hsync {
            self.video.hsync();
        } 

        self.video.tick(&self.registers);

        if int != 0 {
            int |= self.registers.data(INTSET);
            self.registers.set_data(INTSET, int);
            trace!("INTSET -> {:02X}", int);        
            if !bus.grant() { // wake up the cpu
                bus.set_request(true);
            }
        }

        if self.registers.ticks_delay() > 0 {
            self.registers.dec_ticks_delay();
            return;
        }
        
        if self.bus_owner  == MikeyBusOwner::Cpu {           
            match bus.status() {
                BusStatus::PeekDone => {
                    self.cpu_pins.sd(bus.data());
                    bus.set_status(BusStatus::None);
                    self.cpu_pins.pin_off(M6502_RDY);
                    trace!("[{}] < Peek 0x{:02x}, bus:{:?}", self.ticks, bus.data(), bus);
                }
                BusStatus::PokeDone => {
                    bus.set_status(BusStatus::None);
                    self.cpu_pins.pin_off(M6502_RDY);
                    trace!("[{}] < Poke, bus:{:?}", self.ticks, bus);
                }
                BusStatus::PokeIncCartRipple => {
                    self.registers.set_ir(MikeyInstruction::PokeIncCartRipple);
                    self.registers.set_ticks_delay(MIKEY_READ_TICKS + MIKEY_WRITE_TICKS);
                }
                BusStatus::PeekIncCartRipple => {
                    self.registers.set_ir(MikeyInstruction::PeekIncCartRipple);
                    self.registers.set_ticks_delay(MIKEY_READ_TICKS + MIKEY_WRITE_TICKS);
                }
                _ => ()
            }

            if bus.status() == BusStatus::None {
                if let Some(screen_pixel_base) = self.video.required_bytes() {
                    if !bus.grant() {
                        if !bus.request() {
                            self.bus_grant_bkp = Some(false);
                            bus.set_request(true);
                            trace!("Bus grant backup false");
                        }
                    } else {
                        if self.bus_grant_bkp.is_none() {
                            self.bus_grant_bkp = Some(true);
                            trace!("Bus grant backup true");
                        }
                        self.bus_owner = MikeyBusOwner::Refresh;
                        self.registers.set_ticks_delay(RAM_REFRESH_TICKS as u16);
                        self.video_buffer_curr_addr = match self.registers.is_flipped() {
                            false => self.registers.disp_addr() + screen_pixel_base,
                            true => self.registers.disp_addr() - screen_pixel_base
                        };
                        trace!("[{}] Need pixels @ 0x{:04X} (0x{:04X}+0x{:04X})", self.ticks, self.video_buffer_curr_addr, self.registers.disp_addr(), screen_pixel_base);
                    }
                }
            }    
        } 

        match self.bus_owner {            
            MikeyBusOwner::Cpu => {
                if self.cpu.flags().contains(M6502Flags::I) || self.registers.data(INTSET) == 0 {
                    self.cpu_pins.pin_off(M6502_IRQ);
                } else {
                    self.cpu_pins.pin_on(M6502_IRQ);
                }

                match bus.grant() {
                    true => self.cpu_pins.pin_off(M6502_RDY),
                    false => self.cpu_pins.pin_on(M6502_RDY),
                }

                if self.registers.ir() != MikeyInstruction::None {
                    self.process_ir_step(bus, cart);
                }        
               
                if bus.status() == BusStatus::None {
                    self.cpu_tick(bus);
                }
            }
            MikeyBusOwner::Video => {
                match bus.status() {
                    BusStatus::None => {
                        bus.set_addr(match self.registers.is_flipped() {
                            false => self.video_buffer_curr_addr + self.video_buffer_buffer.len() as u16,
                            true => self.video_buffer_curr_addr - self.video_buffer_buffer.len() as u16,
                        });
                        bus.set_data(RAM_PEEK_DATA_DMA);
                        bus.set_status(BusStatus::PeekRAM);
                    }
                    BusStatus::PeekDone => {
                        self.video_buffer_buffer.push(match self.registers.is_flipped() {
                            false => bus.data(),
                            true => bus.data().rotate_left(4)
                        });
                        let buffer_len = self.video_buffer_buffer.len();
                        if buffer_len == 4 {
                            self.video.push_pix_buffer(&self.video_buffer_buffer);
                            self.video_buffer_buffer.clear();
                            self.bus_owner = MikeyBusOwner::Cpu;
                            bus.set_status(BusStatus::None);
                            if self.bus_grant_bkp.is_some() {
                                bus.set_grant(self.bus_grant_bkp.unwrap());
                                trace!("Set bus grant to {}", bus.grant());
                                self.bus_grant_bkp = None;
                            }
                            bus.set_request(false);
                            trace!("[{}] Refresh/Video done.", self.ticks);
                        } else {
                            bus.set_addr(match self.registers.is_flipped() {
                                false => self.video_buffer_curr_addr + buffer_len as u16,
                                true => self.video_buffer_curr_addr - buffer_len as u16,
                            });
                            bus.set_data(RAM_PEEK_DATA_DMA);
                            bus.set_status(BusStatus::PeekRAM);
                        }                        
                    }
                    _ => ()
                }
            },
            MikeyBusOwner::Refresh => {
                trace!("[{}] Refresh", self.ticks);
                self.bus_owner = MikeyBusOwner::Video;
            }
        }
    }

    pub fn get(&self, addr: u16) -> u8 {
        self.registers.data(addr)
    }

    pub fn peek(&mut self, bus: &Bus) {
        assert!(bus.addr() >= MIK_ADDR && bus.addr() <= MIK_ADDR | 0xff);
        let addr = bus.addr();
        match addr {
            TIM0BKUP ..= AUD3MISC => {
                self.registers.set_addr_r(addr);
                self.registers.set_ir(MikeyInstruction::TimersPeek);
                self.registers.set_ticks_delay(MIKEY_TIMER_READ_TICKS);
            },
            INTSET => {
                self.registers.set_addr_r(addr);
                self.registers.set_ir(MikeyInstruction::Peek);
                self.registers.set_ticks_delay(MIKEY_READ_TICKS);
            },
            IODAT => {
                self.registers.set_ir(MikeyInstruction::PeekIodat);
                self.registers.set_ticks_delay(MIKEY_READ_TICKS);
            },
            INTRST => {
                self.registers.set_addr_r(addr);
                self.registers.set_ir(MikeyInstruction::PeekIntRst);
                self.registers.set_ticks_delay(MIKEY_READ_TICKS);
            },
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
            },
        }
        trace!("[{}] > Peek 0x{:04x}", self.ticks, bus.addr());
    }

    pub fn poke(&mut self, bus: &Bus) {
        assert!(bus.addr() >= MIK_ADDR && bus.addr() <= (MIK_ADDR | 0xff));
        let addr = bus.addr();
        match addr {
            TIM0BKUP ..= AUD3MISC  => {
                self.registers.set_addr_r(addr);
                self.registers.set_data_r(bus.data() as u16);
                self.registers.set_ir(MikeyInstruction::TimersPoke);
                self.registers.set_ticks_delay(MIKEY_TIMER_WRITE_TICKS);
            }
            INTSET => {
                self.registers.set_addr_r(addr);
                self.registers.set_data_r(bus.data() as u16);
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
                self.registers.set_data_r(bus.data() as u16);
                self.registers.set_ir(MikeyInstruction::PokeIodat);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
            SYSCTL1 => {
                self.registers.set_addr_r(addr);
                self.registers.set_data_r(bus.data() as u16);
                self.registers.set_ir(MikeyInstruction::PokeSysctl1);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }           
            CPUSLEEP => {
                self.registers.set_addr_r(addr);
                self.registers.set_data_r(bus.data() as u16);
                self.registers.set_ir(MikeyInstruction::CpuSleep);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }      
            DISPCTL => {
                self.registers.set_data_r(bus.data() as u16);
                self.registers.set_ir(MikeyInstruction::PokeDispCtl);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
            SERCTL => {
                self.registers.set_data_r(bus.data() as u16);
                self.registers.set_ir(MikeyInstruction::PokeSerCtl);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
            SERDAT => {
                self.registers.set_data_r(bus.data() as u16);
                self.registers.set_ir(MikeyInstruction::PokeSerDat);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
            PBKUP => {
                self.registers.set_data_r(bus.data() as u16);
                self.registers.set_ir(MikeyInstruction::PokePbkup);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
            ATTEN_A..=MSTEREO => {
                self.registers.set_addr_r(addr);
                self.registers.set_data_r(bus.data() as u16);
                self.registers.set_ir(MikeyInstruction::PokeChangeAttenuation);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
            _ => {
                self.registers.set_addr_r(addr);
                self.registers.set_data_r(bus.data() as u16);
                self.registers.set_ir(MikeyInstruction::Poke);
                self.registers.set_ticks_delay(MIKEY_WRITE_TICKS);
            }
        }
        trace!("[{}] > Poke 0x{:04x} 0x{:02x}", self.ticks, bus.addr(), bus.data());
    }

    fn process_ir_step(&mut self, bus: &mut Bus, cart: &mut Cartridge) {

        match self.registers.ir() {
            
            MikeyInstruction::Peek => { 
                bus.set_data(self.registers.data(self.registers.addr_r())); 
                bus.set_status(BusStatus::PeekDone); 
                self.registers.reset_ir(); 
            }
            MikeyInstruction::Poke => { 
                self.registers.set_data(self.registers.addr_r(), self.registers.data_r() as u8); 
                bus.set_status(BusStatus::PokeDone); 
                self.registers.reset_ir(); 
            }
            MikeyInstruction::PokeChangeAttenuation => { 
                self.registers.set_data(self.registers.addr_r(), self.registers.data_r() as u8); 
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
                self.timers.poke(self.registers.addr_r(), self.registers.data_r() as u8); 
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
                self.iodat_updated(bus, cart);
                self.registers.reset_ir(); 
                bus.set_status(BusStatus::PokeDone); 
            }
            MikeyInstruction::PeekIodat => { 
                let mut iodat = self.registers.data(INTSET);
                if cart.audin() {
                    iodat |= IODAT_AUDIN;
                } else {
                    iodat &= !IODAT_AUDIN;
                }
                bus.set_data(iodat); 
                bus.set_status(BusStatus::PeekDone); 
                self.registers.reset_ir();
            }
            MikeyInstruction::PokeOk => { 
                self.registers.reset_ir(); 
                bus.set_status(BusStatus::PokeDone); 
            }
            MikeyInstruction::PokeIncCartRipple => {
                self.registers.inc_cart_position();
                cart.write_address_to_pins(self.registers.cart_shift(), self.registers.cart_position(), self.registers.audin());
                self.registers.reset_ir(); 
                bus.set_status(BusStatus::PokeDone); 
            }
            MikeyInstruction::PeekIncCartRipple => {
                self.registers.inc_cart_position();
                cart.write_address_to_pins(self.registers.cart_shift(), self.registers.cart_position(), self.registers.audin());
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
                self.registers.set_serctl(&mut self.uart, self.registers.data_r() as u8);
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
                self.uart.set_transmit_holding_buffer(&mut self.registers, data);
                self.registers.reset_ir();
                trace!("< Poke");
                bus.set_status(BusStatus::PokeDone); 
            }   
            MikeyInstruction::PokePbkup => { 
                self.video.set_pbkup(self.registers.data_r() as u8);
                self.registers.reset_ir(); 
                bus.set_status(BusStatus::PokeDone); 
            }
            _ => (),
        }
    }

    fn set_intrst(&mut self, v: u8) {
        trace!("INTRST -> {:02X}", v);
        self.registers.set_data(INTRST, v);
        self.registers.set_data(INTSET, self.registers.data(INTSET) & !v);
    }

    fn sysctl1_updated(&mut self, bus: &mut Bus, cart: &mut Cartridge) {
        let prev = self.registers.data(SYSCTL1);
        let new = self.registers.data_r() as u8;
        self.registers.set_data(SYSCTL1, new); 
        
        if prev & SYSCTL1_POWER == 0 && new & SYSCTL1_POWER != 0 {
            self.registers.reset_cart_position();
            self.registers.reset_cart_shift();
        }

        if new & SYSCTL1_POWER != 0 && prev & SYSCTL1_CAS != 0 && new & SYSCTL1_CAS == 0 {
                let b = if self.registers.data(IODAT) & IODAT_CAD == 0 {0} else {1};
                self.registers.shift_cart_shift(b);
                cart.write_address_to_pins(self.registers.cart_shift(), self.registers.cart_position(), self.registers.audin());
        }

        bus.set_status(BusStatus::PokeDone); 
        self.registers.reset_ir(); 
    }
    
    fn iodat_updated(&mut self, bus: &mut Bus, cart: &mut Cartridge) {
        let new = self.registers.data_r() as u8;
        self.registers.set_data(IODAT, new); 
        
        let b = if self.registers.data(IODAT) & IODAT_AUDIN == 0 {0} else {1};
        self.registers.set_audin(b);
        cart.write_address_to_pins(self.registers.cart_shift(), self.registers.cart_position(), self.registers.audin());

        bus.set_status(BusStatus::PokeDone); 
        self.registers.reset_ir(); 
    }

    pub fn cpu_pins(&self) -> CPUPins {
        self.cpu_pins
    }
    
    pub fn cpu(&self) -> &M6502 {
        &self.cpu
    }
    
    pub fn registers(&self) -> &MikeyRegisters {
        &self.registers
    }

    pub fn registers_mut(&mut self) -> &mut MikeyRegisters {
        &mut self.registers
    }

    pub fn timers(&self) -> &Timers {
        &self.timers
    }

    pub fn audio_sample(&self) -> (i16, i16) {
        let audio0 = self.timers.audio_out(8) as f32;
        let audio1 = self.timers.audio_out(9) as f32;
        let audio2 = self.timers.audio_out(10) as f32;
        let audio3 = self.timers.audio_out(11) as f32;
        
        let left = ((
            audio0 * self.registers.attenuation_left(0) +
            audio1 * self.registers.attenuation_left(1) +
            audio2 * self.registers.attenuation_left(2) +
            audio3 * self.registers.attenuation_left(3)
        ) as i32) << 5;
    
        let right = ((
            audio0 * self.registers.attenuation_right(0) +
            audio1 * self.registers.attenuation_right(1) +
            audio2 * self.registers.attenuation_right(2) +
            audio3 * self.registers.attenuation_right(3)
        ) as i32) << 5;

        (left as i16, right as i16)
    }

    pub fn video_mut(&mut self) -> &mut Video {
        &mut self.video
    }

    pub fn video(&self) -> &Video {
        &self.video
    }

    #[cfg(not(feature = "comlynx_shared_memory"))]
    pub fn set_comlynx_cable(&mut self, cable: &ComlynxCable) {
        self.uart.set_cable(cable);
    }
    
    pub(crate) fn comlynx_cable(&self) -> &ComlynxCable {
        self.uart.cable()
    }
    
    pub fn bus_owner(&self) -> MikeyBusOwner {
        self.bus_owner
    }
}

impl Default for Mikey {
    fn default() -> Self {
        Mikey::new()
    }
}
