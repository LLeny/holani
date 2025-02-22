use log::trace;

use crate::{consts::*, suzy::{SprSysR, SprSysW}};
use super::SuzyRegisters;

pub fn convert_sign(mut v: u16) -> (u16, i8)  {
    /* "
    In signed multiply, the hardware thinks that 8000 is a positive number. [...]
    In signed multiply, the hardware thinks that 0 is a negative number. 
    This is not an immediate problem for a multiply by zero, since the answer will be re-negated to the correct polarity of zero. 
    However, since it will set the sign flag, you can not depend on the sign flag to be correct if you just load the lower byte after a multiply by zero.
    " */
    let mut sign: i8 = 1;
    if v.overflowing_sub(1).0 & 0x8000 != 0 {
        let mut conversion: u16 = v^0xffff;
        conversion = conversion.overflowing_add(1).0; 
        sign = -1;
        v = conversion;
    }
    (v, sign)
}

pub fn divide(regs: &mut SuzyRegisters) {
    let efgh = regs.efgh();
    let np = regs.np() as u32;

    regs.sprsys_r_disable_flag(SprSysR::math_warning);
    regs.sprsys_r_disable_flag(SprSysR::math_carry);

    if 0 == np { // "The number in the dividend as a result of a divide by zero is 'FFFFFFFF (BigNum)."
        trace!("MATH: divide by zero efgh:0x{:08x} / np:0x{:04x} -> abcd:0xffffffff, jklm: 0x0", efgh, np);
        regs.set_abcd(0xffffffff);
        regs.set_jklm(0);
        regs.sprsys_r_enable_flag(SprSysR::math_warning);
        regs.sprsys_r_enable_flag(SprSysR::math_carry);
    }
    else {
        let abcd = efgh / np;
        let jklm = efgh % np;
        trace!("MATH: divide efgh:0x{:08x} / np:0x{:04x} -> abcd:0x{:08x}, jklm: 0x{:08x}", efgh, np, abcd, jklm);
        regs.set_abcd(abcd);
        regs.set_jklm(jklm);
    
        trace!("D;0x{:08X};0x{:04X};0x{:08X};0x{:08X}\n", efgh, np, abcd, jklm);
    }
    
    regs.sprsys_r_disable_flag(SprSysR::math_working);
}

pub fn multiply(regs: &mut SuzyRegisters) {
    let ab = regs.ab() as u32;
    let cd = regs.tmp_cd() as u32;
    let mut efgh = ab.overflowing_mul(cd).0;

    regs.sprsys_r_disable_flag(SprSysR::math_warning);
    regs.sprsys_r_disable_flag(SprSysR::math_carry);
    
    if regs.sprsys_w_is_flag_set(SprSysW::sign_math) && 0 == regs.sign_ab() + regs.tmp_sign_cd() {
        efgh ^= 0xffffffff;
        efgh = efgh.overflowing_add(1).0;
    }

    trace!("MATH: multiply ab:0x{:04x} * cd:0x{:04x} -> efgh:0x{:08x}", ab, cd, efgh);

    regs.set_efgh(efgh);

    if regs.sprsys_w_is_flag_set(SprSysW::accumulate) {
        let jklm = regs.jklm() as i64;
        let efgh = regs.efgh() as i64;
        let r = jklm.overflowing_add(efgh).0;

        trace!("MATH: multiply accumulate jklm:0x{:08x} + efgh:0x{:08x} -> jklm:0x{:08x}", jklm, efgh, r);
        if r > u32::MAX as i64 {
            trace!("MATH: multiply accumulate overflow");
            regs.sprsys_r_enable_flag(SprSysR::math_warning);
            regs.sprsys_r_enable_flag(SprSysR::math_carry);
        }
        regs.set_jklm(r as u32);
    }

    trace!("M;0x{:04X};0x{:04X};0x{:08X};0x{:08X}", ab, cd, efgh, regs.jklm());

    regs.sprsys_r_disable_flag(SprSysR::math_working);
}

pub fn set_matha(regs: &mut SuzyRegisters) {
    // "The conversion that is performed on the CPU provided starting numbers is done when the upper byte is sent by the CPU."
    trace!("[MATHA] = 0x{:02x}", regs.data_r() as u8);
    regs.set_data(MATHA, regs.data_r() as u8);
    if regs.sprsys_w_is_flag_set(SprSysW::sign_math) {
        let (v, s) = convert_sign(regs.ab());
        regs.set_ab(v);
        regs.set_sign_ab(s);
        regs.set_task_ticks_delay(SUZY_MULT_SIGN_TICKS);
    } 
    else if regs.sprsys_w_is_flag_set(SprSysW::accumulate) {
        regs.set_sign_ab(1);
        regs.set_task_ticks_delay(SUZY_MULT_SIGN_TICKS);
    } 
    else {
        regs.set_sign_ab(1);
        regs.set_task_ticks_delay(SUZY_MULT_NON_SIGN_TICKS);
    }
    regs.backup_cd();
    regs.sprsys_r_enable_flag(SprSysR::math_working);
    regs.reset_ir();
}

pub fn set_mathc(regs: &mut SuzyRegisters) {
    // "The conversion that is performed on the CPU provided starting numbers is done when the upper byte is sent by the CPU."
    trace!("[MATHC] = 0x{:02x}", regs.data_r() as u8);
    regs.set_data(MATHC, regs.data_r() as u8);
    if regs.sprsys_w_is_flag_set(SprSysW::sign_math) {
        let (v, s) = convert_sign(regs.cd());
        regs.set_cd(v);
        regs.set_sign_cd(s);
    } else {
        regs.set_sign_cd(1);
    }
    regs.reset_ir();
}

pub fn set_mathe(regs: &mut SuzyRegisters) {
    trace!("[MATHE] = 0x{:02x}", regs.data_r() as u8);
    regs.set_data(MATHE, regs.data_r() as u8);
    regs.sprsys_r_enable_flag(SprSysR::math_working);
    // "Divides take 176 + 14*N ticks where N is the number of most significant zeros in the divisor."
    regs.set_task_ticks_delay(176_u16 + 14 * regs.np().leading_zeros() as u16);
    regs.reset_ir();
}

pub fn set_mathm(regs: &mut SuzyRegisters) { // "The write to 'M' will clear the accumulator overflow bit"
    trace!("[MATHM] = 0x{:02x}, [MATHL] = 0x00", regs.data_r());
    regs.set_data(MATHM, regs.data_r() as u8);
    regs.set_data(MATHL, 0);
    regs.sprsys_r_disable_flag(SprSysR::math_warning);
    regs.reset_ir();
}