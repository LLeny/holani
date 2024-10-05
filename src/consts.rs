pub const INTV_ADDR: u16 = 0xFFFE;
pub const RESV_ADDR: u16 = 0xFFFC;
pub const NMIV_ADDR: u16 = 0xFFFA;
pub const MMC_ADDR: u16 = 0xFFF9;
pub const ROM_ADDR: u16 = 0xFE00;
pub const MIK_ADDR: u16 = 0xFD00;
pub const SUZ_ADDR: u16 = 0xFC00;

pub const SUZ_ADDR_B: u16 = SUZ_ADDR - 1;
pub const MIK_ADDR_B: u16 = MIK_ADDR - 1;
pub const ROM_ADDR_B: u16 = ROM_ADDR - 1;
pub const MMC_ADDR_B: u16 = MMC_ADDR - 1;

pub const INTV_ADDR_A: u16 = INTV_ADDR + 1;
pub const RESV_ADDR_A: u16 = RESV_ADDR + 1;
pub const NMIV_ADDR_A: u16 = NMIV_ADDR + 1;

// "The crystal is the only source of timing information in the system. The basic timing tick of the system is 62.5 ns. Let us now define the term tick to be 62.5 ns."
pub const CRYSTAL_FREQ: u32 = 16_000_000;
pub const CRYSTAL_TICK_LENGTH: f32 = 1.0 / (CRYSTAL_FREQ as f32); // 62.5ns

/* "
The requirement for using a page mode cycle is that the current access is in the same 256 address page of memory as the previous access.
The CPU makes use of the page mode circuitry in its op-code reads.
All writes and all data reads are done in normal memory cycles.
A page mode op-code read takes 4 ticks, a normal read or write to RAM takes 5 ticks."
" */
pub const RAM_NORMAL_READ_TICKS: i8 = 5-1;
pub const RAM_NORMAL_WRITE_TICKS: i8 = 5-1;
pub const RAM_PAGE_READ_TICKS: i8 = 4-1;
pub const RAM_DMA_READ_TICKS: i8 = 3-1;
pub const RAM_REFRESH_TICKS: u8 = 4;

pub const RAM_PEEK_DATA_OPCODE: u8 = 0b00000001;
pub const RAM_PEEK_DATA_DMA:    u8 = 0b00000010;

pub const MIKEY_TIMER_READ_TICKS: u16 = 5-1;
pub const MIKEY_TIMER_WRITE_TICKS: u16 = 5-1;
pub const MIKEY_READ_TICKS: u16 = 5-1;
pub const MIKEY_WRITE_TICKS: u16 = 5-1;

/*
Cycle                              Min       Max
---------------------------------------------------
Suzy Hardware(write)               5          5
Suzy Hardware(read)                9         15
*/

pub const SUZY_WRITE_TICKS: u16 = 5-1;
pub const SUZY_READ_TICKS: u16 = 12-1; // ~~

/* "
Multiplies with out sign or accumulate take 44 ticks to complete.
Multiplies with sign and accumulate take 54 ticks to complete. 
" */
pub const SUZY_MULT_SIGN_TICKS: u16 = 54-1;
pub const SUZY_MULT_NON_SIGN_TICKS: u16 = 44-1;

// "The CPU cycle that performed the actual read uses 15 ticks of the clock."
pub const CART_READ_TICKS: u8 = 15-1;
// "This is a blind write from the CPU and must not be interrupted by another access to Suzy until it is finished."
pub const CART_WRITE_TICKS: u8 = SUZY_WRITE_TICKS as u8; 

pub const M6502_PIN_RW: u8 = 24;
pub const M6502_PIN_SYNC: u8 = 25;
pub const M6502_PIN_IRQ: u8 = 26;
pub const M6502_PIN_NMI: u8 = 27;
pub const M6502_PIN_RDY: u8 = 28;
pub const M6502_PIN_RES: u8 = 30;

