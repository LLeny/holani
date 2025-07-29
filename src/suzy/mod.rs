pub mod math;
pub mod registers;
pub mod renderer;
pub mod sprite_data;

use super::{alloc, bus, consts, mikey, ram};
use bus::{Bus, BusStatus};
use consts::{
    COLLADRL, COLLBASL, COLLOFFL, HOFFL, HPOSSTRTH, HPOSSTRTL, HSIZOFFL, JOYSTICK, LINE_END, MATHA,
    MATHB, MATHC, MATHD, MATHE, MATHF, MATHH, MATHK, MATHM, MATHP, PROCADRL, RAM_PAGE_READ_TICKS,
    RCART0, RCART1, SCBADRL, SCBNEXTH, SCBNEXTL, SPRCOLL, SPRCOLL_DONT_COLLIDE, SPRCOLL_NUMBER,
    SPRCTL0, SPRCTL0_BPP, SPRCTL0_HFLIP, SPRCTL0_SPR_TYPE, SPRCTL0_VFLIP, SPRCTL1,
    SPRCTL1_DRAW_QUAD, SPRCTL1_LITERAL, SPRCTL1_RELOAD_HVS, SPRCTL1_RELOAD_HVST,
    SPRCTL1_REUSE_PALETTE, SPRCTL1_SKIP_SPRITE, SPRDLINEH, SPRDLINEL, SPRDOFFL, SPRGO,
    SPRGO_EVERON, SPRGO_GO, SPRHSIZH, SPRHSIZL, SPRSYS, SPRVPOSL, SPRVSIZH, SPRVSIZL, STRETCHH,
    STRETCHL, SUZYBUSEN, SUZYHREV, SUZY_BUS_GRANT_TICKS, SUZY_DATA_BUFFER_LEN, SUZY_READ_TICKS,
    SUZY_SPRITE_SCB_ADDITIONAL_COST, SUZY_SPRITE_VERT_ADDITIONAL_COST, SUZY_WRITE_TICKS, SUZ_ADDR,
    SWITCHES, TILTACUMH, TILTACUML, TILTH, TILTL, TMPADRH, TMPADRL, VIDADRL, VIDBASL, VOFFL,
    VPOSSTRTH, VPOSSTRTL, VSIZACUMH, VSIZACUML, VSIZOFFL,
};
use log::trace;
use math::{divide, multiply, set_matha, set_mathc, set_mathe, set_mathm};
use ram::Ram;
use registers::{Joystick, SprSysR, SprSysW, SuzyRegisters, Switches, TaskStep};
use renderer::Renderer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SuzyInstruction {
    None,
    Peek,
    Poke,
    PeekNothing,
    PokeNothing,
    PokeMathA,
    PokeMathC,
    PokeMathE,
    PokeMathM,
    PokeAndResetNext,
    PeekSprSys,
    PokeSprSys,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SuzyTask {
    None,
    SpriteGo,
    EndSprite,
    Multiply,
    Divide,
}

#[derive(Serialize, Deserialize)]
pub struct Suzy {
    ticks: u64,
    request_monitor: bool,
    pending_bus_request_ticks: i8,
    renderer: Renderer,
    registers: SuzyRegisters,
}

impl Suzy {
    #[must_use]
    pub fn new() -> Self {
        let mut s = Self {
            ticks: 0,
            request_monitor: true,
            pending_bus_request_ticks: -1,
            renderer: Renderer::new(),
            registers: SuzyRegisters::new(),
        };
        s.registers.set_data(SUZYBUSEN, 1);
        s
    }

    pub fn set_joystick(&mut self, joy: u8) {
        self.registers.set_data(JOYSTICK, joy);
    }

    pub fn set_switches(&mut self, sw: u8) {
        self.registers.set_data(SWITCHES, sw);
    }

    #[must_use]
    pub fn joystick(&self) -> Joystick {
        match Joystick::from_bits(self.registers.data(JOYSTICK)) {
            None => Joystick::empty(),
            Some(v) => v,
        }
    }

    #[must_use]
    pub fn switches(&self) -> Switches {
        match Switches::from_bits(self.registers.data(SWITCHES)) {
            None => Switches::empty(),
            Some(v) => v,
        }
    }

    #[must_use]
    pub fn get(&self, addr: u16) -> u8 {
        self.registers.data(addr)
    }

    pub fn peek(&mut self, bus: &mut Bus) {
        assert!(bus.addr() >= SUZ_ADDR && bus.addr() <= (SUZ_ADDR | 0xff));

        match bus.addr() {
            RCART0 => bus.set_status(BusStatus::PeekCart0),
            RCART1 => bus.set_status(BusStatus::PeekCart1),
            SPRSYS => {
                self.registers.set_ir(SuzyInstruction::PeekSprSys);
                self.registers.set_ir_ticks_delay(SUZY_READ_TICKS);
            }
            TMPADRL..=SWITCHES => {
                self.registers.set_addr_r(bus.addr());
                self.registers.set_ir(SuzyInstruction::Peek);
                self.registers.set_ir_ticks_delay(SUZY_READ_TICKS);
            }
            _ => {
                self.registers.set_ir(SuzyInstruction::PeekNothing);
                self.registers.set_ir_ticks_delay(SUZY_READ_TICKS);
            }
        }
        trace!("[{}] > Peek 0x{:04x}", self.ticks, bus.addr());
    }

    pub fn poke(&mut self, bus: &mut Bus) {
        assert!(bus.addr() >= SUZ_ADDR && bus.addr() <= (SUZ_ADDR | 0xff));
        match bus.addr() {
            RCART0 => bus.set_status(BusStatus::PokeCart0),
            RCART1 => bus.set_status(BusStatus::PokeCart1),
            MATHA => {
                self.registers.set_data_r(u16::from(bus.data()));
                self.registers.set_ir(SuzyInstruction::PokeMathA);
                self.registers.set_ir_ticks_delay(SUZY_WRITE_TICKS);
            }
            MATHC => {
                self.registers.set_data_r(u16::from(bus.data()));
                self.registers.set_ir(SuzyInstruction::PokeMathC);
                self.registers.set_ir_ticks_delay(SUZY_WRITE_TICKS);
            }
            MATHE => {
                self.registers.set_data_r(u16::from(bus.data()));
                self.registers.set_ir(SuzyInstruction::PokeMathE);
                self.registers.set_ir_ticks_delay(SUZY_WRITE_TICKS);
            }
            MATHM => {
                self.registers.set_data_r(u16::from(bus.data()));
                self.registers.set_ir(SuzyInstruction::PokeMathM);
                self.registers.set_ir_ticks_delay(SUZY_WRITE_TICKS);
            }
            TMPADRL | TILTACUML | HOFFL | VOFFL | VIDBASL | COLLBASL | VIDADRL | COLLADRL
            | SCBNEXTL | SPRDLINEL | HPOSSTRTL | VPOSSTRTL | SPRHSIZL | SPRVSIZL | STRETCHL
            | TILTL | SPRDOFFL | SPRVPOSL | COLLOFFL | VSIZACUML | HSIZOFFL | VSIZOFFL
            | SCBADRL | PROCADRL | MATHB | MATHD | MATHF | MATHH | MATHK | MATHP => {
                self.registers.set_addr_r(bus.addr());
                self.registers.set_data_r(u16::from(bus.data()));
                self.registers.set_ir(SuzyInstruction::PokeAndResetNext);
                self.registers.set_ir_ticks_delay(SUZY_WRITE_TICKS);
            }
            SPRSYS => {
                self.registers.set_data_r(u16::from(bus.data()));
                self.registers.set_ir(SuzyInstruction::PokeSprSys);
                self.registers.set_ir_ticks_delay(SUZY_WRITE_TICKS);
            }
            TMPADRL..=SWITCHES => {
                self.registers.set_addr_r(bus.addr());
                self.registers.set_data_r(u16::from(bus.data()));
                self.registers.set_ir(SuzyInstruction::Poke);
                self.registers.set_ir_ticks_delay(SUZY_WRITE_TICKS);
            }
            _ => {
                self.registers.set_ir(SuzyInstruction::PokeNothing);
                self.registers.set_ir_ticks_delay(SUZY_WRITE_TICKS);
            }
        }
        trace!(
            "[{}] > Poke 0x{:04x} 0x{:02x}",
            self.ticks,
            bus.addr(),
            bus.data()
        );
    }

    fn grant_bus(&mut self, bus: &mut Bus) {
        bus.set_grant(true);
        bus.set_request(false);
        self.request_monitor = false;
        trace!(
            "Releasing bus, grant:{} request:{}",
            bus.grant(),
            bus.request()
        );
    }

    fn manage_bus(&mut self, bus: &mut Bus) {
        /* "
        Suzy has a bus enable flip-flop. If it is on, Suzy can participate in the bus game.
        If not, then Suzy ignores bus request and always provides bus grant. [...]
        When the bus request line comes on, Suzy will (eventually) relinquish the bus and set the bus grant line on.
        " */
        let req = bus.request();
        if req && req != self.request_monitor {
            self.pending_bus_request_ticks = SUZY_BUS_GRANT_TICKS as i8; // "The time between Mikey requesting the bus and Suzy releasing it is dependant on the state of the currently running process inside of Suzy. The longest process is 30 ticks. Adding the overhead of accepting the bus request and releasing the bus grant brings the total to 40 ticks."
            self.request_monitor = req;
            trace!("Bus requested.");
        }

        match self.pending_bus_request_ticks {
            -1 => (),
            0 => {
                self.pending_bus_request_ticks = -1;
                self.grant_bus(bus);
                trace!("Bus granted.");
            }
            _ => self.pending_bus_request_ticks -= 1,
        }
    }

    fn manage_ir(&mut self, bus: &mut Bus) {
        if self.registers.ir_ticks_delay() > 0 {
            self.registers.dec_ir_ticks_delay();
            return;
        }

        if self.registers.ir() == SuzyInstruction::None {
            return;
        }

        self.process_ir_step(bus);
    }

    fn manage_task(&mut self, bus: &mut Bus, dma_ram: &mut Ram) {
        if self.registers.task_ticks_delay() > 0 {
            self.registers.dec_task_ticks_delay();
            return;
        }

        if self.registers.task() == SuzyTask::None && self.registers.data(SPRGO) & SPRGO_GO != 0 {
            trace!(
                "[SPRGO] = 0x{:02x} and bus acquired.",
                self.registers.data(SPRGO)
            );
            self.registers
                .sprsys_w_disable_flag(SprSysW::sprite_to_stop);
            self.registers.set_task(SuzyTask::SpriteGo);
            self.registers.set_task_step(TaskStep::InitializePainting);
        }

        if self.registers.task() != SuzyTask::None {
            self.process_task_step(bus, dma_ram);
        }

        if has_bus(bus) && self.registers.task() == SuzyTask::None {
            trace!("Work done. Bus granted.");
            self.grant_bus(bus);
        }
    }

    pub fn tick(&mut self, bus: &mut Bus, dma_ram: &mut Ram) {
        self.ticks += 1;
        self.manage_bus(bus);
        if self.pending_bus_request_ticks >= 0 || self.registers.data(SUZYBUSEN) == 0 {
            return;
        }
        self.manage_ir(bus);
        self.manage_task(bus, dma_ram);
    }

    fn process_ir_step(&mut self, bus: &mut Bus) {
        match self.registers.ir() {
            SuzyInstruction::PokeAndResetNext => {
                //  "Any CPU write to an LSB will set the MSB to 0.""
                self.registers
                    .set_u16(self.registers.addr_r(), self.registers.data_r() & 0xff);
                self.registers.reset_ir();
                trace!("< Poke");
                bus.set_status(BusStatus::PokeDone);
            }
            SuzyInstruction::PokeMathA => {
                set_matha(&mut self.registers);
                self.registers.set_task(SuzyTask::Multiply);
                trace!("< Poke");
                bus.set_status(BusStatus::PokeDone);
            }
            SuzyInstruction::PokeMathC => {
                set_mathc(&mut self.registers);
                self.registers.reset_ir();
                trace!("< Poke");
                bus.set_status(BusStatus::PokeDone);
            }
            SuzyInstruction::PokeMathE => {
                set_mathe(&mut self.registers);
                self.registers.set_task(SuzyTask::Divide);
                trace!("< Poke");
                bus.set_status(BusStatus::PokeDone);
            }
            SuzyInstruction::PokeMathM => {
                set_mathm(&mut self.registers);
                self.registers.reset_ir();
                trace!("< Poke");
                bus.set_status(BusStatus::PokeDone);
            }
            SuzyInstruction::Peek => {
                bus.set_data(self.registers.data(self.registers.addr_r()));
                self.registers.reset_ir();
                trace!("< Peek");
                bus.set_status(BusStatus::PeekDone);
            }
            SuzyInstruction::Poke => {
                self.registers
                    .set_data(self.registers.addr_r(), self.registers.data_r() as u8);
                self.registers.reset_ir();
                trace!("< Poke");
                bus.set_status(BusStatus::PokeDone);
            }
            SuzyInstruction::PeekSprSys => {
                bus.set_data(self.registers.sprsys());
                self.registers.reset_ir();
                trace!("< Peek");
                bus.set_status(BusStatus::PeekDone);
            }
            SuzyInstruction::PokeSprSys => {
                self.registers.set_sprsys(self.registers.data_r() as u8);
                self.registers.reset_ir();
                trace!("< Poke");
                bus.set_status(BusStatus::PokeDone);
            }
            SuzyInstruction::PeekNothing => {
                bus.set_data(0xFF);
                self.registers.reset_ir();
                trace!("< Peek");
                bus.set_status(BusStatus::PeekDone);
            }
            SuzyInstruction::PokeNothing => {
                self.registers.reset_ir();
                trace!("< Poke");
                bus.set_status(BusStatus::PokeDone);
            }
            SuzyInstruction::None => self.registers.reset_ir(),
        }
    }

    fn process_task_step(&mut self, bus: &mut Bus, dma_ram: &mut Ram) {
        match self.registers.task() {
            SuzyTask::SpriteGo => {
                if !has_bus(bus) {
                    return;
                }
                if self.renderer.render_sprites(&mut self.registers, dma_ram) {
                    self.registers
                        .set_data(SPRGO, self.registers.data(SPRGO) & !(1_u8));
                    self.registers.reset_task();
                }
            }
            SuzyTask::EndSprite => {
                if !has_bus(bus) {
                    return;
                }
                let mem_access_count = self.renderer.sprite_end(&mut self.registers, dma_ram);
                self.registers
                    .set_task_ticks_delay(mem_access_count * RAM_PAGE_READ_TICKS as u16);
                self.registers.set_task(SuzyTask::SpriteGo);
            }
            SuzyTask::Multiply => {
                multiply(&mut self.registers);
                self.registers.reset_task();
                trace!("< Multiply");
            }
            SuzyTask::Divide => {
                divide(&mut self.registers);
                self.registers.reset_task();
                trace!("< Divide");
            }
            SuzyTask::None => self.registers.reset_task(),
        }
    }

    #[must_use]
    pub fn registers(&self) -> &SuzyRegisters {
        &self.registers
    }

    #[must_use]
    pub fn left_handed(&self) -> bool {
        self.registers.sprsys_w_is_flag_set(SprSysW::left_handed)
    }
}

fn has_bus(bus: &mut Bus) -> bool {
    !bus.grant()
}

impl Default for Suzy {
    fn default() -> Self {
        Suzy::new()
    }
}
