use std::fmt;
use bitflags::bitflags;
use log::trace;
use super::*;

macro_rules! IR_STEPS {
    ($c:ident,$p:ident,$($e:expr),* ) => {
        [
            $(Box::new(|$c: &mut M6502, $p: &mut CPUPins| { $e })),*
        ]
    };
}

macro_rules! NOP11 {
    () => {
        IR_STEPS!(_cpu,_pins,
            { _pins.fetch(_cpu.pc);} ,
            panic!(), panic!(), panic!(), panic!(), panic!(), panic!(), panic!())
    };
}

macro_rules! NOP12 {
    () => {
        IR_STEPS!(_cpu,_pins,
            { _pins.sa(_cpu.pc); },
            { _pins.fetch(_cpu.pc);} ,
            panic!(), panic!(), panic!(), panic!(), panic!(), panic!())
    };
}

macro_rules! NOP22 {
    () => {
        IR_STEPS!(_cpu,_pins,
            { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0; },
            { _pins.fetch(_cpu.pc);} ,
            panic!(), panic!(), panic!(), panic!(), panic!(), panic!())
    };
}

macro_rules! NOP23 {
    () => {
        IR_STEPS!(_cpu,_pins,
            { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0; },
            { _pins.sa(_cpu.pc); },
            { _pins.fetch(_cpu.pc);} ,
            panic!(), panic!(), panic!(), panic!(), panic!())
    };
}

macro_rules! NOP24 {
    () => {
        IR_STEPS!(_cpu,_pins,
            { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0; },
            { _pins.sa(_cpu.pc); },
            { _pins.sa(_cpu.pc); },
            { _pins.fetch(_cpu.pc);} ,
            panic!(), panic!(), panic!(), panic!())
    };
}

macro_rules! NOP34 {
    () => {
        IR_STEPS!(_cpu,_pins,
            { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0; },
            { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0; },
            { _pins.sa(_cpu.pc); },
            { _pins.fetch(_cpu.pc);} ,
            panic!(), panic!(), panic!(), panic!())
    };
}

macro_rules! NOP38 {
    () => {
        IR_STEPS!(_cpu,_pins,
            { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0; },
            { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0; },
            { _pins.sa(_cpu.pc); },
            { _pins.sa(_cpu.pc); },
            { _pins.sa(_cpu.pc); },
            { _pins.sa(_cpu.pc); },
            { _pins.sa(_cpu.pc); },
            { _pins.fetch(_cpu.pc);})
    };
}

macro_rules! RMB {
    ($b: expr) => {
        IR_STEPS!(_cpu,_pins,
            { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;},
            { _pins.sa(_pins.gd() as u16);},
            { _cpu.ad=_pins.gd() as u16 & !(u16::pow(2, $b));},
            { _pins.sd(_cpu.ad as u8);_pins.pin_off(M6502_RW);},
            { _pins.fetch(_cpu.pc);} ,
            panic!(), panic!(), panic!() )                
    };
}


macro_rules! SMB {
    ($b: expr) => {
        IR_STEPS!(_cpu,_pins,
            { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;},
            { _pins.sa(_pins.gd() as u16);},
            { _cpu.ad=_pins.gd() as u16 | u16::pow(2, $b);},
            { _pins.sd(_cpu.ad as u8);_pins.pin_off(M6502_RW);},
            { _pins.fetch(_cpu.pc);} ,
            panic!(), panic!(), panic!() )
    };
}

macro_rules! BBR {
    ($b: expr) => {
        IR_STEPS!(_cpu,_pins,        
            { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;},        
            { _pins.sa(_pins.gd() as u16);},        
            { _pins.sa(_cpu.pc); _cpu.ad=(_pins.gd() & u8::pow(2, $b)) as u16;},
            { _pins.sa(_cpu.pc); if _cpu.ad != 0 { _cpu.ad=_cpu.pc.overflowing_add(1).0; _cpu.ir_step += 2 } },
            { _pins.sa(_cpu.pc); _cpu.ad=_cpu.pc.overflowing_add(1).0.overflowing_add(_pins.gd() as u16).0 as u16; },
            { _pins.sa((_cpu.pc&0xFF00)|(_cpu.ad&0x00FF)); if(_cpu.ad&0xFF00)==(_cpu.pc&0xFF00) { _cpu.pc=_cpu.ad; _cpu.irq_pip>>=1; _cpu.nmi_pip>>=1; _pins.fetch(_cpu.pc); }; },
            { _cpu.pc=_cpu.ad;_pins.fetch(_cpu.pc);},
            panic!())
    };
}

macro_rules! BBS {
    ($b: expr) => {
        IR_STEPS!(_cpu,_pins,        
            { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;},        
            { _pins.sa(_pins.gd() as u16);},        
            { _pins.sa(_cpu.pc); _cpu.ad=(_pins.gd() & u8::pow(2, $b)) as u16;},
            { _pins.sa(_cpu.pc); if _cpu.ad == 0 {  _cpu.ad=_cpu.pc.overflowing_add(1).0; _cpu.ir_step += 2 } },
            { _pins.sa(_cpu.pc); _cpu.ad=_cpu.pc.overflowing_add(1).0.overflowing_add(_pins.gd() as u16).0 as u16; },
            { _pins.sa((_cpu.pc&0xFF00)|(_cpu.ad&0x00FF)); if(_cpu.ad&0xFF00)==(_cpu.pc&0xFF00) { _cpu.pc=_cpu.ad; _cpu.irq_pip>>=1; _cpu.nmi_pip>>=1; _pins.fetch(_cpu.pc); }; },
            { _cpu.pc=_cpu.ad;_pins.fetch(_cpu.pc);},
            panic!())
    };
}

bitflags! {
    #[derive(Copy, Clone, Debug, Serialize, Deserialize)]
    pub struct M6502Flags:u8 {
        const N = 0b10000000; // 80
        const V = 0b01000000; // 40
        const X = 0b00100000; // 20
        const B = 0b00010000; // 10
        const D = 0b00001000; // 08
        const I = 0b00000100; // 04
        const Z = 0b00000010; // 02
        const C = 0b00000001; // 01
    }
}

impl Default for M6502Flags {
    fn default() -> M6502Flags {
        M6502Flags::I | M6502Flags::X | M6502Flags::Z
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug, Serialize, Deserialize)]
    pub struct M6502BreakFlags:u8 {
        const IRQ = 0b00000001;
        const NMI = 0b00000010;
        const RESET=0b00000100;
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Default)]
pub struct CPUPins {
    data: u32,
}

impl CPUPins {
    pub fn set(&mut self, pins: u32) {
        self.data = pins;
    }

    pub fn is_set(&self, v: u32) -> bool {
        self.data & v != 0
    }

    pub fn ga(&self) -> u16 {
        (self.data & 0xffff) as u16
    }
    
    pub fn sa(&mut self, addr: u16) {
        self.data = (self.data & !0xffff) | addr as u32;
    }
    
    pub fn gd(&self) -> u8 {
        ((self.data & 0xff0000) >> 16) as u8
    }
    
    pub fn sd(&mut self, data: u8) {
        self.data = (self.data & !0xff0000) | (((data as u32) << 16) & 0xff0000);
    }

    pub fn sad(&mut self, addr: u16, data: u8) {
        self.sa(addr);
        self.sd(data);
    }
    
    pub fn pin_on(&mut self, pin: u32) {
        self.data |= pin;
    }
    
    pub fn pin_off(&mut self, pin: u32) {
        self.data &= !pin;
    }
    
    pub fn fetch(&mut self, pc: u16) {
        self.sa(pc);
        self.pin_on(M6502_SYNC);
    }

    pub fn pins(&self) -> u32 {
        self.data
    }
}

impl fmt::Debug for CPUPins {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ addr:0x{:04x} data:0x{:02x} RW:{} SYNC:{} IRQ:{} NMI:{} RDY:{} RES:{} }}", 
        self.ga(), 
        self.gd(),
        self.pins() & M6502_RW != 0,
        self.pins() & M6502_SYNC != 0,
        self.pins() & M6502_IRQ != 0,
        self.pins() & M6502_NMI != 0,
        self.pins() & M6502_RDY != 0,
        self.pins() & M6502_RES != 0)
    }
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct M6502 {
    a: u8,
    x: u8,
    y: u8,
    s: u8,
    pc: u16,
    ad: u16,
    flags: M6502Flags,
    break_flags: M6502BreakFlags,
    pins: CPUPins,
    ir: u8,
    ir_step: u8,
    irq_pip: u16,
    nmi_pip: u16,
    pub last_ir_pc: u16,
}

impl M6502 {
    pub fn new() -> M6502 {
        let mut c = M6502 {
            a: 0,
            x: 0,
            y: 0,
            s: 0,
            pc: 0,
            ad: 0,
            flags: M6502Flags::default(),
            break_flags: M6502BreakFlags::empty(),
            pins: CPUPins::default(),
            ir: 0,
            ir_step: 0,
            irq_pip: 0,
            nmi_pip: 0,
            last_ir_pc: 0,
        };
        c.init();
        c
    }

    pub fn init(&mut self) {
        self.pins.set(M6502_RW | M6502_SYNC | M6502_RES);
        self.ir = 0;
        self.ir_step = 0;
    }

    fn adc(&mut self, val: u8) {
        if self.flags.contains(M6502Flags::D) {
            /* decimal mode (credit goes to MAME) */
            let c: u8 = if self.flags.contains(M6502Flags::C) {1} else {0};
            self.flags &= !(M6502Flags::N|M6502Flags::V|M6502Flags::Z|M6502Flags::C);
            let mut al = (self.a & 0x0F).overflowing_add(val & 0x0F).0.overflowing_add(c).0;
            if al > 9 {
                al += 6;
            }
            let mut ah: u8 = (self.a >> 4) + (val >> 4) + (if al > 0x0F {1} else {0});
            if (self.a + val + c) == 0 {
                self.flags |= M6502Flags::Z;
            }
            else if ah & 0x08 != 0 {
                self.flags |= M6502Flags::N;
            }
            if (!(self.a^val) & (self.a^(ah<<4))) & 0x80 != 0 {
                self.flags |= M6502Flags::V;
            }
            if ah > 9 {
                ah += 6;
            }
            if ah > 15 {
                self.flags |= M6502Flags::C;
            }
            self.a = (ah<<4) | (al & 0x0F);
        }
        else {
            /* default mode */
            let sum: u16 = self.a as u16 + val as u16 + (if self.flags.contains(M6502Flags::C) {1} else {0});
            self.flags &= !(M6502Flags::V|M6502Flags::C);
            self.nz(sum as u8);
            if !(self.a^val) & (self.a^sum as u8) & 0x80 != 0 {
                self.flags |= M6502Flags::V;
            }
            if sum & 0xFF00 != 0 {
                self.flags |= M6502Flags::C;
            }
            self.a = sum as u8;
        }
    }

    fn sbc(&mut self, val: u8) {
        if self.flags.contains(M6502Flags::D) {
            /* decimal mode (credit goes to MAME) */
            let c: u8 = if self.flags.contains(M6502Flags::C) {0} else {1};
            self.flags &= !(M6502Flags::N|M6502Flags::V|M6502Flags::Z|M6502Flags::C);
            let diff = (self.a as u16).overflowing_sub(val as u16).0.overflowing_sub(c as u16).0;
            let mut al = (self.a & 0x0F).overflowing_sub(val & 0x0F).0.overflowing_sub(c).0;
            if (al as i8) < 0 {
                al -= 6;
            }
            let mut ah: u8 = (self.a>>4).overflowing_sub(val>>4).0.overflowing_sub(if (al as i8) < 0 {1} else {0}).0;
            if 0 == (diff as u8) {
                self.flags |= M6502Flags::Z;
            }
            else if diff & 0x80 != 0 {
                self.flags |= M6502Flags::N;
            }
            if ((self.a^val) & (self.a^(diff as u8)) & 0x80) != 0 {
                self.flags |= M6502Flags::V;
            }
            if diff & 0xFF00 == 0 {
                self.flags |= M6502Flags::C;
            }
            if ah & 0x80 != 0 {
                ah -= 6;
            }
            self.a = (ah<<4) | (al & 0x0F);
        }
        else {
            /* default mode */
            let c: u8 = if self.flags.contains(M6502Flags::C) {0} else {1};
            let diff = (self.a as u16).overflowing_sub(val as u16).0.overflowing_sub(c as u16).0;
            self.flags &= !(M6502Flags::V|M6502Flags::C);
            self.nz( diff as u8);
            if ((self.a^val) & (self.a^(diff as u8)) & 0x80) != 0 {
                self.flags |= M6502Flags::V;
            }
            if diff & 0xFF00 == 0 {
                self.flags |= M6502Flags::C;
            }
            self.a = diff as u8;
        }
    }

    fn cmp(&mut self, r: u8, v: u8) {
        let t: u16 = (r as u16).overflowing_sub(v as u16).0;
        self.nz( t as u8);
        self.flags &= !M6502Flags::C;
        if t & 0xFF00 == 0 {
            self.flags |= M6502Flags::C;
        }
    }

    fn asl(&mut self, v: u8) -> u8 {
        self.nz( v<<1);
        self.flags &= !M6502Flags::C;
        if v & 0x80 != 0 {
            self.flags |= M6502Flags::C;
        }
        v<<1
    }

    fn lsr(&mut self, v: u8) -> u8 {
        self.nz( v>>1);
        self.flags &= !M6502Flags::C;
        if v & 0x01 != 0 {
            self.flags |= M6502Flags::C;
        }
        v>>1
    }

    fn rol(&mut self, mut v: u8) -> u8 {
        let carry: bool = !(self.flags & M6502Flags::C).is_empty();
        self.flags &= !(M6502Flags::N|M6502Flags::Z|M6502Flags::C);
        if v & 0x80 != 0 {
            self.flags |= M6502Flags::C;
        }
        v <<= 1;
        if carry {
            v |= 1;
        }
        self.nz(v);
        v
    }