pub const M6502_RW: u32 = 1 << M6502_PIN_RW;
pub const M6502_SYNC: u32 = 1 << M6502_PIN_SYNC;
pub const M6502_IRQ: u32 = 1 << M6502_PIN_IRQ;
pub const M6502_NMI: u32 = 1 << M6502_PIN_NMI;
pub const M6502_RDY: u32 = 1 << M6502_PIN_RDY;
pub const M6502_RES: u32 = 1 << M6502_PIN_RES;

pub const MAPCTL_VEC_BIT: u8 = 0b00001000;
pub const MAPCTL_ROM_BIT: u8 = 0b00000100;
pub const MAPCTL_MIK_BIT: u8 = 0b00000010;
pub const MAPCTL_SUZ_BIT: u8 = 0b00000001;

pub const TIM0BKUP: u16 = 0xfd00;
pub const TIM0CTLA: u16 = 0xfd01;
pub const TIM0CNT: u16 = 0xfd02;
pub const TIM0CTLB: u16 = 0xfd03;
pub const TIM1BKUP: u16 = 0xfd04;
pub const TIM1CTLA: u16 = 0xfd05;
pub const TIM1CNT: u16 = 0xfd06;
pub const TIM1CTLB: u16 = 0xfd07;
pub const TIM2BKUP: u16 = 0xfd08;
pub const TIM2CTLA: u16 = 0xfd09;
pub const TIM2CNT: u16 = 0xfd0a;
pub const TIM2CTLB: u16 = 0xfd0b;
pub const TIM3BKUP: u16 = 0xfd0c;
pub const TIM3CTLA: u16 = 0xfd0d;
pub const TIM3CNT: u16 = 0xfd0e;
pub const TIM3CTLB: u16 = 0xfd0f;
pub const TIM4BKUP: u16 = 0xfd10;
pub const TIM4CTLA: u16 = 0xfd11;
pub const TIM4CNT: u16 = 0xfd12;
pub const TIM4CTLB: u16 = 0xfd13;
pub const TIM5BKUP: u16 = 0xfd14;
pub const TIM5CTLA: u16 = 0xfd15;
pub const TIM5CNT: u16 = 0xfd16;
pub const TIM5CTLB: u16 = 0xfd17;
pub const TIM6BKUP: u16 = 0xfd18;
pub const TIM6CTLA: u16 = 0xfd19;
pub const TIM6CNT: u16 = 0xfd1a;
pub const TIM6CTLB: u16 = 0xfd1b;
pub const TIM7BKUP: u16 = 0xfd1c;
pub const TIM7CTLA: u16 = 0xfd1d;
pub const TIM7CNT: u16 = 0xfd1e;
pub const TIM7CTLB: u16 = 0xfd1f;
pub const AUD0VOL: u16 = 0xfd20;
pub const AUD0SHFTFB: u16 = 0xfd21;
pub const AUD0OUTVAL: u16 = 0xfd22;
pub const AUD0L8SHFT: u16 = 0xfd23;
pub const AUD0TBACK: u16 = 0xfd24;
pub const AUD0CTL: u16 = 0xfd25;
pub const AUD0COUNT: u16 = 0xfd26;
pub const AUD0MISC: u16 = 0xfd27;
pub const AUD1VOL: u16 = 0xfd28;
pub const AUD1SHFTFB: u16 = 0xfd29;
pub const AUD1OUTVAL: u16 = 0xfd2a;
pub const AUD1L8SHFT: u16 = 0xfd2b;
pub const AUD1TBACK: u16 = 0xfd2c;
pub const AUD1CTL: u16 = 0xfd2d;
pub const AUD1COUNT: u16 = 0xfd2e;
pub const AUD1MISC: u16 = 0xfd2f;
pub const AUD2VOL: u16 = 0xfd30;
pub const AUD2SHFTFB: u16 = 0xfd31;
pub const AUD2OUTVAL: u16 = 0xfd32;
pub const AUD2L8SHFT: u16 = 0xfd33;
pub const AUD2TBACK: u16 = 0xfd34;
pub const AUD2CTL: u16 = 0xfd35;
pub const AUD2COUNT: u16 = 0xfd36;
pub const AUD2MISC: u16 = 0xfd37;
pub const AUD3VOL: u16 = 0xfd38;
pub const AUD3SHFTFB: u16 = 0xfd39;
pub const AUD3OUTVAL: u16 = 0xfd3a;
pub const AUD3L8SHFT: u16 = 0xfd3b;
pub const AUD3TBACK: u16 = 0xfd3c;
pub const AUD3CTL: u16 = 0xfd3d;
pub const AUD3COUNT: u16 = 0xfd3e;
pub const AUD3MISC: u16 = 0xfd3f;
pub const ATTEN_A: u16 = 0xFD40;
pub const ATTEN_B: u16 =  0xFD41;
pub const ATTEN_C: u16 =  0xFD42;
pub const ATTEN_D: u16 =  0xFD43;
pub const MPAN: u16 =  0xFD44;
pub const MSTEREO: u16 = 0xfd50;
pub const INTRST: u16 = 0xfd80;
pub const INTSET: u16 = 0xfd81;
pub const MAGRDY0: u16 = 0xfd84;
pub const MAGRDY1: u16 = 0xfd85;
pub const AUDIN: u16 = 0xfd86;
pub const SYSCTL1: u16 = 0xfd87;
pub const MIKEYHREV: u16 = 0xfd88;
pub const MIKEYSREV: u16 = 0xfd89;
pub const IODIR: u16 = 0xfd8a;
pub const IODAT: u16 = 0xfd8b;
pub const SERCTL: u16 = 0xfd8c;
pub const SERDAT: u16 = 0xfd8d;
pub const SDONEACK: u16 = 0xfd90;
pub const CPUSLEEP: u16 = 0xfd91;
pub const DISPCTL: u16 = 0xfd92;
pub const PBKUP: u16 = 0xfd93;
pub const DISPADR: u16 = 0xfd94;
pub const DISPADRL: u16 = 0xfd94;
pub const DISPADRH: u16 = 0xfd95;
pub const MTEST0: u16 = 0xfd9c;
pub const MTEST1: u16 = 0xfd9d;
pub const MTEST2: u16 = 0xfd9e;
pub const GREEN0: u16 = 0xfda0;
pub const GREEN1: u16 = 0xfda1;
pub const GREEN2: u16 = 0xfda2;
pub const GREEN3: u16 = 0xfda3;
pub const GREEN4: u16 = 0xfda4;
pub const GREEN5: u16 = 0xfda5;
pub const GREEN6: u16 = 0xfda6;
pub const GREEN7: u16 = 0xfda7;
pub const GREEN8: u16 = 0xfda8;
pub const GREEN9: u16 = 0xfda9;
pub const GREENA: u16 = 0xfdaa;
pub const GREENB: u16 = 0xfdab;
pub const GREENC: u16 = 0xfdac;
pub const GREEND: u16 = 0xfdad;
pub const GREENE: u16 = 0xfdae;
pub const GREENF: u16 = 0xfdaf;
pub const BLUERED0: u16 = 0xfdb0;
pub const BLUERED1: u16 = 0xfdb1;
pub const BLUERED2: u16 = 0xfdb2;
pub const BLUERED3: u16 = 0xfdb3;
pub const BLUERED4: u16 = 0xfdb4;
pub const BLUERED5: u16 = 0xfdb5;
pub const BLUERED6: u16 = 0xfdb6;
pub const BLUERED7: u16 = 0xfdb7;
pub const BLUERED8: u16 = 0xfdb8;
pub const BLUERED9: u16 = 0xfdb9;
pub const BLUEREDA: u16 = 0xfdba;
pub const BLUEREDB: u16 = 0xfdbb;
pub const BLUEREDC: u16 = 0xfdbc;
pub const BLUEREDD: u16 = 0xfdbd;
pub const BLUEREDE: u16 = 0xfdbe;
pub const BLUEREDF: u16 = 0xfdbf;

