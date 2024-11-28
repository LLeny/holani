use bitflags::bitflags;
use super::*;

bitflags! {
    #[derive(Copy, Clone, Debug, Serialize, Deserialize)]
    pub struct SprSysR:u8
    {
        const math_working   = 0b10000000;
        const math_warning   = 0b01000000;
        const math_carry     = 0b00100000;
        const v_stretching   = 0b00010000;
        const left_handed    = 0b00001000;
        const unsafe_acces   = 0b00000100;
        const sprite_to_stop = 0b00000010;
        const sprite_working = 0b00000001;
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug, Serialize, Deserialize)]
    pub struct SprSysW:u8
    {
        const sign_math      = 0b10000000;
        const accumulate     = 0b01000000;
        const no_collide     = 0b00100000;
        const v_stretching   = 0b00010000;
        const left_handed    = 0b00001000;
        const clear_unsafe   = 0b00000100;
        const sprite_to_stop = 0b00000010;
        const no_effect      = 0b00000001;
    }
}

bitflags! {
    #[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Joystick:u8
    {
        const up       = 0b01000000;
        const down     = 0b10000000;
        const left     = 0b00010000;
        const right    = 0b00100000;
        const option_1 = 0b00001000;
        const option_2 = 0b00000100;
        const inside   = 0b00000010;
        const outside  = 0b00000001;
    }
}