    fn ror(&mut self, mut v: u8) -> u8 {
        let carry: bool = !(self.flags & M6502Flags::C).is_empty();
        self.flags &= !(M6502Flags::N|M6502Flags::Z|M6502Flags::C);
        if v & 1 != 0 {
            self.flags |= M6502Flags::C;
        }
        v >>= 1;
        if carry {
            v |= 0x80;
        }
        self.nz(v);
        v
    }

    fn bit(&mut self, v: u8) {
        let t: u8 = self.a & v;
        self.flags &= !(M6502Flags::N|M6502Flags::Z|M6502Flags::V);
        if t == 0 {
            self.flags |= M6502Flags::Z;
        }
        self.flags |= M6502Flags::from_bits(v).unwrap() & (M6502Flags::N|M6502Flags::V);
    }

    pub fn z(&mut self, value: u8) {
        if value == 0 {
            self.flags |= M6502Flags::Z;
        } else {
            self.flags &= !M6502Flags::Z;
        }
    }

    pub fn nz(&mut self, value: u8) {
        self.flags &= !(M6502Flags::N | M6502Flags::Z);
        self.flags |= if value != 0 {
            if value & (M6502Flags::N).bits() != 0 {
                M6502Flags::N
            }
            else {
                M6502Flags::empty()
            }
        } else {
            M6502Flags::Z
        }
    }

    pub fn set_a(&mut self, a: u8) {
        self.a = a;
    }

    pub fn set_x(&mut self, x: u8) {
        self.x = x;
    }

    pub fn set_y(&mut self, y: u8) {
        self.y = y;
    }

    pub fn set_s(&mut self, s: u8) {
        self.s = s;
    }

    pub fn set_pc(&mut self, pc: u16) {
        self.pc = pc;
    }

    pub fn set_flags(&mut self, flags: M6502Flags) {
        self.flags = flags;
    }

    pub fn a(&self) -> u8 {
        self.a
    }

    pub fn x(&self) -> u8 {
        self.x
    }

    pub fn y(&self) -> u8 {
        self.y
    }

    pub fn s(&self) -> u8 {
        self.s
    }

    pub fn pc(&self) -> u16 {
        self.pc
    }

    pub fn flags(&self) -> M6502Flags {
        self.flags
    }

    pub fn break_flags(&self) -> M6502BreakFlags {
        self.break_flags
    }

    pub fn pins(&self) -> CPUPins {
        self.pins
    }

    pub fn ir_step(&self) -> u8 {
        self.ir_step
    }
}

impl Default for M6502 {
    fn default() -> Self {
        Self::new()
    }
}

type InstructionSteps = [Box<dyn Fn(&mut M6502, &mut CPUPins)>; 8];

pub struct M6502Stepper {
    instruction_steps: [InstructionSteps; 0x100],
}

impl M6502Stepper {
    pub fn tick(&self, cpu: &mut M6502, pins: CPUPins) -> CPUPins {
        let mut ps = pins;
        if ps.is_set(M6502_SYNC | M6502_IRQ | M6502_NMI | M6502_RDY | M6502_RES) {
            if 0 != (ps.pins() & (ps.pins() ^ cpu.pins.pins()) & M6502_NMI) {
                cpu.nmi_pip |= 0x100;
            }
            // IRQ test is level triggered
            if ps.is_set(M6502_IRQ) && !cpu.flags.contains(M6502Flags::I) {
                cpu.irq_pip |= 0x100;
            }

            // RDY pin is only checked during read cycles
            if ps.is_set(M6502_RDY) && ps.is_set(M6502_RW) {
                cpu.pins = ps;
                cpu.irq_pip <<= 1;
                return ps;
            }

            if ps.is_set(M6502_SYNC) {
                cpu.ir = ps.gd();
                cpu.ir_step = 0;
                ps.pin_off(M6502_SYNC);
                trace!("Load instruction {:?}", cpu);
                cpu.last_ir_pc = ps.ga();

                // check IRQ, NMI and RES state
                //  - IRQ is level-triggered and must be active in the full cycle
                //    before SYNC
                //  - NMI is edge-triggered,nd the change must have happened in
                //    any cycle before SYNC
                //  - RES behaves slightly different than on a real 6502, we go
                //    into RES state as soon as the pin goes active, from there
                //    on, behaviour is 'standard'
                if 0 != (cpu.irq_pip & 0x400) {
                    cpu.break_flags |= M6502BreakFlags::IRQ;
                }
                if 0 != (cpu.nmi_pip & 0xFC00) {
                    cpu.break_flags |= M6502BreakFlags::NMI;
                }
                if pins.is_set(M6502_RES) {
                    cpu.break_flags |= M6502BreakFlags::RESET;
                }
                cpu.irq_pip &= 0x3FF;
                cpu.nmi_pip &= 0x3FF;

                // if interrupt or reset was requested, force a BRK instruction
                if !cpu.break_flags.is_empty() {
                    cpu.ir = 0;
                    cpu.flags &= !M6502Flags::B;
                    trace!("IRQ, flags:{:08b}", cpu.flags);
                    ps.pin_off(M6502_RES);
                }
                else {
                    cpu.pc+=1;
                }
            }
        }

        ps.pin_on(M6502_RW);

        (self.instruction_steps[cpu.ir as usize][cpu.ir_step as usize])(cpu, &mut ps);
        trace!("IR Step {:?}, pins:{:?}", cpu, ps);
        cpu.ir_step += 1;

        cpu.pins = ps;
        cpu.irq_pip <<= 1;
        cpu.nmi_pip <<= 1;
        cpu.pins
    }

    pub fn new() -> M6502Stepper {
        M6502Stepper { instruction_steps:
        [
            /* 0x00 BRK */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);},
                { if !_cpu.break_flags.contains(M6502BreakFlags::IRQ) && !_cpu.break_flags.contains(M6502BreakFlags::NMI) {_cpu.pc=_cpu.pc.overflowing_add(1).0;};_pins.sad(0x0100|(_cpu.s as u16),(_cpu.pc>>8) as u8); _cpu.s=_cpu.s.overflowing_sub(1).0;if !_cpu.break_flags.contains(M6502BreakFlags::RESET) {_pins.pin_off(M6502_RW);}},
                { _pins.sad(0x0100|(_cpu.s as u16), _cpu.pc as u8);_cpu.s=_cpu.s.overflowing_sub(1).0;if !_cpu.break_flags.contains(M6502BreakFlags::RESET) {_pins.pin_off(M6502_RW);}},
                { _pins.sad(0x0100|(_cpu.s as u16), (_cpu.flags|M6502Flags::X|if _cpu.break_flags.is_empty() {M6502Flags::B} else {M6502Flags::empty()}).bits()); _cpu.s=_cpu.s.overflowing_sub(1).0;if _cpu.break_flags.contains(M6502BreakFlags::RESET) {_cpu.ad=0xFFFC;}else{_pins.pin_off(M6502_RW);if _cpu.break_flags.contains(M6502BreakFlags::NMI) {_cpu.ad=0xFFFA;}else{_cpu.ad=0xFFFE;}}},
                { _pins.sa(_cpu.ad); _cpu.ad +=1;_cpu.flags|=M6502Flags::I|M6502Flags::B;_cpu.break_flags=M6502BreakFlags::empty(); },
                { _pins.sa(_cpu.ad);_cpu.ad=_pins.gd() as u16; },
                { _cpu.pc=((_pins.gd() as u16)<<8)|_cpu.ad;_pins.fetch(_cpu.pc);},
                panic!() ),
            
            /* 0x01 ORA (zp,X) */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _cpu.ad=(_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.a|=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),
            
            /* 0x02 NOP 2 2 */
            NOP22!(),

            /* 0x03 NOP 1 1 */ 
            NOP11!(),