pub const IODAT_CAD: u8 = 0b00000010;
pub const IODAT_AUDIN: u8 = 0b00010000;
pub const SYSCTL1_CAS: u8 = 0b00000001;
pub const SYSCTL1_POWER: u8 = 0b00000010;

pub const INT_TIMER0: u8 = 0b00000001;
pub const INT_TIMER2: u8 = 0b00000100;
pub const INT_TIMER4: u8 = 0b00010000;

pub const CART_PIN_D3: u32 = 1;
pub const CART_PIN_D2: u32 = 2;
pub const CART_PIN_D4: u32 = 3;
pub const CART_PIN_D1: u32 = 4;
pub const CART_PIN_D5: u32 = 5;
pub const CART_PIN_D0: u32 = 6;
pub const CART_PIN_D6: u32 = 7;
pub const CART_PIN_D7: u32 = 8;
pub const CART_PIN_CE: u32 = 9;
pub const CART_PIN_A1: u32 = 10;
pub const CART_PIN_A2: u32 = 11;
pub const CART_PIN_A3: u32 = 12;
pub const CART_PIN_A6: u32 = 13;
pub const CART_PIN_A4: u32 = 14;
pub const CART_PIN_A5: u32 = 15;
pub const CART_PIN_A0: u32 = 16;
pub const CART_PIN_A7: u32 = 17;
pub const CART_PIN_A16: u32 = 18;
pub const CART_PIN_A17: u32 = 19;
pub const CART_PIN_A18: u32 = 20;
pub const CART_PIN_A19: u32 = 21;
pub const CART_PIN_A15: u32 = 22;
pub const CART_PIN_A14: u32 = 23;
pub const CART_PIN_A13: u32 = 24;
pub const CART_PIN_A12: u32 = 25;
pub const CART_PIN_WE: u32 = 26;
pub const CART_PIN_A8: u32 = 27;
pub const CART_PIN_A9: u32 = 28;
pub const CART_PIN_A10: u32 = 29;
pub const CART_PIN_AUDIN: u32 = 31;