bitflags! {
    #[derive(Copy, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Switches:u8
    {
        const cart1_inactive = 0b00000100;
        const cart0_inactive = 0b00000010;
        const pause          = 0b00000001;
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TaskStep {
    None = 0,
    InitializePainting,
    LoadSCB,
    InitializeQuadrants,
    InitializeQuadrant,
    RenderLinesStart,
    RenderPixelHeightStart,
    RenderPixelsInLine,
    RenderPixelheightEnd,
    RenderLinesEnd,
    NextQuadrant,
    SpriteEnd,
    MaxSteps,
}

impl TaskStep {
    pub const ZERO: TaskStep = TaskStep::None;
    pub const ONE: TaskStep = TaskStep::InitializePainting;
}

impl core::ops::Add<u8> for TaskStep {
    type Output = Self;
    fn add(self, rhs: u8) -> Self::Output {
        let mut s = self as u8;
        s += rhs;
        s %= TaskStep::MaxSteps as u8;
        unsafe { core::mem::transmute(s) }
    }
}

pub fn joystick_swap(mut j: Joystick, b1: Joystick, b2: Joystick) -> Joystick {
    let b1_set = j.contains(b1);
    j.set(b1, j.contains(b2));
    j.set(b2, b1_set);
    j
}

#[derive(Serialize, Deserialize)]
pub struct SuzyRegisters {
    data: Vec<u8>,
    ir_ticks_delay: u16,
    task_ticks_delay: u16,
    sprsys_r: SprSysR,
    sprsys_w: SprSysW,
    sign_ab: i8,
    sign_cd: i8,
    tmp_cd: u16,
    tmp_sign_cd: i8,
    ir: SuzyInstruction,
    addr_r: u16,
    data_r: u16,
    task: SuzyTask,
    task_step: TaskStep
}

impl SuzyRegisters {
    pub fn new() -> Self {
        let mut r = Self {
            data: vec![0; 0x100],      
            ir_ticks_delay: 0,      
            task_ticks_delay: 0,     
            sprsys_r: SprSysR::empty(),
            sprsys_w: SprSysW::empty(),
            sign_ab: 0,
            sign_cd: 0,
            ir: SuzyInstruction::None,
            tmp_cd: 0,
            tmp_sign_cd: 0,
            addr_r: 0,
            data_r: 0,
            task: SuzyTask::None,
            task_step: TaskStep::None,
        };
        r.set_data(SUZYHREV, 1); //SUZYHREV hardware version (always 1.0 for hardware)
        r.set_abcd(0xffffffff);
        r.set_efgh(0xffffffff);
        r.set_jklm(0xffffffff);
        r.set_np(0xffff);
        r.set_sign_ab(1);
        r.set_sign_cd(1);
        r.set_data(HSIZOFFL, 0x7f);
        r.set_data(VSIZOFFL, 0x7f);
        r.set_data(SWITCHES, 0b110);
        r
    }

    pub fn data(&self, addr: u16) -> u8 {
        self.data[(addr - SUZ_ADDR) as usize]
    }

    pub fn set_data(&mut self, addr: u16, data: u8) {
        self.data[(addr - SUZ_ADDR) as usize] = data;
    }

    pub fn u16(&self, addr: u16) -> u16 {
        self.data(addr) as u16 | ((self.data(addr+1) as u16) << 8)
    }

    pub fn i16(&self, addr: u16) -> i16 {
        (self.data(addr) as u16 | ((self.data(addr+1) as u16) << 8)) as i16
    }

    pub fn u32(&self, addr: u16) -> u32 {
        self.data(addr) as u32 | ((self.data(addr+1) as u32) << 8) | ((self.data(addr+2) as u32) << 16) | ((self.data(addr+3) as u32) << 24)
    }

    pub fn set_u16(&mut self, addr: u16, data: u16) {
        self.set_data(addr,   (data & 0xff) as u8);   
        self.set_data(addr+1, ((data & 0xff00) >> 8) as u8);
    }

    pub fn set_i16(&mut self, addr: u16, data: i16) {
        self.set_data(addr,   (data & 0xff) as u8);   
        self.set_data(addr+1, (((data as u16) & 0xff00) >> 8) as u8);
    }

    pub fn set_u32(&mut self, addr: u16, data: u32) {
        self.set_data(addr,   (data & 0xff) as u8);   
        self.set_data(addr+1, ((data & 0xff00) >> 8) as u8);
        self.set_data(addr+2, ((data & 0xff0000) >> 16) as u8);
        self.set_data(addr+3, ((data & 0xff000000) >> 24) as u8);
    }

    pub fn efgh(&self) -> u32 {
        self.u32(MATHH)
    }

    pub fn jklm(&self) -> u32 {
        self.u32(MATHM)
    }

    pub fn abcd(&self) -> u32 {
        self.u32(MATHD)
    }

    pub fn np(&self) -> u16 {
        self.u16(MATHP)
    }

    pub fn ab(&self) -> u16 {
        self.u16(MATHB)
    }

    pub fn cd(&self) -> u16 {
        self.u16(MATHD)
    }

    pub fn set_ab(&mut self, v: u16) {
        self.set_u16(MATHB, v)
    }

    pub fn set_cd(&mut self, v: u16) {
        self.set_u16(MATHD, v)
    }

    pub fn set_abcd(&mut self, v: u32) {
        self.set_u32(MATHD, v);
    }

    pub fn set_efgh(&mut self, v: u32) {
        self.set_u32(MATHH, v);
    }

    pub fn set_jklm(&mut self, v: u32) {
        self.set_u32(MATHM, v);
    }

    pub fn set_np(&mut self, v: u16) {
        self.set_u16(MATHP, v);
    }

    pub fn sprsys(&self) -> u8 {
        self.sprsys_r.bits()
    }

    pub fn set_joystick(&mut self, joy: Joystick) {
        self.set_data(JOYSTICK, joy.bits());
    }

    pub fn set_switches(&mut self, sw: Switches) {
        self.set_data(SWITCHES, sw.bits());
    }

    pub fn joystick(&self) -> Joystick {
        match Joystick::from_bits(self.data[(JOYSTICK - SUZ_ADDR) as usize]) {
            None => Joystick::empty(),
            Some(v) => v
        }
    }

    pub fn switches(&self) -> Switches {
        match Switches::from_bits(self.data[(SWITCHES - SUZ_ADDR) as usize]) {
            None => Switches::empty(),
            Some(v) => v
        }
    }

    pub fn set_sprsys(&mut self, v: u8) {
        self.sprsys_w = match SprSysW::from_bits(v) {
            Some(bits) => bits,
            None => SprSysW::empty()
        };
        if self.sprsys_w_is_flag_set(SprSysW::v_stretching) {
            self.sprsys_r_enable_flag(SprSysR::v_stretching)
        }
        if self.sprsys_w_is_flag_set(SprSysW::left_handed) {
            self.sprsys_r_enable_flag(SprSysR::left_handed)
        }
        if self.sprsys_w_is_flag_set(SprSysW::sprite_to_stop) {
            self.sprsys_r_enable_flag(SprSysR::sprite_to_stop)
        }
        if self.sprsys_w_is_flag_set(SprSysW::clear_unsafe) {
            self.sprsys_r_disable_flag(SprSysR::unsafe_acces)
        }
    }

    pub fn sprsys_r_enable_flag(&mut self, flag: SprSysR) {
        self.sprsys_r.set(flag, true);
    }

    pub fn sprsys_r_disable_flag(&mut self, flag: SprSysR) {
        self.sprsys_r.set(flag, false);
    }

    pub fn sprsys_r_is_flag_set(&self, flag: SprSysR) -> bool {
        self.sprsys_r.contains(flag)
    }

    pub fn sprsys_w_enable_flag(&mut self, flag: SprSysW) {
        self.sprsys_w.set(flag, true);
    }

    pub fn sprsys_w_disable_flag(&mut self, flag: SprSysW) {
        self.sprsys_w.set(flag, false);
    }

    pub fn sprsys_w_is_flag_set(&self, flag: SprSysW) -> bool {
        self.sprsys_w.contains(flag)
    }

    pub fn sprctl0(&self) -> u8 {
        self.data(SPRCTL0)
    }

    pub fn bpp(&self) -> u8 {
        (self.data(SPRCTL0) & SPRCTL0_BPP) >> 6
    }

    pub fn sprctl1(&self) -> u8 {
        self.data(SPRCTL1)
    }

    pub fn start_quadrant(&self) -> u8 {
        static ORDER: [u8;4] = [0, 3, 1, 2];
        ORDER[(self.sprctl1() & SPRCTL1_DRAW_QUAD) as usize]
    }

    pub fn sprcoll(&self) -> u8 {
        self.data(SPRCOLL)
    }

    pub fn sbc_next(&self) -> u16 {
        self.u16(SCBNEXTL)
    }

    pub fn sprdline(&self) -> u16 {
        self.u16(SPRDLINEL)
    }    

    pub fn inc_sprdline(&mut self) {
        let (v, _) = self.u16(SPRDLINEL).overflowing_add(1);
        self.set_u16(SPRDLINEL, v);
    }

    pub fn set_scb_addr(&mut self, v: u16) {
        self.set_u16(SCBADRL, v);
    }

    pub fn set_proc_addr(&mut self, v: u16) {
        self.set_u16(PROCADRL, v);
    }

    pub fn set_tiltacum(&mut self, v: u16) {
        self.set_u16(TILTACUML, v);
    }

    pub fn scb_addr(&self) -> u16 {
        self.u16(SCBADRL)
    }

    pub fn vid_addr(&self) -> u16 {
        self.u16(VIDADRL)
    }

    pub fn ir_ticks_delay(&self) -> u16 {
        self.ir_ticks_delay
    }
    
    pub fn set_ir_ticks_delay(&mut self, ticks_delay: u16) {
        self.ir_ticks_delay = ticks_delay;
    }
    
    pub fn dec_ir_ticks_delay(&mut self) {
        self.ir_ticks_delay -= 1;
    }

    pub fn task_ticks_delay(&self) -> u16 {
        self.task_ticks_delay
    }
    
    pub fn set_task_ticks_delay(&mut self, ticks_delay: u16) {
        self.task_ticks_delay = ticks_delay;
    }
    
    pub fn dec_task_ticks_delay(&mut self) {
        self.task_ticks_delay -= 1;
    }

    pub fn data_r(&self) -> u16 {
        self.data_r
    }
    
    pub fn set_data_r(&mut self, data_r: u16) {
        self.data_r = data_r;
    }
    
    pub fn addr_r(&self) -> u16 {
        self.addr_r
    }
    
    pub fn set_addr_r(&mut self, addr_r: u16) {
        self.addr_r = addr_r;
    }
    
    pub fn task(&self) -> SuzyTask {
        self.task
    } 
    pub fn set_task(&mut self, t: SuzyTask) {
        self.task = t;
    }

    pub fn ir(&self) -> SuzyInstruction {
        self.ir
    }
    
    pub fn set_ir(&mut self, ir: SuzyInstruction) {
        self.ir = ir;
    }

    pub fn reset_ir(&mut self) {
        self.ir = SuzyInstruction::None;
    }

    pub fn reset_task(&mut self) {
        self.task_step = TaskStep::None;
        self.task = SuzyTask::None;
    }
    
    pub fn sign_ab(&self) -> i8 {
        self.sign_ab
    }
    
    pub fn set_sign_ab(&mut self, sign_ab: i8) {
        self.sign_ab = sign_ab;
    }
    
    pub fn sign_cd(&self) -> i8 {
        self.sign_cd
    }
    
    pub fn set_sign_cd(&mut self, sign_cd: i8) {
        self.sign_cd = sign_cd;
    }
    
    pub fn task_step(&self) -> TaskStep {
        self.task_step
    }
    
    pub fn set_task_step(&mut self, step: TaskStep) {
        self.task_step = step;
    }

    pub fn inc_task_step(&mut self) {
        self.task_step = self.task_step + 1;
    }

    pub fn scb_peek_sprite_data(&mut self) {
        self.set_task(SuzyTask::SpriteDataPeek); 
        self.set_task_ticks_delay(RAM_DMA_READ_TICKS as u16 * SUZY_DATA_BUFFER_LEN); 
    }
    
    pub fn tmp_cd(&self) -> u16 {
        self.tmp_cd
    }
    
    pub fn backup_cd(&mut self) {
        self.tmp_cd = self.cd();
        self.tmp_sign_cd = self.sign_cd();
    }
    
    pub fn tmp_sign_cd(&self) -> i8 {
        self.tmp_sign_cd
    }
}

impl Default for SuzyRegisters {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    struct TestCore {
        regs: SuzyRegisters,
    }

    impl Default for TestCore {
        fn default() -> Self {
            Self { regs: SuzyRegisters::new() }
        }
    }

    macro_rules! T {
        ($b: expr) => {
            assert!($b);
        }
    }

    macro_rules! MULT {
        ($c: expr) => {
            $c.regs.backup_cd();
            multiply(&mut $c.regs)
        }
    }

    macro_rules! DIV {
        ($c: expr) => {
            divide(&mut $c.regs)
        }
    }

    macro_rules! SIGNED {
        ($c: ident) => {
            $c.regs.sprsys_w_enable_flag(SprSysW::sign_math)
        }
    }

    macro_rules! ACC {
        ($c: expr) => {
            $c.regs.sprsys_w_enable_flag(SprSysW::accumulate)
        }
    }

    macro_rules! CY {
        ($c: expr) => {
            $c.regs.sprsys_r_is_flag_set(SprSysR::math_carry)
        }
    }

    macro_rules! WN {
        ($c: expr) => {
            $c.regs.sprsys_r_is_flag_set(SprSysR::math_warning)
        }
    }

    macro_rules! SA {
        ($c: expr, $v: expr) => {
            $c.regs.set_data_r($v);
            set_matha(&mut $c.regs);
        }
    }

    macro_rules! SB {
        ($c: expr, $v: expr) => {
            $c.regs.set_data(MATHB, $v);
        }
    }

    macro_rules! SC {
        ($c: expr, $v: expr) => {
            $c.regs.set_data_r($v);
            set_mathc(&mut $c.regs);
        }
    }

    macro_rules! SD {
        ($c: expr, $v: expr) => {
            $c.regs.set_data(MATHD, $v);
        }
    }
   
    macro_rules! SE {
        ($c: expr, $v: expr) => {
            $c.regs.set_data_r($v);
            set_mathe(&mut $c.regs);
        }
    }

    macro_rules! SF {
        ($c: expr, $v: expr) => {
            $c.regs.set_data(MATHF, $v);
        }
    }

    macro_rules! SG {
        ($c: expr, $v: expr) => {
            $c.regs.set_data(MATHG, $v);
        }
    }

    macro_rules! SH {
        ($c: expr, $v: expr) => {
            $c.regs.set_data(MATHH, $v);
        }
    }

    macro_rules! SJ {
        ($c: expr, $v: expr) => {
            $c.regs.set_data(MATHJ, $v);
        }
    }

    macro_rules! SK {
        ($c: expr, $v: expr) => {
            $c.regs.set_data(MATHK, $v);
        }
    }

    macro_rules! SL {
        ($c: expr, $v: expr) => {
            $c.regs.set_data(MATHL, $v);
        }
    }

    macro_rules! SM {
        ($c: expr, $v: expr) => {
            $c.regs.set_data_r($v);
            set_mathm(&mut $c.regs);
        }
    }

    macro_rules! SN {
        ($c: expr, $v: expr) => {
            $c.regs.set_data(MATHN, $v);
        }
    }

    macro_rules! SP {
        ($c: expr, $v: expr) => {
            $c.regs.set_data(MATHP, $v);
        }
    }

    macro_rules! SAB {
        ($c: expr, $v: expr) => {
            SB!($c, (($v) & 0xFF) as u8);
            SA!($c, ($v) >> 8);
        }
    }

    macro_rules! SCD {
        ($c: expr, $v: expr) => {
            SD!($c, (($v) & 0xFF) as u8);
            SC!($c, ($v) >> 8);
        }
    }

    macro_rules! SNP {
        ($c: expr, $v: expr) => {
            SP!($c, (($v) & 0xFF) as u8);
            SN!($c, (($v) >> 8) as u8);
        }
    }

    macro_rules! ABCD {
        ($c: expr) => {
            $c.regs.abcd()
        }
    }

    macro_rules! EFGH {
        ($c: expr) => {
            $c.regs.efgh()
        }
    }

    macro_rules! JKLM {
        ($c: expr) => {
            $c.regs.jklm()
        }
    }

    macro_rules! SJKLM {
        ($c: expr, $v: expr) => {
            SM!($c, (($v) & 0xFF) as u16);
            SL!($c, ((($v) >> 8) & 0xFF) as u8);
            SK!($c, ((($v) >> 16) & 0xFF) as u8);
            SJ!($c, ((($v) >> 24) & 0xFF) as u8);
        }
    }

    macro_rules! SEFGH {
        ($c: expr, $v: expr) => {
            SH!($c, (($v) & 0xFF) as u8);
            SG!($c, ((($v) >> 8) & 0xFF) as u8);
            SF!($c, ((($v) >> 16) & 0xFF) as u8);
            SE!($c, ((($v) >> 24) & 0xFF) as u16);
        }
    }

    macro_rules! TJKLM {
        ($c: expr, $v: expr, $cy: expr, $wn: expr) => {
            T!($v == JKLM!($c)); 
            T!(CY!($c) == $cy); 
            T!(WN!($c) == $wn);
        };
    }

    macro_rules! MULT_T {
        ($c: expr, $ab: expr, $cd: expr, $exp: expr) => {
            SAB!($c, $ab as u16); 
            SCD!($c, $cd as u16); 
            MULT!($c); 
            T!(EFGH!($c) == ($exp as u32));
        };
    }

    macro_rules! DIV_T {
        ($c: expr, $efgh: expr, $np: expr) => {
            let div = if $np == 0 { u32::MAX } else { $efgh / $np };
            let mo = if $np == 0 { 0 } else { $efgh % $np };
            SEFGH!($c, $efgh as u32); 
            SNP!($c, $np as u16); 
            DIV!($c); 
            T!(ABCD!($c) == div);
            T!(JKLM!($c) == mo);
        };
    }

    #[test]
    fn mult() {
        let mut m: TestCore = TestCore::default();
        
        MULT_T!(m, 0, 0, 0);
        MULT_T!(m, 10, 0, 0);
        MULT_T!(m, 0, 10, 0);
        MULT_T!(m, 512, 0, 0);
        MULT_T!(m, 0, 2048, 0);
        MULT_T!(m, 10, 10, 100);
        MULT_T!(m, 100, 100, (100*100));
        MULT_T!(m, 12, 256, (12*256));
        MULT_T!(m, 512, 256, (512*256));
        MULT_T!(m, 347, 5420, (347*5420));
    }

    #[test]
    fn mult_accumulator() {
        let mut m: TestCore = TestCore::default();
              
        ACC!(m);

        T!(0xffffffff == JKLM!(m));
        
        SJKLM!(m, 0);
        T!(0 == JKLM!(m));

        MULT_T!(m, 10, 10, 100);                                TJKLM!(m, 100, false, false);
        MULT_T!(m, 100, 100, (100*100));                        TJKLM!(m, 10100, false, false);
        MULT_T!(m, 12, 256, (12*256));                          TJKLM!(m, 13172, false, false);
        MULT_T!(m, 512, 256, (512*256));                        TJKLM!(m, 144244, false, false);
        MULT_T!(m, 347, 5420, (347*5420));                      TJKLM!(m, 2024984, false, false);
        MULT_T!(m, 16000, 35002, (16000*35002));                TJKLM!(m, 562_056_984, false, false);
        MULT_T!(m, 50800, 35002, (50800*35002));                TJKLM!(m, 2_340_158_584, false, false);
        MULT_T!(m, 50800, 45002, (50800_u32*45002_u32));        TJKLM!(m, 331_292_888, true, true);
        MULT_T!(m, 12, 256, (12*256));                          TJKLM!(m, 3072+331_292_888, false, false);
    }

    #[test]
    fn mult_signed() {
        let mut m: TestCore = TestCore::default();
      
        SIGNED!(m);

        MULT_T!(m, 0, 0, 0);
        MULT_T!(m, 10, 0, 0);
        MULT_T!(m, 0, 10, 0);
        MULT_T!(m, 512, 0, 0);
        MULT_T!(m, 0, 2048, 0);
        MULT_T!(m, 10, 10, 100);
        MULT_T!(m, 100, 100, (100*100));
        MULT_T!(m, 12, 256, (12*256));
        MULT_T!(m, 512, 256, (512*256));
        MULT_T!(m, 347, 5420, (347*5420));

        MULT_T!(m, 0, -10_i16, 0);
        MULT_T!(m, -10_i16, 0, 0);
        MULT_T!(m, 10, -10_i16, -100_i32);
        MULT_T!(m, -10_i16, -10_i16, 100);
        MULT_T!(m, -10_i16, 10, -100_i32);        
        MULT_T!(m, 512, -512_i16, (-512*512));
        MULT_T!(m, -10_i16, -2048_i16, 20480_i32);
        MULT_T!(m, -768_i16, 10, -7680_i32);        
        MULT_T!(m, -23768_i16, -23768_i16, -23768 * -23768);       
        MULT_T!(m, -22768_i16, 23768_i16, 22768 * -23768);       
    }

    #[test]
    fn div() {
        let mut m: TestCore = TestCore::default();

        DIV_T!(m, 0_u32, 10_u32);
        DIV_T!(m, 0_u32, 5310_u32);
        DIV_T!(m, 200_u32, 10_u32);
        DIV_T!(m, 56740_u32, 24355_u32);
        DIV_T!(m, 1234_u32, 2_u32);
        DIV_T!(m, 12_u32, 512_u32);
        DIV_T!(m, 65535_u32, 512_u32);
        DIV_T!(m, 65535_u32, 2_u32);
        DIV_T!(m, 65535_u32, 65535_u32);
    }

    #[test]
    fn div_0() {
        let mut m: TestCore = TestCore::default();

        DIV_T!(m, 0_u32, 10_u32);
        DIV_T!(m, 456_u32, 0_u32); T!(WN!(m)); T!(CY!(m));
        DIV_T!(m, 0_u32, 10_u32);  T!(!WN!(m)); T!(!CY!(m));
    }
}