            /* 0x04 TSB zp, 2, 5, A ∨ M → M */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.ad=_pins.gd() as u16;} ,
                { _cpu.z(_cpu.ad as u8 & _cpu.a); _cpu.ad|=_cpu.a as u16;_pins.sd(_cpu.ad as u8);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0x05 ORA zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.a|=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x06 ASL zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _pins.sd(_cpu.asl(_cpu.ad as u8));_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0x07 RMB0 */ 
            RMB!(0),

            /*0x08 PHP */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _pins.sad(0x0100|(_cpu.s as u16),(_cpu.flags|M6502Flags::X|M6502Flags::B).bits());_cpu.s=_cpu.s.overflowing_sub(1).0;_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x09 ORA # */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.a|=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x0A ASL A */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.a=_cpu.asl(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x0B NOP 1 1 */ 
            NOP11!(),

            /*0x0C TSB abs, 3, 6, A ∨ M → M */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _cpu.z(_cpu.ad as u8 & _cpu.a); _cpu.ad|=_cpu.a as u16;_pins.sd(_cpu.ad as u8);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x0D ORA abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.a|=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x0E ASL abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _pins.sd(_cpu.asl(_cpu.ad as u8));_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x0F BBR0 */ 
            BBR!(0),

            /*0x10 BPL # */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc);_cpu.ad=_cpu.pc.overflowing_add((_pins.gd() as i8) as u16).0;if!(_cpu.flags&M6502Flags::N).is_empty(){_pins.fetch(_cpu.pc);};} ,
                { _pins.sa((_cpu.pc&0xFF00)|(_cpu.ad&0x00FF));if(_cpu.ad&0xFF00)==(_cpu.pc&0xFF00){_cpu.pc=_cpu.ad;_cpu.irq_pip>>=1;_cpu.nmi_pip>>=1;_pins.fetch(_cpu.pc);};} ,
                { _cpu.pc=_cpu.ad;_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x11 ORA (zp),Y */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.y as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.y as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.y as u16).0);} ,
                { _cpu.a|=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x12 ORA (zp), 2, 5, A ∨ M → A */
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa(_cpu.ad);} ,
                { _cpu.a|=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /* 0x13 NOP 1 1 */ 
            NOP11!(),

            /*0x14 TRB zp, 2, 5, ~A ∧ M → M */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _cpu.z(_cpu.ad as u8 & _cpu.a); _cpu.ad&=!(_cpu.a as u16);_pins.sd(_cpu.ad as u8);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0x15 ORA zp,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0x00FF);} ,
                { _cpu.a|=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x16 ASL zp,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0x00FF);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _pins.sd(_cpu.asl(_cpu.ad as u8));_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x17 RMB1 */ 
            RMB!(1),

            /*0x18 CLC */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.flags&=!M6502Flags::C;_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x19 ORA abs,Y */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.y as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.y as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.y as u16).0);} ,
                { _cpu.a|=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0x1A INC A, 1, 2, A + 1 → A */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.a = _cpu.a.overflowing_add(1).0; _cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x1B NOP 1 1 */ 
            NOP11!(),

            /*0x1C TRB abs, 6, 3, ~A ∧ M → M */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _cpu.z(_cpu.ad as u8 & _cpu.a); _cpu.ad&=!(_cpu.a as u16);_pins.sd(_cpu.ad as u8);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x1D ORA abs,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.x as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.x as u16).0);} ,
                { _cpu.a|=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0x1E ASL abs,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF));} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.x as u16).0);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _pins.sd(_cpu.asl(_cpu.ad as u8));_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!() ),

            /*0x1F BBR1 */ 
            BBR!(1),

            /*0x20 JSR */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(0x0100|(_cpu.s as u16));_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sad(0x0100|(_cpu.s as u16),(_cpu.pc>>8) as u8);_cpu.s=_cpu.s.overflowing_sub(1).0;_pins.pin_off(M6502_RW);} ,
                { _pins.sad(0x0100|(_cpu.s as u16),_cpu.pc as u8);_cpu.s=_cpu.s.overflowing_sub(1).0;_pins.pin_off(M6502_RW);} ,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.pc=((_pins.gd() as u16)<<8)|_cpu.ad;_pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x21 AND (zp,X) */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _cpu.ad=(_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.a&=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x22 Nop 2 2 */
            NOP22!(),

            /* 0x23 NOP 1 1 */ 
            NOP11!(),

            /*0x24 BIT zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.bit(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x25 AND zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.a&=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x26 ROL zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _pins.sd(_cpu.rol(_cpu.ad as u8));_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0x27 RMB2 */ 
            RMB!(2),

            /*0x28 PLP */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _pins.sa(0x0100|(_cpu.s as u16));_cpu.s=_cpu.s.overflowing_add(1).0;} ,
                { _pins.sa(0x0100|(_cpu.s as u16));} ,
                { _cpu.flags=M6502Flags::from_bits(_pins.gd()).unwrap()&!(M6502Flags::B|M6502Flags::X);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x29 AND # */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.a&=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x2A ROLA */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.a=_cpu.rol(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x2B NOP 1 1 */ 
            NOP11!(),

            /*0x2C BIT abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.bit(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x2D AND abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.a&=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x2E ROL abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _pins.sd(_cpu.rol(_cpu.ad as u8));_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x2F BBR2 */ 
            BBR!(2),

            /*0x30 BMI # */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc);_cpu.ad=_cpu.pc.overflowing_add((_pins.gd() as i8) as u16).0;if(_cpu.flags&M6502Flags::N).is_empty(){_pins.fetch(_cpu.pc);};} ,
                { _pins.sa((_cpu.pc&0xFF00)|(_cpu.ad&0x00FF));if (_cpu.ad&0xFF00)==(_cpu.pc&0xFF00) {_cpu.pc=_cpu.ad;_cpu.irq_pip>>=1;_cpu.nmi_pip>>=1;_pins.fetch(_cpu.pc);};} ,
                { _cpu.pc=_cpu.ad;_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x31 AND (zp),Y */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.y as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.y as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.y as u16).0);} ,
                { _cpu.a&=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x32 AND (zp), 2, 5, A ∧ M → A */
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa(_cpu.ad);} ,
                { _cpu.a&=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /* 0x33 NOP 1 1 */ 
            NOP11!(),

            /*0x34 BIT zp,X, 2, 3, A ∧ M, M7 → N, M6 → V */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0x00FF);} ,
                { _cpu.bit(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x35 AND zp,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0x00FF);} ,
                { _cpu.a&=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x36 ROL zp,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0x00FF);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _pins.sd(_cpu.rol(_cpu.ad as u8));_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x37 RMB3 */ 
            RMB!(3),

            /*0x38 SEC */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.flags|=M6502Flags::C;_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x39 AND abs,Y */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.y as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.y as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.y as u16).0);} ,
                { _cpu.a&=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0x3A DEC A, 2, 1, A - 1 → A */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.a = _cpu.a.overflowing_sub(1).0; _cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x3B NOP 1 1 */ 
            NOP11!(),

            /*0x3C BIT abs,X, 3, 4, A ∧ M, M7 → N, M6 → V */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa(_cpu.ad.overflowing_add(_cpu.x as u16).0);} ,
                { _cpu.bit(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x3D AND abs,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.x as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.x as u16).0);} ,
                { _cpu.a&=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0x3E ROL abs,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF));} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.x as u16).0);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _pins.sd(_cpu.rol(_cpu.ad as u8));_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!() ),

            /*0x3F BBR3 */ 
            BBR!(3),

            /*0x40 RTI */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _pins.sa(0x0100|(_cpu.s as u16));_cpu.s=_cpu.s.overflowing_add(1).0;} ,
                { _pins.sa(0x0100|(_cpu.s as u16));_cpu.s=_cpu.s.overflowing_add(1).0;} ,
                { _pins.sa(0x0100|(_cpu.s as u16));_cpu.s=_cpu.s.overflowing_add(1).0;_cpu.flags=M6502Flags::from_bits(_pins.gd()).unwrap()&!(M6502Flags::B|M6502Flags::X);} ,
                { _pins.sa(0x0100|(_cpu.s as u16));_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.pc=((_pins.gd() as u16)<<8)|_cpu.ad;_pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x41 EOR (zp,X) */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _cpu.ad=(_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.a^=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x42 NOP 2 2 */
            NOP22!(),

            /* 0x43 NOP 1 1 */ 
            NOP11!(),

            /*0x44 NOP 2 3 */ 
            NOP23!(),

            /*0x45 EOR zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.a^=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x46 LSR zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _pins.sd(_cpu.lsr(_cpu.ad as u8));_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0x47 RMB4 */ 
            RMB!(4),

            /*0x48 PHA */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _pins.sad(0x0100|(_cpu.s as u16),_cpu.a);_cpu.s=_cpu.s.overflowing_sub(1).0;_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x49 EOR # */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.a^=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x4A LSRA */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.a=_cpu.lsr(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x4B NOP 1 1 */ 
            NOP11!(),

            /*0x4C JMP */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0; _cpu.ad=_pins.gd() as u16; },
                { _cpu.pc=((_pins.gd() as u16)<<8)|_cpu.ad; _pins.fetch(_cpu.pc); } ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x4D EOR abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.a^=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x4E LSR abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _pins.sd(_cpu.lsr(_cpu.ad as u8));_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x4F BBR4 */ 
            BBR!(4),

            /*0x50 BVC # */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc);_cpu.ad=_cpu.pc.overflowing_add((_pins.gd() as i8) as u16).0;if!(_cpu.flags&M6502Flags::V).is_empty(){_pins.fetch(_cpu.pc);};} ,
                { _pins.sa((_cpu.pc&0xFF00)|(_cpu.ad&0x00FF));if (_cpu.ad&0xFF00)==(_cpu.pc&0xFF00) {_cpu.pc=_cpu.ad;_cpu.irq_pip>>=1;_cpu.nmi_pip>>=1;_pins.fetch(_cpu.pc);};} ,
                { _cpu.pc=_cpu.ad;_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x51 EOR (zp),Y */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.y as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.y as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.y as u16).0);} ,
                { _cpu.a^=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x52 EOR (zp), 2, 5, A ⊻ M → A */
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa(_cpu.ad);} ,
                { _cpu.a^=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),
            
            /* 0x53 NOP 1 1 */ 
            NOP11!(),

            /*0x54 NOP 2 3 */ 
            NOP23!(),

            /*0x55 EOR zp,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0x00FF);} ,
                { _cpu.a^=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x56 LSR zp,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0x00FF);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _pins.sd(_cpu.lsr(_cpu.ad as u8));_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x57 RMB5 */ 
            RMB!(5),

            /*0x58 CLI */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.flags&=!M6502Flags::I;_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x59 EOR abs,Y */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.y as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.y as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.y as u16).0);} ,
                { _cpu.a^=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0x5A PHY, 1, 3, Y↑ */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _pins.sad(0x0100|(_cpu.s as u16),_cpu.y);_cpu.s=_cpu.s.overflowing_sub(1).0;_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x5B NOP 1 1 */ 
            NOP11!(),

            /*0x5C NOP 3 8 */ 
            NOP38!(),

            /*0x5D EOR abs,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.x as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.x as u16).0);} ,
                { _cpu.a^=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0x5E LSR abs,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF));} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.x as u16).0);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _pins.sd(_cpu.lsr(_cpu.ad as u8));_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!() ),

            /*0x5F BBR5 */ 
            BBR!(5),

            /*0x60 RTS */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _pins.sa(0x0100|(_cpu.s as u16));_cpu.s=_cpu.s.overflowing_add(1).0;} ,
                { _pins.sa(0x0100|(_cpu.s as u16));_cpu.s=_cpu.s.overflowing_add(1).0;} ,
                { _pins.sa(0x0100|(_cpu.s as u16));_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.pc=((_pins.gd() as u16)<<8)|_cpu.ad;_pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x61 ADC (zp,X) */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _cpu.ad=(_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.adc(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x62 Nop 2 2 */
            NOP22!(),
            
            /* 0x63 NOP 1 1 */ 
            NOP11!(),

            /*0x64 STZ zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);_pins.sd(0);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x65 ADC zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.adc(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x66 ROR zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _pins.sd(_cpu.ror(_cpu.ad as u8));_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0x67 RMB6 */ 
            RMB!(6),

            /*0x68 PLA */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _pins.sa(0x0100|(_cpu.s as u16));_cpu.s=_cpu.s.overflowing_add(1).0;} ,
                { _pins.sa(0x0100|(_cpu.s as u16));} ,
                { _cpu.a=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x69 ADC # */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.adc(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x6A ROR A */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.a=_cpu.ror(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x6B NOP 1 1 */ 
            NOP11!(),

            /*0x6C JMPI */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad & 0xFF00)|((_cpu.ad+1)&0x00FF));_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.pc=((_pins.gd() as u16)<<8)|_cpu.ad;_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0x6D ADC abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.adc(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x6E ROR abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _pins.sd(_cpu.ror(_cpu.ad as u8));_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x6F BBR6 */ 
            BBR!(6),

            /*0x70 BVS # */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc);_cpu.ad=_cpu.pc.overflowing_add((_pins.gd() as i8) as u16).0;if(_cpu.flags&M6502Flags::V).is_empty(){_pins.fetch(_cpu.pc);};} ,
                { _pins.sa((_cpu.pc&0xFF00)|(_cpu.ad&0x00FF));if (_cpu.ad&0xFF00)==(_cpu.pc&0xFF00) {_cpu.pc=_cpu.ad;_cpu.irq_pip>>=1;_cpu.nmi_pip>>=1;_pins.fetch(_cpu.pc);};} ,
                { _cpu.pc=_cpu.ad;_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x71 ADC (zp),Y */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.y as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.y as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.y as u16).0);} ,
                { _cpu.adc(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x72 ADC (zp), 2, 5-6, A + M + C → A, C */
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa(_cpu.ad);} ,
                { _cpu.adc(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ), 
            
            /* 0x73 NOP 1 1 */ 
            NOP11!(),

            /*0x74 STZ zp,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0x00FF);_pins.sd(0);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x75 ADC zp,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0x00FF);} ,
                { _cpu.adc(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x76 ROR zp,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0x00FF);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _pins.sd(_cpu.ror(_cpu.ad as u8));_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x77 RMB7 */ 
            RMB!(7),

            /*0x78 SEI */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.flags|=M6502Flags::I;_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x79 ADC abs,Y */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.y as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.y as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.y as u16).0);} ,
                { _cpu.adc(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0x7A PLY, 1, 4, Y↑ */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _pins.sa(0x0100|(_cpu.s as u16));_cpu.s=_cpu.s.overflowing_add(1).0;} ,
                { _pins.sa(0x0100|(_cpu.s as u16));} ,
                { _cpu.y=_pins.gd();_cpu.nz(_cpu.y);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x7B NOP 1 1 */ 
            NOP11!(),

            /*0x7C JMP (abs, X), 3, 6, [PC + 1] → PCL, [PC + 2] → PCH */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8; } ,
                { _cpu.ad=_cpu.ad.overflowing_add(_cpu.x as u16).0; _pins.sa(_cpu.ad);} ,
                { _pins.sa(_cpu.ad+1);_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.pc=((_pins.gd() as u16)<<8)|_cpu.ad; _pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x7D ADC abs,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.x as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.x as u16).0);} ,
                { _cpu.adc(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0x7E ROR abs,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF));} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.x as u16).0);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _pins.sd(_cpu.ror(_cpu.ad as u8));_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!() ),

            /*0x7F BBR7 */ 
            BBR!(7),

            /*0x80 BRA  */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.ad=_cpu.pc.overflowing_add((_pins.gd() as i8) as u16).0;} ,
                { _cpu.pc=_cpu.ad;_pins.fetch(_cpu.pc);},
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x81 STA (zp,X) */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _cpu.ad=(_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);_pins.sd(_cpu.a);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x82 Nop 2 2 */ 
            NOP22!(),

            /*0x83 NOP 1 1 */ 
            NOP11!(),

            /*0x84 STY zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);_pins.sd(_cpu.y);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x85 STA zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);_pins.sd(_cpu.a);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x86 STX zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);_pins.sd(_cpu.x);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x87 SMB0 */ 
            SMB!(0),

            /*0x88 DEY */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.y = _cpu.y.overflowing_sub(1).0;_cpu.nz(_cpu.y);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x89 BIT # , 2, 3, A ∧ M, M7 → N, M6 → V */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.ad=_pins.gd() as u16;} ,
                { _cpu.bit(_cpu.ad as u8);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x8A TXA */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.a=_cpu.x;_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x8B NOP 1 1 */ 
            NOP11!(),

            /*0x8C STY abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);_pins.sd(_cpu.y);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x8D STA abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);_pins.sd(_cpu.a);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x8E STX abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);_pins.sd(_cpu.x);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x8F BBS0 */ 
            BBS!(0),

            /*0x90 BCC # */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc);_cpu.ad=_cpu.pc.overflowing_add((_pins.gd() as i8) as u16).0;if !(_cpu.flags&M6502Flags::C).is_empty() {_pins.fetch(_cpu.pc);};} ,
                { _pins.sa((_cpu.pc&0xFF00)|(_cpu.ad&0x00FF));if (_cpu.ad&0xFF00)==(_cpu.pc&0xFF00) {_cpu.pc=_cpu.ad;_cpu.irq_pip>>=1;_cpu.nmi_pip>>=1;_pins.fetch(_cpu.pc);};} ,
                { _cpu.pc=_cpu.ad;_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x91 STA (zp),Y */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(1).0)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.y as u16).0)&0xFF));} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.y as u16).0);_pins.sd(_cpu.a);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0x92 STA (zp), 2, 5, A → M */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(1).0)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa(_cpu.ad);_pins.sd(_cpu.a);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),
            
            /* 0x93 NOP 1 1 */ 
            NOP11!(),

            /*0x94 STY zp,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0x00FF);_pins.sd(_cpu.y);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x95 STA zp,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0x00FF);_pins.sd(_cpu.a);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x96 STX zp,Y */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(_cpu.y as u16).0)&0x00FF);_pins.sd(_cpu.x);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x97 SMB1 */ 
            SMB!(1),

            /*0x98 TYA */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.a=_cpu.y;_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x99 STA abs,Y */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.y as u16).0)&0xFF));} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.y as u16).0);_pins.sd(_cpu.a);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0x9A TXS */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.s=_cpu.x;_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0x9B NOP 1 1 */ 
            NOP11!(),

            /*0x9C STZ abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);_pins.sd(0);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0x9D STA abs,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF));} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.x as u16).0);_pins.sd(_cpu.a);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0x9E STZ abs,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF));} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.x as u16).0);_pins.sd(0);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0x9F BBS1 */ 
            BBS!(1),

            /*0xA0 LDY # */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.y=_pins.gd();_cpu.nz(_cpu.y);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xA1 LDA (zp,X) */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _cpu.ad=(_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.a=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0xA2 LDX # */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.x=_pins.gd();_cpu.nz(_cpu.x);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xA3 NOP 1 1 */ 
            NOP11!(),

            /*0xA4 LDY zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.y=_pins.gd();_cpu.nz(_cpu.y);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xA5 LDA zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.a=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xA6 LDX zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.x=_pins.gd();_cpu.nz(_cpu.x);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xA7 SMB2 */ 
            SMB!(2),

            /*0xA8 TAY */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.y=_cpu.a;_cpu.nz(_cpu.y);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xA9 LDA # */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.a=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xAA TAX */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.x=_cpu.a;_cpu.nz(_cpu.x);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xAB NOP 1 1 */ 
            NOP11!(),

            /*0xAC LDY abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.y=_pins.gd();_cpu.nz(_cpu.y);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0xAD LDA abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.a=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0xAE LDX abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.x=_pins.gd();_cpu.nz(_cpu.x);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0xAF BBS2 */ 
            BBS!(2),

            /*0xB0 BCS # */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc);_cpu.ad=_cpu.pc.overflowing_add((_pins.gd() as i8) as u16).0;if (_cpu.flags&M6502Flags::C).is_empty() {_pins.fetch(_cpu.pc);};} ,
                { _pins.sa((_cpu.pc&0xFF00)|(_cpu.ad&0x00FF));if (_cpu.ad&0xFF00)==(_cpu.pc&0xFF00) {_cpu.pc=_cpu.ad;_cpu.irq_pip>>=1;_cpu.nmi_pip>>=1;_pins.fetch(_cpu.pc);};} ,
                { _cpu.pc=_cpu.ad;_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0xB1 LDA (zp),Y */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.y as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.y as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.y as u16).0);} ,
                { _cpu.a=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0xB2 LDA (zp), 2, 5, M → A */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa(_cpu.ad);} ,
                { _cpu.a=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),
            
            /* 0xB3 NOP 1 1 */ 
            NOP11!(),

            /*0xB4 LDY zp,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0x00FF);} ,
                { _cpu.y=_pins.gd();_cpu.nz(_cpu.y);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0xB5 LDA zp,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0x00FF);} ,
                { _cpu.a=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0xB6 LDX zp,Y */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(_cpu.y as u16).0)&0x00FF);} ,
                { _cpu.x=_pins.gd();_cpu.nz(_cpu.x);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0xB7 SMB3 */ 
            SMB!(3),

            /*0xB8 CLV */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.flags&=!M6502Flags::V;_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xB9 LDA abs,Y */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|(_cpu.ad.overflowing_add(_cpu.y as u16).0 & 0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.y as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.y as u16).0);} ,
                { _cpu.a=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0xBA TSX */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.x=_cpu.s;_cpu.nz(_cpu.x);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xBB NOP 1 1 */ 
            NOP11!(),

            /*0xBC LDY abs,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.x as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.x as u16).0);} ,
                { _cpu.y=_pins.gd();_cpu.nz(_cpu.y);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0xBD LDA abs,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.x as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.x as u16).0);} ,
                { _cpu.a=_pins.gd();_cpu.nz(_cpu.a);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0xBE LDX abs,Y */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.y as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.y as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.y as u16).0);} ,
                { _cpu.x=_pins.gd();_cpu.nz(_cpu.x);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0xBF BBS3 */ 
            BBS!(3),

            /*0xC0 CPY # */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.cmp(_cpu.y, _pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xC1 CMP (zp,X) */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _cpu.ad=(_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.cmp(_cpu.a, _pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0xC2 NOP 2 2 */ 
            NOP22!(),

            /*0xC3 NOP 1 1 */ 
            NOP11!(),

            /*0xC4 CPY zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.cmp(_cpu.y, _pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xC5 CMP zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.cmp(_cpu.a, _pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xC6 DEC zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _cpu.ad=_cpu.ad.overflowing_sub(1).0;_cpu.nz(_cpu.ad as u8);_pins.sd(_cpu.ad as u8);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0xC7 SMB4 */ 
            SMB!(4),

            /*0xC8 INY */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.y = _cpu.y.overflowing_add(1).0;_cpu.nz(_cpu.y);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xC9 CMP # */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.cmp(_cpu.a, _pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xCA DEX */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.x=_cpu.x.overflowing_sub(1).0;_cpu.nz(_cpu.x);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xCB NOP 1 1 */ 
            NOP11!(),

            /*0xCC CPY abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.cmp(_cpu.y, _pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0xCD CMP abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.cmp(_cpu.a, _pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0xCE DEC abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _cpu.ad=_cpu.ad.overflowing_sub(1).0;_cpu.nz(_cpu.ad as u8);_pins.sd(_cpu.ad as u8);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0xCF BBS4 */ 
            BBS!(4),

            /*0xD0 BNE # */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.ad=_cpu.pc.overflowing_add((_pins.gd() as i8) as u16).0; if !(_cpu.flags&M6502Flags::Z).is_empty() {_pins.fetch(_cpu.pc);};} ,
                { _pins.sa((_cpu.pc&0xFF00)|(_cpu.ad&0x00FF));if (_cpu.ad&0xFF00)==(_cpu.pc&0xFF00) {_cpu.pc=_cpu.ad;_cpu.irq_pip>>=1;_cpu.nmi_pip>>=1;_pins.fetch(_cpu.pc);};} ,
                { _cpu.pc=_cpu.ad;_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0xD1 CMP (zp),Y */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.y as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.y as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.y as u16).0);} ,
                { _cpu.cmp(_cpu.a, _pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0xD2 CMP (zp), 2, 5, A - M */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa(_cpu.ad);} ,
                { _cpu.cmp(_cpu.a, _pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /* 0xD3 NOP 1 1 */ 
            NOP11!(),

            /*0xD4 NOP 2 4 */ 
            NOP24!(),

            /*0xD5 CMP zp,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0x00FF);} ,
                { _cpu.cmp(_cpu.a, _pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0xD6 DEC zp,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0x00FF);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _cpu.ad=_cpu.ad.overflowing_sub(1).0;_cpu.nz(_cpu.ad as u8);_pins.sd(_cpu.ad as u8);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0xD7 SMB5 */ 
            SMB!(5),

            /*0xD8 CLD */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.flags&=!M6502Flags::D;_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xD9 CMP abs,Y */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.y as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.y as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.y as u16).0);} ,
                { _cpu.cmp(_cpu.a, _pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0xDA PHX, 1, 3, X↑ */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _pins.sad(0x0100|(_cpu.s as u16),_cpu.x);_cpu.s=_cpu.s.overflowing_sub(1).0;_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xDB NOP 1 1 */ 
            NOP11!(),

            /*0xDC NOP 3 4 */ 
            NOP34!(),

            /*0xDD CMP abs,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.x as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.x as u16).0);} ,
                { _cpu.cmp(_cpu.a, _pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0xDE DEC abs,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF));} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.x as u16).0);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _cpu.ad=_cpu.ad.overflowing_sub(1).0;_cpu.nz(_cpu.ad as u8);_pins.sd(_cpu.ad as u8);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!() ),

            /*0xDF BBS5 */ 
            BBS!(5),

            /*0xE0 CPX # */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.cmp(_cpu.x, _pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xE1 SBC (zp,X) */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _cpu.ad=(_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(1).0)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.sbc(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0xE2 NOP 2 2 */ 
            NOP22!(),

            /*0xE3 NOP 1 1 */ 
            NOP11!(),

            /*0xE4 CPX zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.cmp(_cpu.x, _pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xE5 SBC zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.sbc(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xE6 INC zp */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_pins.gd() as u16);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _cpu.ad=_cpu.ad.overflowing_add(1).0;_cpu.nz(_cpu.ad as u8);_pins.sd(_cpu.ad as u8);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0xE7 SMB6 */ 
            SMB!(6),

            /*0xE8 INX */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.x = _cpu.x.overflowing_add(1).0;_cpu.nz(_cpu.x);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xE9 SBC # */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.sbc(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xEA NOP */ 
            NOP12!(),

            /*0xEB NOP 1 1 */ 
            NOP11!(),

            /*0xEC CPX abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.cmp(_cpu.x, _pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0xED SBC abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.sbc(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0xEE INC abs */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _pins.sa(((_pins.gd() as u16)<<8)|_cpu.ad);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _cpu.ad=_cpu.ad.overflowing_add(1).0;_cpu.nz(_cpu.ad as u8);_pins.sd(_cpu.ad as u8);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0xEF BBS6 */ 
            BBS!(6),

            /*0xF0 BEQ # */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc);_cpu.ad=_cpu.pc.overflowing_add((_pins.gd() as i8) as u16).0;if (_cpu.flags&M6502Flags::Z).is_empty() {_pins.fetch(_cpu.pc);};} ,
                { _pins.sa((_cpu.pc&0xFF00)|(_cpu.ad&0x00FF));if (_cpu.ad&0xFF00)==(_cpu.pc&0xFF00) {_cpu.pc=_cpu.ad;_cpu.irq_pip>>=1;_cpu.nmi_pip>>=1;_pins.fetch(_cpu.pc);};} ,
                { _cpu.pc=_cpu.ad;_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0xF1 SBC (zp),Y */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.y as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.y as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.y as u16).0);} ,
                { _cpu.sbc(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0xF2 SBC (zp), 2, 5-6, A - M - ~C → A */
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad+1)&0xFF);_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa(_cpu.ad);} ,
                { _cpu.sbc(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ), 
            
            /* 0xF3 NOP 1 1 */ 
            NOP11!(),

            /*0xF4 NOP 2 4 */ 
            NOP24!(),

            /*0xF5 SBC zp,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0x00FF);} ,
                { _cpu.sbc(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0xF6 INC zp,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _cpu.ad=_pins.gd() as u16;_pins.sa(_cpu.ad);} ,
                { _pins.sa((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0x00FF);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _cpu.ad=_cpu.ad.overflowing_add(1).0;_cpu.nz(_cpu.ad as u8);_pins.sd(_cpu.ad as u8);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!(), panic!() ),

            /*0xF7 SMB0 */ 
            SMB!(7),

            /*0xF8 SED */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _cpu.flags|=M6502Flags::D;_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!(), panic!(), panic!() ),

            /*0xF9 SBC abs,Y */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.y as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.y as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.y as u16).0);} ,
                { _cpu.sbc(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0xFA PLX, 1, 4, X↑ */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc);} ,
                { _pins.sa(0x0100|(_cpu.s as u16));_cpu.s=_cpu.s.overflowing_add(1).0;} ,
                { _pins.sa(0x0100|(_cpu.s as u16));} ,
                { _cpu.x=_pins.gd();_cpu.nz(_cpu.x);_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!(), panic!() ),

            /*0xFB NOP 1 1 */ 
            NOP11!(),

            /*0xFC NOP 3 4 */ 
            NOP34!(),

            /*0xFD SBC abs,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF));let v = (_cpu.ad>>8).overflowing_sub(_cpu.ad.overflowing_add(_cpu.x as u16).0>>8).0;_cpu.ir_step+=(!v as u8) & 1;} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.x as u16).0);} ,
                { _cpu.sbc(_pins.gd());_pins.fetch(_cpu.pc);} ,
                panic!(), panic!(), panic!() ),

            /*0xFE INC abs,X */ 
            IR_STEPS!(_cpu,_pins,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;} ,
                { _pins.sa(_cpu.pc); _cpu.pc=_cpu.pc.overflowing_add(1).0;_cpu.ad=_pins.gd() as u16;} ,
                { _cpu.ad|=(_pins.gd() as u16)<<8;_pins.sa((_cpu.ad&0xFF00)|((_cpu.ad.overflowing_add(_cpu.x as u16).0)&0xFF));} ,
                { _pins.sa(_cpu.ad.overflowing_add(_cpu.x as u16).0);} ,
                { _cpu.ad=_pins.gd() as u16;_pins.pin_off(M6502_RW);} ,
                { _cpu.ad=_cpu.ad.overflowing_add(1).0;_cpu.nz(_cpu.ad as u8);_pins.sd(_cpu.ad as u8);_pins.pin_off(M6502_RW);} ,
                { _pins.fetch(_cpu.pc);} ,
                panic!() ),

            /*0xFF BBS7 */ 
            BBS!(7),
        ],        
        }
    }
}