pub const TMPADRL: u16 = 0xFC00; // "Temporary address" Low byte
pub const TMPADRH: u16 = 0xFC01; // "Temporary address" High byte
pub const TILTACUML: u16 = 0xFC02; // "Accumulator for tilt value" Low byte
pub const TILTACUMH: u16 = 0xFC03; // "Accumulator for tilt value" High byte
pub const HOFFL: u16 = 0xFC04; // "Offset to H edge of screen" Low byte
pub const HOFFH: u16 = 0xFC05; // "Offset to H edge of screen" High byte
pub const VOFFL: u16 = 0xFC06; // "Offset to V edge of screen" Low byte
pub const VOFFH: u16 = 0xFC07; // "Offset to V edge of screen" High byte
pub const VIDBASL: u16 = 0xFC08; // "Base Address of Video Build Buffer" Low byte
pub const VIDBASH: u16 = 0xFC09; // "Base Address of Video Build Buffer" High byte
pub const COLLBASL: u16 = 0xFC0A; // "Base Address of Coll Build Buffer" Low byte
pub const COLLBASH: u16 = 0xFC0B; // "Base Address of Coll Build Buffer" High byte
pub const VIDADRL: u16 = 0xFC0C; // "Current Video Build Address" Low byte
pub const VIDADRH: u16 = 0xFC0D; // "Current Video Build Address" High byte
pub const COLLADRL: u16 = 0xFC0E; // "Current Collision Build Address" Low byte
pub const COLLADRH: u16 = 0xFC0F; // "Current Collision Build Address" High byte
pub const SCBNEXTL: u16 = 0xFC10; // "Address of Next SCB" Low byte
pub const SCBNEXTH: u16 = 0xFC11; // "Address of Next SCB" High byte
pub const SPRDLINEL: u16 = 0xFC12; // "Start of Sprite Data Line Address" Low byte
pub const SPRDLINEH: u16 = 0xFC13; // "Start of Sprite Data Line Address" High byte
pub const HPOSSTRTL: u16 = 0xFC14; // "Starting Hpos" Low byte
pub const HPOSSTRTH: u16 = 0xFC15; // "Starting Hpos" High byte
pub const VPOSSTRTL: u16 = 0xFC16; // "Starting Vpos" Low byte
pub const VPOSSTRTH: u16 = 0xFC17; // "Starting Vpos" High byte
pub const SPRHSIZL: u16 = 0xFC18; // "H Size" Low byte
pub const SPRHSIZH: u16 = 0xFC19; // "H Size" High byte
pub const SPRVSIZL: u16 = 0xFC1A; // "V Size" Low byte
pub const SPRVSIZH: u16 = 0xFC1B; // "V Size" High byte
pub const STRETCHL: u16 = 0xFC1C; // "H Size Adder" Low byte
pub const STRETCHH: u16 = 0xFC1D; // "H Size Adder" High byte
pub const TILTL: u16 = 0xFC1E; // "H Position Adder" Low byte
pub const TILTH: u16 = 0xFC1F; // "H Position Adder" High byte
pub const SPRDOFFL: u16 = 0xFC20; // "Offset to Next Sprite Data Line" Low byte
pub const SPRDOFFH: u16 = 0xFC21; // "Offset to Next Sprite Data Line" High byte
pub const SPRVPOSL: u16 = 0xFC22; // "Current Vpos" Low byte
pub const SPRVPOSH: u16 = 0xFC23; // "Current Vpos" High byte
pub const COLLOFFL: u16 = 0xFC24; // "Offset to Collision Depository" Low byte
pub const COLLOFFH: u16 = 0xFC25; // "Offset to Collision Depository" High byte
pub const VSIZACUML: u16 = 0xFC26; // "Vertical Size Accumulator" Low byte
pub const VSIZACUMH: u16 = 0xFC27; // "Vertical Size Accumulator" High byte
pub const HSIZOFFL: u16 = 0xFC28; // "Horizontal Size Offset" Low byte
pub const HSIZOFFH: u16 = 0xFC29; // "Horizontal Size Offset" High byte
pub const VSIZOFFL: u16 = 0xFC2A; // "Vertical Size Offset" Low byte
pub const VSIZOFFH: u16 = 0xFC2B; // "Vertical Size Offset" High byte
pub const SCBADRL: u16 = 0xFC2C; // "Address of Current SCB" Low byte
pub const SCBADRH: u16 = 0xFC2D; // "Address of Current SCB" High byte
pub const PROCADRL: u16 = 0xFC2E; // "Current Spr Data Proc Address" Low byte
pub const PROCADRH: u16 = 0xFC2F; // "Current Spr Data Proc Address" High byte
pub const MATHD: u16 = 0xFC52;
pub const MATHC: u16 = 0xFC53;
pub const MATHB: u16 = 0xFC54;
pub const MATHA: u16 = 0xFC55;
pub const MATHP: u16 = 0xFC56;
pub const MATHN: u16 = 0xFC57;
pub const MATHH: u16 = 0xFC60;
pub const MATHG: u16 = 0xFC61;
pub const MATHF: u16 = 0xFC62;
pub const MATHE: u16 = 0xFC63;
pub const MATHM: u16 = 0xFC6C;
pub const MATHL: u16 = 0xFC6D;
pub const MATHK: u16 = 0xFC6E;
pub const MATHJ: u16 = 0xFC6F;
pub const SPRCTL0: u16 = 0xFC80; // "FC80 = SPRCTL0 Sprite Control Bits 0 (W)"
pub const SPRCTL1: u16 = 0xFC81; // "FC81 = SPRCTL1 Sprite Control Bits 1 (W)(U)"
pub const SPRCOLL: u16 = 0xFC82; // "FC82 = SPRCOLL. Sprite Collision Number (W)"
pub const SPRINIT: u16 = 0xFC83; // "Sprite Initialization Bits (W)(U)"
pub const SUZYBUSEN: u16 = 0xFC90; // "FC90 = SUZYBUSEN. Suzy Bus Enable (W)"
pub const SPRGO: u16 = 0xFC91; // "FC91 = SPRG0. Sprite Process Start Bit (W)"
pub const SPRSYS: u16 = 0xFC92; // "FC92 = SPRSYS. System Control Bits (R/W)"
pub const SUZYHREV: u16 = 0xFC88; // Suzy Hardware Revision (R)
pub const JOYSTICK: u16 = 0xFCB0; // "Read Joystick and Switches (R)"
pub const SWITCHES: u16 = 0xFCB1; // "Read Other Switches (R)"
pub const RCART0: u16 = 0xFCB2; // RCART(R/W)
pub const RCART1: u16 = 0xFCB3; // RCART(R/W)

pub const SPRSYS_SIGN_MATH: u8 = 0b10000000;
pub const SPRSYS_ACCUMULATE: u8 = 0b01000000;
pub const SPRSYS_DONT_COLLIDE: u8 = 0b00100000;
pub const SPRSYS_VSTRETCH: u8 = 0b00010000;
pub const SPRSYS_LEFTHAND: u8 = 0b00001000;
pub const SPRSYS_CLEAR_UNSAFE: u8 = 0b00000100;
pub const SPRSYS_STOP_CURRENT_SPRITE: u8 = 0b00000010;

pub const SPRSYS_MATH_IN_PROGRESS: u8 = 0b10000000;
pub const SPRSYS_MATHBIT: u8 = 0b01000000;
pub const SPRSYS_LAST_CARRY: u8 = 0b00100000;
pub const SPRSYS_UNSAFE_ACCESS: u8 = 0b00000100;
pub const SPRSYS_SPRITE_IN_PROGRESS: u8 = 0b00000001;

pub const SPRCTL1_LITERAL           : u8 = 0b10000000;
pub const SPRCTL1_ALGO_3            : u8 = 0b01000000;
pub const SPRCTL1_RELOAD_HVST       : u8 = 0b00110000;
pub const SPRCTL1_RELOAD_HVS        : u8 = 0b00100000;
pub const SPRCTL1_RELOAD_HV         : u8 = 0b00010000;
pub const SPRCTL1_REUSE_PALETTE     : u8 = 0b00001000;
pub const SPRCTL1_SKIP_SPRITE       : u8 = 0b00000100;
pub const SPRCTL1_DRAW_UP           : u8 = 0b00000010;
pub const SPRCTL1_DRAW_LEFT         : u8 = 0b00000001;
pub const SPRCTL1_DRAW_QUAD         : u8 = 0b00000011;