impl Default for M6502Stepper {
    fn default() -> Self {
        M6502Stepper::new()
    }
}

impl fmt::Debug for M6502 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ a:0x{:02x}, x: 0x{:02x}, y: 0x{:02x},\ns: 0x{:02x} pc: 0x{:04x} ad: 0x{:04x}\nflags: {:?}, break_flags: {:?},\nrq_pip: 0x{:02x}, nmi_pip: 0x{:02x}\nir: 0x{:02x}, ir_step: {}, pins: {:?} }}", 
        self.a, self.x, self.y, self.s, self.pc, self.ad,
        self.flags, self.break_flags, self.irq_pip, self.nmi_pip,
        self.ir, self.ir_step,
        self.pins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCore {
        cpu_pins: CPUPins,
        cpu: M6502,
        stepper: M6502Stepper,
        ram: [u8; 0x10000],
    }

    impl Default for TestCore {
        fn default() -> Self {
            Self { cpu_pins: CPUPins::default(), cpu: M6502::new(), stepper: M6502Stepper::new(), ram: [0; 0x10000] }
        }
    }

    macro_rules! init {
        ($c: expr) => {            
            $c.ram.fill(0);
            $c.cpu.set_s(0xbd);
            $c.cpu.set_flags(M6502Flags::Z | M6502Flags::B | M6502Flags::I);
        }
    }

    macro_rules! T {
        ($b: expr) => {
            assert!($b);
        }
    }

    macro_rules! R {
        ($c:expr, $r: ident) => {
            $c.cpu.$r()
        }
    }

    macro_rules! tf {
        ($c: expr, $expected: expr) => {
            (R!($c, flags)&!(M6502Flags::X|M6502Flags::I|M6502Flags::B)).bits() == $expected.bits()
        };
    }

    fn get(c: &TestCore, addr: u16) -> u8 {
        c.ram[addr as usize]
    }

    fn set(c: &mut TestCore, addr: u16, data: u8) {
        c.ram[addr as usize] = data;
    }

    fn copy(c: &mut TestCore, dest: u16, buf: &[u8]) {
        let d = dest as usize;
        c.ram[d..d+buf.len()].copy_from_slice(buf);
    }

    fn cpu_prefetch(c: &mut TestCore, pc: u16) {
        c.cpu_pins.set(M6502_SYNC);
        c.cpu_pins.sa(pc);
        c.cpu_pins.sd(get(c, pc));
        c.cpu.set_pc(pc);
    }

    fn step(c: &mut TestCore) -> i32 {
        let mut ticks = 0;
        loop {
            c.cpu_pins = c.stepper.tick(&mut c.cpu, c.cpu_pins);
            let addr = c.cpu_pins.ga();

            if c.cpu_pins.is_set(M6502_RW) {
                c.cpu_pins.sd(get(c, addr));
            } else {
                set(c, addr, c.cpu_pins.gd());
            }
            ticks+=1;
            if c.cpu.pins.is_set(M6502_SYNC) {
                break;
            }
        }
        ticks
    }

    fn w8(c: &mut TestCore, addr: u16, data: u8) {
        c.ram[addr as usize] = data;
    }
    
    fn w16(c: &mut TestCore, addr: u16, data: u16) {
        c.ram[addr as usize] = (data & 0xFF) as u8;
        c.ram[(addr+1) as usize] = (data>>8) as u8;
    }
    
    fn r16(c: &TestCore, addr: u16) -> u16 {
        let l = c.ram[addr as usize];
        let h = c.ram[(addr+1) as usize] ;
        ((h as u16) << 8) | (l as u16)
    }

    #[test]
    fn init() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        T!(0 == R!(core, a));
        T!(0 == R!(core, x));
        T!(0 == R!(core, y));
        T!(0xBD == R!(core, s));
        T!(tf!(core, M6502Flags::Z));
    }

    #[test]
    fn brk() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA9, 0xAA,     // LDA #$AA
            0x00,           // BRK
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xA9, 0xBB,     // LDA #$BB
        ];
        copy(&mut core, 0x0000, &prog);
        // set BRK/IRQ vector
        w16(&mut core, 0xFFFE, 0x0010);
        cpu_prefetch(&mut core, 0x0000);
    
        T!(2 == step(&mut core)); T!(R!(core, a) == 0xAA);
        T!(7 == step(&mut core)); T!(R!(core, pc) == 0x0010); T!(R!(core, s)==0xBA); T!(get(&core, 0x01BB) == 0xB4); T!(r16(&core, 0x01BC) == 0x0004);
        T!(2 == step(&mut core)); T!(R!(core, a) == 0xBB);
    }

    #[test]
    fn lda() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            // immediate
            0xA9, 0x00,         // LDA #$00
            0xA9, 0x01,         // LDA #$01
            0xA9, 0x00,         // LDA #$00
            0xA9, 0x80,         // LDA #$80
    
            // zero page
            0xA5, 0x02,         // LDA $02
            0xA5, 0x03,         // LDA $03
            0xA5, 0x80,         // LDA $80
            0xA5, 0xFF,         // LDA $FF
    
            // absolute
            0xAD, 0x00, 0x10,   // LDA $1000
            0xAD, 0xFF, 0xFF,   // LDA $FFFF
            0xAD, 0x21, 0x00,   // LDA $0021
    
            // zero page,X
            0xA2, 0x0F,         // LDX #$0F
            0xB5, 0x10,         // LDA $10,X    => 0x1F
            0xB5, 0xF8,         // LDA $FE,X    => 0x07
            0xB5, 0x78,         // LDA $78,X    => 0x87

            // absolute,X
            0xBD, 0xF1, 0x0F,   // LDA $0x0FF1,X    => 0x1000
            0xBD, 0xF0, 0xFF,   // LDA $0xFFF0,X    => 0xFFFF
            0xBD, 0x12, 0x00,   // LDA $0x0012,X    => 0x0021
    
            // absolute,Y
            0xA0, 0xF0,         // LDY #$F0
            0xB9, 0x10, 0x0F,   // LDA $0x0F10,Y    => 0x1000
            0xB9, 0x0F, 0xFF,   // LDA $0xFF0F,Y    => 0xFFFF
            0xB9, 0x31, 0xFF,   // LDA $0xFF31,Y    => 0x0021
    
            // indirect,X
            0xA1, 0xF0,         // LDA ($F0,X)  => 0xFF, second byte in 0x00 => 0x1234
            0xA1, 0x70,         // LDA ($70,X)  => 0x70 => 0x4321
    
            // indirect,Y
            0xB1, 0xFF,         // LDA ($FF),Y  => 0x1234+0xF0 => 0x1324
            0xB1, 0x7F,         // LDA ($7F),Y  => 0x4321+0xF0 => 0x4411

            // zp indirect
            0xB2, 0xFF,         // LDA ($FF)  => 0x1234
            0xB2, 0x7F,         // LDA ($7F)  => 0x4321
        ];
        w8(&mut core, 0x0002, 0x01); w8(&mut core, 0x0003, 0x00); w8(&mut core, 0x0080, 0x80); w8(&mut core, 0x00FF, 0x03);
        w8(&mut core, 0x1000, 0x12); w8(&mut core, 0xFFFF, 0x34); w8(&mut core, 0x0021, 0x56);
        w8(&mut core, 0x001F, 0xAA); w8(&mut core, 0x0007, 0x33); w8(&mut core, 0x0087, 0x22);
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        // immediate
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x01); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x80); T!(tf!(core,M6502Flags::N));
    
        // zero page
        T!(3 == step(&mut core)); T!(R!(core, a) == 0x01); T!(tf!(core,M6502Flags::empty()));
        T!(3 == step(&mut core)); T!(R!(core, a) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(3 == step(&mut core)); T!(R!(core, a) == 0x80); T!(tf!(core,M6502Flags::N));
        T!(3 == step(&mut core)); T!(R!(core, a) == 0x03); T!(tf!(core,M6502Flags::empty()));
    
        // absolute
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x12); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x34); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x56); T!(tf!(core,M6502Flags::empty()));
    
        // zero page,X
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x0F); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0xAA); T!(tf!(core,M6502Flags::N));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x33); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x22); T!(tf!(core,M6502Flags::empty()));

        // absolute,X
        T!(5 == step(&mut core)); T!(R!(core, a) == 0x12); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x34); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x56); T!(tf!(core,M6502Flags::empty()));
    
        // absolute,Y
        T!(2 == step(&mut core)); T!(R!(core, y) == 0xF0); T!(tf!(core,M6502Flags::N));
        T!(5 == step(&mut core)); T!(R!(core, a) == 0x12); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x34); T!(tf!(core,M6502Flags::empty()));
        T!(5 == step(&mut core)); T!(R!(core, a) == 0x56); T!(tf!(core,M6502Flags::empty()));
    
        // indirect,X
        w8(&mut core, 0x00FF, 0x34); w8(&mut core, 0x0000, 0x12); w16(&mut core, 0x007f, 0x4321);
        w8(&mut core, 0x1234, 0x89); w8(&mut core, 0x4321, 0x8A);
        T!(6 == step(&mut core)); T!(R!(core, a) == 0x89); T!(tf!(core,M6502Flags::N));
        T!(6 == step(&mut core)); T!(R!(core, a) == 0x8A); T!(tf!(core,M6502Flags::N));
    
        // indirect,Y (both 6 cycles because page boundary crossed)
        w8(&mut core, 0x1324, 0x98); w8(&mut core, 0x4411, 0xA8);
        T!(6 == step(&mut core)); T!(R!(core, a) == 0x98); T!(tf!(core,M6502Flags::N));
        T!(6 == step(&mut core)); T!(R!(core, a) == 0xA8); T!(tf!(core,M6502Flags::N));

        // zp indirect
        w8(&mut core, 0x1234, 0x47); w8(&mut core, 0x4321, 0x87);
        T!(5 == step(&mut core)); T!(R!(core, a) == 0x47); T!(tf!(core,M6502Flags::empty()));
        T!(5 == step(&mut core)); T!(R!(core, a) == 0x87); T!(tf!(core,M6502Flags::N));
    }
    
    #[test]
    fn ldx() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            // immediate
            0xA2, 0x00,         // LDX #$00
            0xA2, 0x01,         // LDX #$01
            0xA2, 0x00,         // LDX #$00
            0xA2, 0x80,         // LDX #$80
    
            // zero page
            0xA6, 0x02,         // LDX $02
            0xA6, 0x03,         // LDX $03
            0xA6, 0x80,         // LDX $80
            0xA6, 0xFF,         // LDX $FF
    
            // absolute
            0xAE, 0x00, 0x10,   // LDX $1000
            0xAE, 0xFF, 0xFF,   // LDX $FFFF
            0xAE, 0x21, 0x00,   // LDX $0021
    
            // zero page,Y
            0xA0, 0x0F,         // LDY #$0F
            0xB6, 0x10,         // LDX $10,Y    => 0x1F
            0xB6, 0xF8,         // LDX $FE,Y    => 0x07
            0xB6, 0x78,         // LDX $78,Y    => 0x87
    
            // absolute,Y
            0xA0, 0xF0,         // LDY #$F0
            0xBE, 0x10, 0x0F,   // LDX $0F10,Y    => 0x1000
            0xBE, 0x0F, 0xFF,   // LDX $FF0F,Y    => 0xFFFF
            0xBE, 0x31, 0xFF,   // LDX $FF31,Y    => 0x0021
        ];
        w8(&mut core, 0x0002, 0x01); w8(&mut core, 0x0003, 0x00); w8(&mut core, 0x0080, 0x80); w8(&mut core, 0x00FF, 0x03);
        w8(&mut core, 0x1000, 0x12); w8(&mut core, 0xFFFF, 0x34); w8(&mut core, 0x0021, 0x56);
        w8(&mut core, 0x001F, 0xAA); w8(&mut core, 0x0007, 0x33); w8(&mut core, 0x0087, 0x22);
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        // immediate
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x01); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x80); T!(tf!(core,M6502Flags::N));
    
        // zero page
        T!(3 == step(&mut core)); T!(R!(core, x) == 0x01); T!(tf!(core,M6502Flags::empty()));
        T!(3 == step(&mut core)); T!(R!(core, x) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(3 == step(&mut core)); T!(R!(core, x) == 0x80); T!(tf!(core,M6502Flags::N));
        T!(3 == step(&mut core)); T!(R!(core, x) == 0x03); T!(tf!(core,M6502Flags::empty()));
    
        // absolute
        T!(4 == step(&mut core)); T!(R!(core, x) == 0x12); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, x) == 0x34); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, x) == 0x56); T!(tf!(core,M6502Flags::empty()));
    
        // zero page,Y
        T!(2 == step(&mut core)); T!(R!(core, y) == 0x0F); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, x) == 0xAA); T!(tf!(core,M6502Flags::N));
        T!(4 == step(&mut core)); T!(R!(core, x) == 0x33); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, x) == 0x22); T!(tf!(core,M6502Flags::empty()));
    
        // absolute,X
        T!(2 == step(&mut core)); T!(R!(core, y) == 0xF0); T!(tf!(core,M6502Flags::N));
        T!(5 == step(&mut core)); T!(R!(core, x) == 0x12); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, x) == 0x34); T!(tf!(core,M6502Flags::empty()));
        T!(5 == step(&mut core)); T!(R!(core, x) == 0x56); T!(tf!(core,M6502Flags::empty()));
    }
    
    #[test]
    fn ldy() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            // immediate
            0xA0, 0x00,         // LDY #$00
            0xA0, 0x01,         // LDY #$01
            0xA0, 0x00,         // LDY #$00
            0xA0, 0x80,         // LDY #$80
    
            // zero page
            0xA4, 0x02,         // LDY $02
            0xA4, 0x03,         // LDY $03
            0xA4, 0x80,         // LDY $80
            0xA4, 0xFF,         // LDY $FF
    
            // absolute
            0xAC, 0x00, 0x10,   // LDY $1000
            0xAC, 0xFF, 0xFF,   // LDY $FFFF
            0xAC, 0x21, 0x00,   // LDY $0021
    
            // zero page,X
            0xA2, 0x0F,         // LDX #$0F
            0xB4, 0x10,         // LDY $10,X    => 0x1F
            0xB4, 0xF8,         // LDY $FE,X    => 0x07
            0xB4, 0x78,         // LDY $78,X    => 0x87
    
            // absolute,X
            0xA2, 0xF0,         // LDX #$F0
            0xBC, 0x10, 0x0F,   // LDY $0F10,X    => 0x1000
            0xBC, 0x0F, 0xFF,   // LDY $FF0F,X    => 0xFFFF
            0xBC, 0x31, 0xFF,   // LDY $FF31,X    => 0x0021
        ];
        w8(&mut core, 0x0002, 0x01); w8(&mut core, 0x0003, 0x00); w8(&mut core, 0x0080, 0x80); w8(&mut core, 0x00FF, 0x03);
        w8(&mut core, 0x1000, 0x12); w8(&mut core, 0xFFFF, 0x34); w8(&mut core, 0x0021, 0x56);
        w8(&mut core, 0x001F, 0xAA); w8(&mut core, 0x0007, 0x33); w8(&mut core, 0x0087, 0x22);
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        // immediate
        T!(2 == step(&mut core)); T!(R!(core, y) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, y) == 0x01); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(R!(core, y) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, y) == 0x80); T!(tf!(core,M6502Flags::N));
    
        // zero page
        T!(3 == step(&mut core)); T!(R!(core, y) == 0x01); T!(tf!(core,M6502Flags::empty()));
        T!(3 == step(&mut core)); T!(R!(core, y) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(3 == step(&mut core)); T!(R!(core, y) == 0x80); T!(tf!(core,M6502Flags::N));
        T!(3 == step(&mut core)); T!(R!(core, y) == 0x03); T!(tf!(core,M6502Flags::empty()));
    
        // absolute
        T!(4 == step(&mut core)); T!(R!(core, y) == 0x12); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, y) == 0x34); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, y) == 0x56); T!(tf!(core,M6502Flags::empty()));
    
        // zero page,Y
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x0F); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, y) == 0xAA); T!(tf!(core,M6502Flags::N));
        T!(4 == step(&mut core)); T!(R!(core, y) == 0x33); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, y) == 0x22); T!(tf!(core,M6502Flags::empty()));
    
        // absolute,X
        T!(2 == step(&mut core)); T!(R!(core, x) == 0xF0); T!(tf!(core,M6502Flags::N));
        T!(5 == step(&mut core)); T!(R!(core, y) == 0x12); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, y) == 0x34); T!(tf!(core,M6502Flags::empty()));
        T!(5 == step(&mut core)); T!(R!(core, y) == 0x56); T!(tf!(core,M6502Flags::empty()));
    }
    
    #[test]
    fn sta() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA9, 0x23,             // LDA #$23
            0xA2, 0x10,             // LDX #$10
            0xA0, 0xC0,             // LDY #$C0
            0x85, 0x10,             // STA $10
            0x8D, 0x34, 0x12,       // STA $1234
            0x95, 0x10,             // STA $10,X
            0x9D, 0x00, 0x20,       // STA $2000,X
            0x99, 0x00, 0x20,       // STA $2000,Y
            0x81, 0x10,             // STA ($10,X)
            0x91, 0x20,             // STA ($20),Y
            0x92, 0x22,             // STA ($22) ;->5432
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x23);
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x10);
        T!(2 == step(&mut core)); T!(R!(core, y) == 0xC0);
        T!(3 == step(&mut core)); T!(get(&core, 0x0010) == 0x23);
        T!(4 == step(&mut core)); T!(get(&core, 0x1234) == 0x23);
        T!(4 == step(&mut core)); T!(get(&core, 0x0020) == 0x23);
        T!(5 == step(&mut core)); T!(get(&core, 0x2010) == 0x23);
        T!(5 == step(&mut core)); T!(get(&core, 0x20C0) == 0x23);
        w16(&mut core, 0x0020, 0x4321);
        w16(&mut core, 0x0022, 0x5432);
        T!(6 == step(&mut core)); T!(get(&core, 0x4321) == 0x23);
        T!(6 == step(&mut core)); T!(get(&core, 0x43E1) == 0x23);
        T!(5 == step(&mut core)); T!(get(&core, 0x5432) == 0x23);
    }
    
    #[test]
    fn stx() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA2, 0x23,             // LDX #$23
            0xA0, 0x10,             // LDY #$10
    
            0x86, 0x10,             // STX $10
            0x8E, 0x34, 0x12,       // STX $1234
            0x96, 0x10,             // STX $10,Y
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x23);
        T!(2 == step(&mut core)); T!(R!(core, y) == 0x10);
        T!(3 == step(&mut core)); T!(get(&core, 0x0010) == 0x23);
        T!(4 == step(&mut core)); T!(get(&core, 0x1234) == 0x23);
        T!(4 == step(&mut core)); T!(get(&core, 0x0020) == 0x23);
    }
    
    #[test]
    fn sty() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA0, 0x23,             // LDY #$23
            0xA2, 0x10,             // LDX #$10
    
            0x84, 0x10,             // STX $10
            0x8C, 0x34, 0x12,       // STX $1234
            0x94, 0x10,             // STX $10,Y
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(R!(core, y) == 0x23);
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x10);
        T!(3 == step(&mut core)); T!(get(&core, 0x0010) == 0x23);
        T!(4 == step(&mut core)); T!(get(&core, 0x1234) == 0x23);
        T!(4 == step(&mut core)); T!(get(&core, 0x0020) == 0x23);
    }
    
    #[test]
    fn tax_txa() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA9, 0x00,     // LDA #$00
            0xA2, 0x10,     // LDX #$10
            0xAA,           // TAX
            0xA9, 0xF0,     // LDA #$F0
            0x8A,           // TXA
            0xA9, 0xF0,     // LDA #$F0
            0xA2, 0x00,     // LDX #$00
            0xAA,           // TAX
            0xA9, 0x01,     // LDA #$01
            0x8A,           // TXA
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x10); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0xF0); T!(tf!(core,M6502Flags::N));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0xF0); T!(tf!(core,M6502Flags::N));
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, x) == 0xF0); T!(tf!(core,M6502Flags::N));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x01); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0xF0); T!(tf!(core,M6502Flags::N));
    }
    
    #[test]
    fn tay_tya() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA9, 0x00,     // LDA #$00
            0xA0, 0x10,     // LDY #$10
            0xA8,           // TAY
            0xA9, 0xF0,     // LDA #$F0
            0x98,           // TYA
            0xA9, 0xF0,     // LDA #$F0
            0xA0, 0x00,     // LDY #$00
            0xA8,           // TAY
            0xA9, 0x01,     // LDA #$01
            0x98,           // TYA
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, y) == 0x10); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(R!(core, y) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0xF0); T!(tf!(core,M6502Flags::N));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0xF0); T!(tf!(core,M6502Flags::N));
        T!(2 == step(&mut core)); T!(R!(core, y) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, y) == 0xF0); T!(tf!(core,M6502Flags::N));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x01); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0xF0); T!(tf!(core,M6502Flags::N));
    }
    
    #[test]
    fn dec_inc() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA9, 0x01,     // LDA #$01
            0x3A,           // DEC
            0x3A,           // DEC
            0x1A,           // INC
            0x1A,           // INC            
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x01); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0xFF); T!(tf!(core,M6502Flags::N));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x01); T!(tf!(core,M6502Flags::empty()));
    }

    #[test]
    fn dex_inx_dey_iny() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA2, 0x01,     // LDX #$01
            0xCA,           // DEX
            0xCA,           // DEX
            0xE8,           // INX
            0xE8,           // INX
            0xA0, 0x01,     // LDY #$01
            0x88,           // DEY
            0x88,           // DEY
            0xC8,           // INY
            0xC8,           // INY
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x01); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, x) == 0xFF); T!(tf!(core,M6502Flags::N));
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x01); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(R!(core, y) == 0x01); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(R!(core, y) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, y) == 0xFF); T!(tf!(core,M6502Flags::N));
        T!(2 == step(&mut core)); T!(R!(core, y) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, y) == 0x01); T!(tf!(core,M6502Flags::empty()));
    }
    
    #[test]
    fn txs_tsx() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA2, 0xAA,     // LDX #$AA
            0xA9, 0x00,     // LDA #$00
            0x9A,           // TXS
            0xAA,           // TAX
            0xBA,           // TSX
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(R!(core, x) == 0xAA); T!(tf!(core,M6502Flags::N));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, s) == 0xAA); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, x) == 0xAA); T!(tf!(core,M6502Flags::N));
    }
    
    #[test]
    fn ora() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA9, 0x00,         // LDA #$00
            0xA2, 0x01,         // LDX #$01
            0xA0, 0x02,         // LDY #$02
            0x09, 0x00,         // ORA #$00
            0x05, 0x10,         // ORA $10
            0x15, 0x10,         // ORA $10,X
            0x0d, 0x00, 0x10,   // ORA $1000
            0x1d, 0x00, 0x10,   // ORA $1000,X
            0x19, 0x00, 0x10,   // ORA $1000,Y
            0x01, 0x22,         // ORA ($22,X)
            0x11, 0x20,         // ORA ($20),Y
            0xA9, 0x05,         // LDA #$00
            0x12, 0x20,         // ORA ($20)
        ];
        copy(&mut core, 0x0200, &prog);
        w16(&mut core, 0x0020, 0x1002);
        w16(&mut core, 0x0023, 0x1003);
        w8(&mut core, 0x0010, 1<<0);
        w8(&mut core, 0x0011, 1<<1);
        w8(&mut core, 0x1000, 1<<2);
        w8(&mut core, 0x1001, 1<<3);
        w8(&mut core, 0x1002, 1<<4);
        w8(&mut core, 0x1003, 1<<5);
        w8(&mut core, 0x1004, 1<<6);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x01); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(R!(core, y) == 0x02); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(3 == step(&mut core)); T!(R!(core, a) == 0x01); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x03); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x07); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x0F); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x1F); T!(tf!(core,M6502Flags::empty()));
        T!(6 == step(&mut core)); T!(R!(core, a) == 0x3F); T!(tf!(core,M6502Flags::empty()));
        T!(5 == step(&mut core)); T!(R!(core, a) == 0x7F); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x05); T!(tf!(core,M6502Flags::empty()));
        T!(5 == step(&mut core)); T!(R!(core, a) == 0x15); T!(tf!(core,M6502Flags::empty()));
    }
    
    #[test]
    fn and() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA9, 0xFF,         // LDA #$FF
            0xA2, 0x01,         // LDX #$01
            0xA0, 0x02,         // LDY #$02
            0x29, 0xFF,         // AND #$FF
            0x25, 0x10,         // AND $10
            0x35, 0x10,         // AND $10,X
            0x2d, 0x00, 0x10,   // AND $1000
            0x3d, 0x00, 0x10,   // AND $1000,X
            0x39, 0x00, 0x10,   // AND $1000,Y
            0x21, 0x22,         // AND ($22,X)
            0x31, 0x20,         // AND ($20),Y
            0xA9, 0x15,         // LDA #$15
            0x32, 0x20,         // AND ($20)
        ];
        copy(&mut core, 0x0200, &prog);
        w16(&mut core, 0x0020, 0x1002);
        w16(&mut core, 0x0023, 0x1003);
        w8(&mut core, 0x0010, 0x7F);
        w8(&mut core, 0x0011, 0x3F);
        w8(&mut core, 0x1000, 0x1F);
        w8(&mut core, 0x1001, 0x0F);
        w8(&mut core, 0x1002, 0x07);
        w8(&mut core, 0x1003, 0x03);
        w8(&mut core, 0x1004, 0x01);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(R!(core, a) == 0xFF); T!(tf!(core,M6502Flags::N));
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x01); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(R!(core, y) == 0x02); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0xFF); T!(tf!(core,M6502Flags::N));
        T!(3 == step(&mut core)); T!(R!(core, a) == 0x7F); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x3F); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x1F); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x0F); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x07); T!(tf!(core,M6502Flags::empty()));
        T!(6 == step(&mut core)); T!(R!(core, a) == 0x03); T!(tf!(core,M6502Flags::empty()));
        T!(5 == step(&mut core)); T!(R!(core, a) == 0x01); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x15); T!(tf!(core,M6502Flags::empty()));
        T!(5 == step(&mut core)); T!(R!(core, a) == 0x05); T!(tf!(core,M6502Flags::empty()));
    }
    
    #[test]
    fn eor() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA9, 0xFF,         // LDA #$FF
            0xA2, 0x01,         // LDX #$01
            0xA0, 0x02,         // LDY #$02
            0x49, 0xFF,         // EOR #$FF
            0x45, 0x10,         // EOR $10
            0x55, 0x10,         // EOR $10,X
            0x4d, 0x00, 0x10,   // EOR $1000
            0x5d, 0x00, 0x10,   // EOR $1000,X
            0x59, 0x00, 0x10,   // EOR $1000,Y
            0x41, 0x22,         // EOR ($22,X)
            0x51, 0x20,         // EOR ($20),Y
            0x52, 0x20,         // EOR ($20)
        ];
        copy(&mut core, 0x0200, &prog);
        w16(&mut core, 0x0020, 0x1002);
        w16(&mut core, 0x0023, 0x1003);
        w8(&mut core, 0x0010, 0x7F);
        w8(&mut core, 0x0011, 0x3F);
        w8(&mut core, 0x1000, 0x1F);
        w8(&mut core, 0x1001, 0x0F);
        w8(&mut core, 0x1002, 0x07);
        w8(&mut core, 0x1003, 0x03);
        w8(&mut core, 0x1004, 0x01);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(R!(core, a) == 0xFF); T!(tf!(core,M6502Flags::N));
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x01); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(R!(core, y) == 0x02); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x00); T!(tf!(core,M6502Flags::Z));
        T!(3 == step(&mut core)); T!(R!(core, a) == 0x7F); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x40); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x5F); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x50); T!(tf!(core,M6502Flags::empty()));
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x57); T!(tf!(core,M6502Flags::empty()));
        T!(6 == step(&mut core)); T!(R!(core, a) == 0x54); T!(tf!(core,M6502Flags::empty()));
        T!(5 == step(&mut core)); T!(R!(core, a) == 0x55); T!(tf!(core,M6502Flags::empty()));
        T!(5 == step(&mut core)); T!(R!(core, a) == 0x52); T!(tf!(core,M6502Flags::empty()));
    }
    
    #[test]
    fn nop() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xEA,       // NOP
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
        T!(2 == step(&mut core));
    }
    
    #[test]
    fn pha_pla_php_plp() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA9, 0x23,     // LDA #$23
            0x48,           // PHA
            0xA9, 0x32,     // LDA #$32
            0x68,           // PLA
            0x08,           // PHP
            0xA9, 0x00,     // LDA #$00
            0x28,           // PLP
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x23); T!(R!(core, s) == 0xBD);
        T!(3 == step(&mut core)); T!(R!(core, s) == 0xBC); T!(get(&core, 0x01BD) == 0x23);
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x32);
        T!(4 == step(&mut core)); T!(R!(core, a) == 0x23); T!(R!(core, s) == 0xBD); T!(tf!(core,M6502Flags::empty()));
        T!(3 == step(&mut core)); T!(R!(core, s) == 0xBC); T!(get(&core, 0x01BD) == (M6502Flags::X|M6502Flags::I|M6502Flags::B).bits());
        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::Z));
        T!(4 == step(&mut core)); T!(R!(core, s) == 0xBD); T!(tf!(core,M6502Flags::empty()));
    }

    #[test]
    fn phx_plx_phy_ply() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA2, 0x23,     // LDX #$23
            0xDA,           // PHX
            0xA2, 0x32,     // LDX #$32
            0xFA,           // PLX

            0xA0, 0x23,     // LDY #$23
            0x5A,           // PHY
            0xA0, 0x32,     // LDY #$32
            0x7A,           // PLY
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x23); T!(R!(core, s) == 0xBD);
        T!(3 == step(&mut core)); T!(R!(core, s) == 0xBC); T!(get(&core, 0x01BD) == 0x23);
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x32);
        T!(4 == step(&mut core)); T!(R!(core, x) == 0x23); T!(R!(core, s) == 0xBD); T!(tf!(core,M6502Flags::empty()));
        
        T!(2 == step(&mut core)); T!(R!(core, y) == 0x23); T!(R!(core, s) == 0xBD);
        T!(3 == step(&mut core)); T!(R!(core, s) == 0xBC); T!(get(&core, 0x01BD) == 0x23);
        T!(2 == step(&mut core)); T!(R!(core, y) == 0x32);
        T!(4 == step(&mut core)); T!(R!(core, y) == 0x23); T!(R!(core, s) == 0xBD); T!(tf!(core,M6502Flags::empty()));
    }
    
    #[test]
    fn clc_sec_cli_sei_clv_cld_sed() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xB8,       // CLV
            0x78,       // SEI
            0x58,       // CLI
            
            0x38,       // SEC
            0x18,       // CLC
            0xF8,       // SED
            0xD8,       // CLD
        ];
        copy(&mut core, 0x0200, &prog);
        core.cpu.set_flags(R!(core, flags) | M6502Flags::V);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); //T!(tf!(core,M6502Flags::Z|M6502Flags::IF));   // FIXME: interrupt bit is ignored in tf!(core,)
        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::Z|M6502Flags::C));
        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::Z|M6502Flags::D));
        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::Z));
    }
    
    #[test]
    fn inc_dec() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA2, 0x10,         // LDX #$10
            0xE6, 0x33,         // INC $33
            0xF6, 0x33,         // INC $33,X
            0xEE, 0x00, 0x10,   // INC $1000
            0xFE, 0x00, 0x10,   // INC $1000,X
            0xC6, 0x33,         // DEC $33
            0xD6, 0x33,         // DEC $33,X
            0xCE, 0x00, 0x10,   // DEC $1000
            0xDE, 0x00, 0x10,   // DEC $1000,X
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(0x10 == R!(core, x));
        T!(5 == step(&mut core)); 
        T!(0x01 == get(&core, 0x0033)); 
        T!(tf!(core,M6502Flags::empty()));
        T!(6 == step(&mut core)); T!(0x01 == get(&core, 0x0043)); T!(tf!(core,M6502Flags::empty()));
        T!(6 == step(&mut core)); T!(0x01 == get(&core, 0x1000)); T!(tf!(core,M6502Flags::empty()));
        T!(7 == step(&mut core)); T!(0x01 == get(&core, 0x1010)); T!(tf!(core,M6502Flags::empty()));
        T!(5 == step(&mut core)); T!(0x00 == get(&core, 0x0033)); T!(tf!(core,M6502Flags::Z));
        T!(6 == step(&mut core)); T!(0x00 == get(&core, 0x0043)); T!(tf!(core,M6502Flags::Z));
        T!(6 == step(&mut core)); T!(0x00 == get(&core, 0x1000)); T!(tf!(core,M6502Flags::Z));
        T!(7 == step(&mut core)); T!(0x00 == get(&core, 0x1010)); T!(tf!(core,M6502Flags::Z));
    }
    
    #[test]
    fn adc_sbc() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA9, 0x01,         // LDA #$01
            0x85, 0x10,         // STA $10
            0x8D, 0x00, 0x10,   // STA $1000
            0xA9, 0xFC,         // LDA #$FC
            0xA2, 0x08,         // LDX #$08
            0xA0, 0x04,         // LDY #$04
            0x18,               // CLC
            0x69, 0x01,         // ADC #$01
            0x65, 0x10,         // ADC $10
            0x75, 0x08,         // ADC $8,X
            0x6D, 0x00, 0x10,   // ADC $1000
            0x7D, 0xF8, 0x0F,   // ADC $0FF8,X
            0x79, 0xFC, 0x0F,   // ADC $0FFC,Y
            0xF9, 0xFC, 0x0F,   // SBC $0FFC,Y
            0xFD, 0xF8, 0x0F,   // SBC $0FF8,X
            0xED, 0x00, 0x10,   // SBC $1000
            0xF5, 0x08,         // SBC $8,X
            0xE5, 0x10,         // SBC $10
            0xE9, 0x01,         // SBC #$10

            0x72, 0x20,         // ADC ($20)
            0xF2, 0x20,         // SBC ($20)
        ];
        w16(&mut core, 0x20, 0x3000);
        w8(&mut core, 0x3000, 0x12);
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(0x01 == R!(core, a));
        T!(3 == step(&mut core)); T!(0x01 == get(&core, 0x0010));
        T!(4 == step(&mut core)); T!(0x01 == get(&core, 0x1000));
        T!(2 == step(&mut core)); T!(0xFC == R!(core, a));
        T!(2 == step(&mut core)); T!(0x08 == R!(core, x));
        T!(2 == step(&mut core)); T!(0x04 == R!(core, y));
        T!(2 == step(&mut core));  // CLC
        // ADC
        T!(2 == step(&mut core)); T!(0xFD == R!(core, a)); T!(tf!(core,M6502Flags::N));
        T!(3 == step(&mut core)); T!(0xFE == R!(core, a)); T!(tf!(core,M6502Flags::N));
        T!(4 == step(&mut core)); T!(0xFF == R!(core, a)); T!(tf!(core,M6502Flags::N));
        T!(4 == step(&mut core)); T!(0x00 == R!(core, a)); T!(tf!(core,M6502Flags::Z|M6502Flags::C));
        T!(5 == step(&mut core)); T!(0x02 == R!(core, a)); T!(tf!(core,M6502Flags::empty()));
        T!(5 == step(&mut core)); T!(0x03 == R!(core, a)); T!(tf!(core,M6502Flags::empty()));
        // SBC
        T!(5 == step(&mut core)); T!(0x01 == R!(core, a)); T!(tf!(core,M6502Flags::C));
        T!(5 == step(&mut core)); T!(0x00 == R!(core, a)); T!(tf!(core,M6502Flags::Z|M6502Flags::C));
        T!(4 == step(&mut core)); T!(0xFF == R!(core, a)); T!(tf!(core,M6502Flags::N));
        T!(4 == step(&mut core)); T!(0xFD == R!(core, a)); T!(tf!(core,M6502Flags::N|M6502Flags::C));
        T!(3 == step(&mut core)); T!(0xFC == R!(core, a)); T!(tf!(core,M6502Flags::N|M6502Flags::C));
        T!(2 == step(&mut core)); T!(0xFB == R!(core, a)); T!(tf!(core,M6502Flags::N|M6502Flags::C));
        
        T!(5 == step(&mut core)); T!(0x0E == R!(core, a)); T!(tf!(core,M6502Flags::C));
        T!(5 == step(&mut core)); T!(0xFC == R!(core, a)); T!(tf!(core,M6502Flags::N));
    }
    
    #[test]
    fn cmp_cpx_cpy() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA9, 0x01,     // LDA #$01
            0xA2, 0x02,     // LDX #$02
            0xA0, 0x03,     // LDY #$03

            0xC9, 0x00,     // CMP #$00
            0xC9, 0x01,     // CMP #$01
            0xC9, 0x02,     // CMP #$02

            0xE0, 0x01,     // CPX #$01
            0xE0, 0x02,     // CPX #$02
            0xE0, 0x03,     // CPX #$03

            0xC0, 0x02,     // CPY #$02
            0xC0, 0x03,     // CPY #$03
            0xC0, 0x04,     // CPY #$04

            0xD2, 0x30,      // CMP ($30)
            0xD2, 0x32,      // CMP ($32)
            0xD2, 0x34,      // CMP ($34)
        ];
        w16(&mut core, 0x0030, 0x1234);
        w8(&mut core, 0x1234, 0x00);
        w16(&mut core, 0x0032, 0x1236);
        w8(&mut core, 0x1236, 0x01);
        w16(&mut core, 0x0034, 0x1238);
        w8(&mut core, 0x1238, 0x02);

        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(0x01 == R!(core, a));
        T!(2 == step(&mut core)); T!(0x02 == R!(core, x));
        T!(2 == step(&mut core)); T!(0x03 == R!(core, y));

        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::C));
        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::Z|M6502Flags::C));
        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::N));

        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::C));
        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::Z|M6502Flags::C));
        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::N));

        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::C));
        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::Z|M6502Flags::C));
        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::N));
        
        T!(5 == step(&mut core)); T!(tf!(core,M6502Flags::C));
        T!(5 == step(&mut core)); T!(tf!(core,M6502Flags::Z|M6502Flags::C));
        T!(5 == step(&mut core)); T!(tf!(core,M6502Flags::N));
    }
    
    #[test]
    fn asl() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        // FIXME: more addressing modes
        let prog = [
            0xA9, 0x81,     // LDA #$81
            0xA2, 0x01,     // LDX #$01
            0x85, 0x10,     // STA $10
            0x06, 0x10,     // ASL $10
            0x16, 0x0F,     // ASL $0F,X
            0x0A,           // ASL
            0x0A,           // ASL
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(0x81 == R!(core, a));
        T!(2 == step(&mut core)); T!(0x01 == R!(core, x));
        T!(3 == step(&mut core)); T!(0x81 == get(&core, 0x0010));
        T!(5 == step(&mut core)); T!(0x02 == get(&core, 0x0010)); T!(tf!(core,M6502Flags::C));
        T!(6 == step(&mut core)); T!(0x04 == get(&core, 0x0010)); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(0x02 == R!(core, a)); T!(tf!(core,M6502Flags::C));
        T!(2 == step(&mut core)); T!(0x04 == R!(core, a)); T!(tf!(core,M6502Flags::empty()));
    }
    
    #[test]
    fn lsr() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        // FIXME: more addressing modes
        let prog = [
            0xA9, 0x81,     // LDA #$81
            0xA2, 0x01,     // LDX #$01
            0x85, 0x10,     // STA $10
            0x46, 0x10,     // LSR $10
            0x56, 0x0F,     // LSR $0F,X
            0x4A,           // LSR
            0x4A,           // LSR
         ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);

        T!(2 == step(&mut core)); T!(0x81 == R!(core, a));
        T!(2 == step(&mut core)); T!(0x01 == R!(core, x));
        T!(3 == step(&mut core)); T!(0x81 == get(&core, 0x0010));
        T!(5 == step(&mut core)); T!(0x40 == get(&core, 0x0010)); T!(tf!(core,M6502Flags::C));
        T!(6 == step(&mut core)); T!(0x20 == get(&core, 0x0010)); T!(tf!(core,M6502Flags::empty()));
        T!(2 == step(&mut core)); T!(0x40 == R!(core, a)); T!(tf!(core,M6502Flags::C));
        T!(2 == step(&mut core)); T!(0x20 == R!(core, a)); T!(tf!(core,M6502Flags::empty()));
    }
    
    #[test]
    fn tsb_trb() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA9, 0x10,       // LDA #$10
            0x04, 0x10,       // TSB $10
            0x0C, 0x00, 0x20, // TSB $2000
            0xA9, 0x00,       // LDA #$00
            0x04, 0x11,       // TSB $11
            0x0C, 0x01, 0x20, // TSB $2001

            0xA9, 0x22,       // LDA #$22
            0x14, 0x10,       // TRB $10
            0x1C, 0x00, 0x20, // TRB $2000
            0xA9, 0xC4,       // LDA #$C4
            0x14, 0x10,       // TRB $10
            0x1C, 0x00, 0x20, // TRB $2000

         ];
        w8(&mut core, 0x0010, 0xC0);
        w8(&mut core, 0x2000, 0x22);
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);

    
        T!(2 == step(&mut core)); T!(0x10 == R!(core, a));
        T!(5 == step(&mut core)); T!(0xD0 == get(&core, 0x0010)); 
        T!(6 == step(&mut core)); T!(0x32 == get(&core, 0x2000));
        T!(2 == step(&mut core)); T!(0x00 == R!(core, a));
        T!(5 == step(&mut core)); T!(0x00 == get(&core, 0x0011)); T!(tf!(core,M6502Flags::Z));
        T!(6 == step(&mut core)); T!(0x00 == get(&core, 0x2001)); T!(tf!(core,M6502Flags::Z));

        w8(&mut core, 0x0010, 0xE0);
        w8(&mut core, 0x2000, 0x26);
        T!(2 == step(&mut core)); T!(0x22 == R!(core, a));
        T!(5 == step(&mut core)); T!(0xC0 == get(&core, 0x0010)); 
        T!(6 == step(&mut core)); T!(0x04 == get(&core, 0x2000)); 
        T!(2 == step(&mut core)); T!(0xC4 == R!(core, a));
        T!(5 == step(&mut core)); T!(0x00 == get(&core, 0x0010)); T!(tf!(core,M6502Flags::N));
        T!(6 == step(&mut core)); T!(0x00 == get(&core, 0x2000)); T!(tf!(core,M6502Flags::N));
    }

    #[test]
    fn rmb_smb() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0x07, 0x10,       // RMB0 $10
            0x17, 0x11,       // RMB1 $11
            0x27, 0x12,       // RMB2 $12
            0x37, 0x13,       // RMB3 $13
            0x47, 0x14,       // RMB4 $14
            0x57, 0x15,       // RMB5 $15
            0x67, 0x16,       // RMB6 $16
            0x77, 0x17,       // RMB7 $17
            0x87, 0x10,       // SMB0 $10
            0x97, 0x11,       // SMB1 $11
            0xA7, 0x12,       // SMB2 $12
            0xB7, 0x13,       // SMB3 $13
            0xC7, 0x14,       // SMB4 $14
            0xD7, 0x15,       // SMB5 $15
            0xE7, 0x16,       // SMB6 $16
            0xF7, 0x17,       // SMB7 $17
         ];
        w8(&mut core, 0x0010, 0xFF);
        w8(&mut core, 0x0011, 0xFF);
        w8(&mut core, 0x0012, 0xFF);
        w8(&mut core, 0x0013, 0xFF);
        w8(&mut core, 0x0014, 0xFF);
        w8(&mut core, 0x0015, 0xFF);
        w8(&mut core, 0x0016, 0xFF);
        w8(&mut core, 0x0017, 0xFF);
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(5 == step(&mut core)); T!(0b1111_1110 == get(&core, 0x0010)); 
        T!(5 == step(&mut core)); T!(0b1111_1101 == get(&core, 0x0011));
        T!(5 == step(&mut core)); T!(0b1111_1011 == get(&core, 0x0012));
        T!(5 == step(&mut core)); T!(0b1111_0111 == get(&core, 0x0013));
        T!(5 == step(&mut core)); T!(0b1110_1111 == get(&core, 0x0014));
        T!(5 == step(&mut core)); T!(0b1101_1111 == get(&core, 0x0015));
        T!(5 == step(&mut core)); T!(0b1011_1111 == get(&core, 0x0016));
        T!(5 == step(&mut core)); T!(0b0111_1111 == get(&core, 0x0017));

        w8(&mut core, 0x0010, 0x00);
        w8(&mut core, 0x0011, 0x00);
        w8(&mut core, 0x0012, 0x00);
        w8(&mut core, 0x0013, 0x00);
        w8(&mut core, 0x0014, 0x00);
        w8(&mut core, 0x0015, 0x00);
        w8(&mut core, 0x0016, 0x00);
        w8(&mut core, 0x0017, 0x00);

        T!(5 == step(&mut core)); T!(0b0000_0001 == get(&core, 0x0010)); 
        T!(5 == step(&mut core)); T!(0b0000_0010 == get(&core, 0x0011));
        T!(5 == step(&mut core)); T!(0b0000_0100 == get(&core, 0x0012));
        T!(5 == step(&mut core)); T!(0b0000_1000 == get(&core, 0x0013));
        T!(5 == step(&mut core)); T!(0b0001_0000 == get(&core, 0x0014));
        T!(5 == step(&mut core)); T!(0b0010_0000 == get(&core, 0x0015));
        T!(5 == step(&mut core)); T!(0b0100_0000 == get(&core, 0x0016));
        T!(5 == step(&mut core)); T!(0b1000_0000 == get(&core, 0x0017));        
    }

    #[test]
    fn bbr() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0x0F, 0x10, 0x06,   // BBR0 $10 nok 
            0x0F, 0x11, 0x02,   // BBR0 $11 -> +$02
            0xEA, 0xA,          // NOP, NOP            
            0x0F, 0x11, 0x10,   // BBR0 $11 -> +$10
        ];
        copy(&mut core, 0x02F0, &prog);
        cpu_prefetch(&mut core, 0x02F0);
    
        w8(&mut core, 0x0010, 0x01);
        w8(&mut core, 0x0011, 0x02);

        T!(5 == step(&mut core)); T!(R!(core, pc) == 0x02F3);
        T!(6 == step(&mut core)); T!(R!(core, pc) == 0x02F8);
        T!(7 == step(&mut core)); T!(R!(core, pc) == 0x030B);        
    }

    #[test]
    fn bbs() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0x8F, 0x10, 0x06,   // BBS0 $10 nok 
            0x8F, 0x11, 0x02,   // BBS0 $11 -> +$02
            0xEA, 0xA,          // NOP, NOP            
            0x8F, 0x11, 0x10,   // BBS0 $11 -> +$11
        ];
        copy(&mut core, 0x02F0, &prog);
        cpu_prefetch(&mut core, 0x02F0);
    
        w8(&mut core, 0x0010, 0x02);
        w8(&mut core, 0x0011, 0x01);

        T!(5 == step(&mut core)); T!(R!(core, pc) == 0x02F3);
        T!(6 == step(&mut core)); T!(R!(core, pc) == 0x02F8);
        T!(7 == step(&mut core)); T!(R!(core, pc) == 0x030B);        
    }

    #[test]
    fn ror_rol() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        // FIXME: more adressing modes
        let prog = [
            0xA9, 0x81,     // LDA #$81
            0xA2, 0x01,     // LDX #$01
            0x85, 0x10,     // STA $10
            0x26, 0x10,     // ROL $10
            0x36, 0x0F,     // ROL $0F,X
            0x76, 0x0F,     // ROR $0F,X
            0x66, 0x10,     // ROR $10
            0x6A,           // ROR
            0x6A,           // ROR
            0x2A,           // ROL
            0x2A,           // ROL
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(0x81 == R!(core, a));
        T!(2 == step(&mut core)); T!(0x01 == R!(core, x));
        T!(3 == step(&mut core)); T!(0x81 == get(&core, 0x0010));
        T!(5 == step(&mut core)); T!(0x02 == get(&core, 0x0010)); T!(tf!(core,M6502Flags::C));
        T!(6 == step(&mut core)); T!(0x05 == get(&core, 0x0010)); T!(tf!(core,M6502Flags::empty()));
        T!(6 == step(&mut core)); T!(0x02 == get(&core, 0x0010)); T!(tf!(core,M6502Flags::C));
        T!(5 == step(&mut core)); T!(0x81 == get(&core, 0x0010)); T!(tf!(core,M6502Flags::N));
        T!(2 == step(&mut core)); T!(0x40 == R!(core, a)); T!(tf!(core,M6502Flags::C));
        T!(2 == step(&mut core)); T!(0xA0 == R!(core, a)); T!(tf!(core,M6502Flags::N));
        T!(2 == step(&mut core)); T!(0x40 == R!(core, a)); T!(tf!(core,M6502Flags::C));
        T!(2 == step(&mut core)); T!(0x81 == R!(core, a)); T!(tf!(core,M6502Flags::N));
    }
    
    #[test]
    fn bit() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA9, 0x00,         // LDA #$00
            0x85, 0x1F,         // STA $1F
            0xA9, 0x80,         // LDA #$80
            0x85, 0x20,         // STA $20
            0xA9, 0xC0,         // LDA #$C0
            0x8D, 0x00, 0x10,   // STA $1000
            0x24, 0x1F,         // BIT $1F
            0x24, 0x20,         // BIT $20
            0x2C, 0x00, 0x10,   // BIT $1000
            0xA9, 0xC7,         // LDA #$C7 
            0x89, 0x80,         // BIT #$80
            0x89, 0xD0,         // BIT #$D0
            0x89, 0x40,         // BIT #$40
            0x89, 0x08,         // BIT #$08
            0xA2, 0x02,         // LDX #$02
            0x3C, 0x1E, 0x00,   // BIT $001E, X
            0x3C, 0xFE, 0x0F,   // BIT $0FFE, X            
            0x34, 0x1E,         // BIT $1E, X
            0x34, 0xFE,         // BIT $1D, X  
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(0x00 == R!(core, a));
        T!(3 == step(&mut core)); T!(0x00 == get(&core, 0x001F));
        T!(2 == step(&mut core)); T!(0x80 == R!(core, a));
        T!(3 == step(&mut core)); T!(0x80 == get(&core, 0x0020));
        T!(2 == step(&mut core)); T!(0xC0 == R!(core, a));
        T!(4 == step(&mut core)); T!(0xC0 == get(&core, 0x1000));
        T!(3 == step(&mut core)); T!(tf!(core,M6502Flags::Z));
        T!(3 == step(&mut core)); T!(tf!(core,M6502Flags::N));
        T!(4 == step(&mut core)); T!(tf!(core,M6502Flags::N|M6502Flags::V));
        T!(2 == step(&mut core)); T!(0xc7 == R!(core, a));
        T!(3 == step(&mut core)); T!(tf!(core,M6502Flags::N));
        T!(3 == step(&mut core)); T!(tf!(core,M6502Flags::N|M6502Flags::V));
        T!(3 == step(&mut core)); T!(tf!(core,M6502Flags::V));
        T!(3 == step(&mut core)); T!(tf!(core,M6502Flags::Z));
        T!(2 == step(&mut core)); T!(0x02 == R!(core, x));
        T!(4 == step(&mut core)); T!(tf!(core,M6502Flags::N));
        T!(4 == step(&mut core)); T!(tf!(core,M6502Flags::N|M6502Flags::V));
        T!(3 == step(&mut core)); T!(tf!(core,M6502Flags::N));
        T!(3 == step(&mut core)); T!(tf!(core,M6502Flags::Z));
    }
    
    #[test]
    fn bne_beq() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA9, 0x10,         // LDA #$10
            0xC9, 0x10,         // CMP #$10
            0xF0, 0x02,         // BEQ eq
            0xA9, 0x0F,         // ne: LDA #$0F
            0xC9, 0x0F,         // eq: CMP #$0F
            0xD0, 0xFA,         // BNE ne -> executed 2x, second time not taken
            0xEA,
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(0x10 == R!(core, a));
        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::Z|M6502Flags::C));
        T!(3 == step(&mut core)); T!(R!(core, pc) == 0x0208);
        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::C));
        T!(3 == step(&mut core)); T!(R!(core, pc) == 0x0206);
        T!(2 == step(&mut core)); T!(0x0F == R!(core, a));
        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::Z|M6502Flags::C));
        T!(2 == step(&mut core)); T!(R!(core, pc) == 0x020C);
    
        // patch jump target, and test jumping across 256 bytes page
        cpu_prefetch(&mut core, 0x0200);
        w8(&mut core, 0x0205, 0xC0);
        T!(2 == step(&mut core)); T!(0x10 == R!(core, a));
        T!(2 == step(&mut core)); T!(tf!(core,M6502Flags::Z|M6502Flags::C));
        T!(4 == step(&mut core)); T!(R!(core, pc) == 0x01C6);
    
        // FIXME: test the other branches
    }
    
    #[test]
    fn jmp() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0x4C, 0x00, 0x10,   // JMP $1000
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
        T!(3 == step(&mut core)); T!(R!(core, pc) == 0x1000);
    }
    
    #[test]
    fn jmp_indirect_samepage() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA9, 0x33,         // LDA #$33
            0x8D, 0x10, 0x21,   // STA $2110
            0xA9, 0x22,         // LDA #$22
            0x8D, 0x11, 0x21,   // STA $2111
            0x6C, 0x10, 0x21,   // JMP ($2110)
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x33);
        T!(4 == step(&mut core)); T!(get(&core, 0x2110) == 0x33);
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x22);
        T!(4 == step(&mut core)); T!(get(&core, 0x2111) == 0x22);
        T!(5 == step(&mut core)); T!(R!(core, pc) == 0x2233);
    }
    
    #[test]
    fn jmp_abs_x_indexed_indirect() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA9, 0x33,         // LDA #$33
            0x8D, 0x10, 0x21,   // STA $2110
            0xA9, 0x22,         // LDA #$22
            0x8D, 0x11, 0x21,   // STA $2111   
            0xA2, 0x10,         // LDX #$10
            0x7C, 0x00, 0x21,   // JMP ($2100, X)
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x33);
        T!(4 == step(&mut core)); T!(get(&core, 0x2110) == 0x33);
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x22);
        T!(4 == step(&mut core)); T!(get(&core, 0x2111) == 0x22);
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x10);
        T!(6 == step(&mut core)); T!(R!(core, pc) == 0x2233);
    }

    #[test]
    fn jmp_abs_x_indexed_indirect_next_page() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA9, 0x33,         // LDA #$33
            0x8D, 0x05, 0x22,   // STA $2205
            0xA9, 0x22,         // LDA #$22
            0x8D, 0x06, 0x22,   // STA $2206   
            0xA2, 0x10,         // LDX #$10
            0x7C, 0xF5, 0x21,   // JMP ($21F5, X)
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x33);
        T!(4 == step(&mut core)); T!(get(&core, 0x2205) == 0x33);
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x22);
        T!(4 == step(&mut core)); T!(get(&core, 0x2206) == 0x22);
        T!(2 == step(&mut core)); T!(R!(core, x) == 0x10);
        T!(6 == step(&mut core)); T!(R!(core, pc) == 0x2233);
    }

    #[test]
    fn jmp_indirect_wrap() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA9, 0x33,         // LDA #$33
            0x8D, 0xFF, 0x21,   // STA $21FF
            0xA9, 0x22,         // LDA #$22
            0x8D, 0x00, 0x21,   // STA $2100    // note: wraps around!
            0x6C, 0xFF, 0x21,   // JMP ($21FF)
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x33);
        T!(4 == step(&mut core)); T!(get(&core, 0x21FF) == 0x33);
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x22);
        T!(4 == step(&mut core)); T!(get(&core, 0x2100) == 0x22);
        T!(5 == step(&mut core)); T!(R!(core, pc) == 0x2233);
    }
    
    #[test]
    fn jsr_rts() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0x20, 0x05, 0x03,   // JSR fun
            0xEA, 0xEA,         // NOP, NOP
            0xEA,               // fun: NOP
            0x60,               // RTS
        ];
        copy(&mut core, 0x0300, &prog);
        cpu_prefetch(&mut core, 0x0300);
    
        T!(R!(core, s) == 0xBD);
        T!(6 == step(&mut core)); T!(R!(core, pc) == 0x0305); T!(R!(core, s) == 0xBB); T!(r16(&core, 0x01BC)==0x0302);
        T!(2 == step(&mut core));
        T!(6 == step(&mut core)); T!(R!(core, pc) == 0x0303); T!(R!(core, s) == 0xBD);
    }
    
    #[test]
    fn rti() {
        let mut core: TestCore = TestCore::default();
        init!(core);
        let prog = [
            0xA9, 0x11,     // LDA #$11
            0x48,           // PHA
            0xA9, 0x22,     // LDA #$22
            0x48,           // PHA
            0xA9, 0x33,     // LDA #$33
            0x48,           // PHA
            0x40,           // RTI
        ];
        copy(&mut core, 0x0200, &prog);
        cpu_prefetch(&mut core, 0x0200);
    
        T!(R!(core, s) == 0xBD);
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x11);
        T!(3 == step(&mut core)); T!(R!(core, s) == 0xBC);
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x22);
        T!(3 == step(&mut core)); T!(R!(core, s) == 0xBB);
        T!(2 == step(&mut core)); T!(R!(core, a) == 0x33);
        T!(3 == step(&mut core)); T!(R!(core, s) == 0xBA);
        T!(6 == step(&mut core)); T!(R!(core, s) == 0xBD); T!(R!(core, pc) == 0x1122); T!(tf!(core,M6502Flags::Z|M6502Flags::C));
    }
}