pub const SPRCTL0_BPP               : u8 = 0b11000000;
pub const SPRCTL0_HFLIP             : u8 = 0b00100000;
pub const SPRCTL0_VFLIP             : u8 = 0b00010000;
pub const SPRCTL0_SPR_TYPE          : u8 = 0b00000111;

pub const SPRCOLL_DONT_COLLIDE      : u8 = 0b00100000;
pub const SPRCOLL_NUMBER            : u8 = 0b00001111;

pub const SPRGO_GO                  : u8 = 0b00000001;
pub const SPRGO_EVERON              : u8 = 0b00000100;

pub const R_SPRCTL0    : u16 = 0;
pub const R_SPRCTL1    : u16 = 1;
pub const R_SPRCOLL    : u16 = 2;
pub const R_SCBNEXTL   : u16 = 3;
pub const R_SCBNEXTH   : u16 = 4;
pub const R_SPRDATAL   : u16 = 5;
pub const R_SPRDATAH   : u16 = 6;
pub const R_HPOSL      : u16 = 7;
pub const R_HPOSH      : u16 = 8;
pub const R_VPOSL      : u16 = 9;
pub const R_VPOSH      : u16 = 10;
pub const R_HSIZEL     : u16 = 11;
pub const R_HSIZEH     : u16 = 12;
pub const R_VSIZEL     : u16 = 13;
pub const R_VSIZEH     : u16 = 14;
pub const R_STRETCHL   : u16 = 15;
pub const R_STRETCHH   : u16 = 16;
pub const R_TILTL      : u16 = 17;
pub const R_TILTH      : u16 = 18;
pub const R_PALETTE_00 : u16 = 19;
pub const R_PALETTE_01 : u16 = 20;
pub const R_PALETTE_02 : u16 = 21;
pub const R_PALETTE_03 : u16 = 22;
pub const R_PALETTE_04 : u16 = 23;
pub const R_PALETTE_05 : u16 = 24;
pub const R_PALETTE_06 : u16 = 25;
pub const R_PALETTE_07 : u16 = 26;

pub const LINE_END : u32 = 0x80;
