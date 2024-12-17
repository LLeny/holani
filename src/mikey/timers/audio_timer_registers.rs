use alloc::fmt;
use log::trace;
use super::*;

#[derive(Clone, Serialize, Deserialize, Copy, Default)]
pub struct AudioTimerRegisters {
    volume: u8,
    feedback: u8,
    shift_register: u8,
    output: i8,
    disabled: bool,
}

impl AudioTimerRegisters{
    pub fn new() -> Self {
        Self {
            volume: 0,
            feedback: 0,
            shift_register: 0,
            output: 0,
            disabled: false,
        }
    }

    #[allow(dead_code)]
    fn reset(&mut self) {
        trace!("AudioTimerRegisters reset.");
        self.volume = 0;
        self.feedback = 0;
        self.shift_register = 0;
        self.output = 0;
        self.disabled = false;
    }

    #[inline]
    pub fn update_disabled(&mut self, backup_value: u8) {
        self.disabled = backup_value == 0 && self.feedback == 1;
    }
 
    #[inline]
    pub fn volume(&self) -> u8 {
        self.volume
    }
   
    #[inline]
    pub fn output(&self) -> i8 {
        self.output
    }
    
    #[inline]
    pub fn set_volume(&mut self, value: u8) {
        trace!("AudioTimerRegisters volume = {}.", value);
        self.volume = value;
    }
    
    #[inline]
    pub fn set_output(&mut self, value: i8) {
        trace!("AudioTimerRegisters output = {}.", value);
        self.output = value;
    }  

    #[inline]
    pub fn integrate(&self, timer: &Timer) -> bool {
        timer.control_a() & 0b00100000 != 0
    }
    
    #[inline]
    pub fn feedback(&self) -> u8 {
        self.feedback
    }
    
    #[inline]
    pub fn shift_register(&self) -> u8 {
        self.shift_register
    }
    
    #[inline]
    pub fn set_feedback(&mut self, backup_value: u8, feedback: u8) {
        self.feedback = feedback;
        self.update_disabled(backup_value);
    }
    
    #[inline]
    pub fn set_shift_register(&mut self, shift_register: u8) {
        self.shift_register = shift_register;
    }
    
    #[allow(dead_code)]
    fn set_audio_feedback(&mut self, timer: &mut Timer, feedback: u16) {
        trace!("AudioTimerRegisters feedback = {}.", feedback);
        let mut ctrla = timer.control_a() & !0b10000000;
        ctrla |= (feedback as u8) & 0b10000000; // B7=feedback bit 7
        timer.set_control_a_transaprent(ctrla);
        self.feedback = (feedback as u8) & 0b00111111;
        self.feedback |= ((feedback & 0b00001100_00000000) >> 4) as u8; // B7= feedback bit 11, B6=feedback bit 10
    }

    pub fn set_audio_shift_register(&mut self, timer: &mut Timer, value: u16) {
        trace!("AudioTimerRegisters shift register = {}.", value);
        self.shift_register = value as u8;
        let mut ctrlb = timer.control_b(); 
        ctrlb &= !0b11110000; // B7=shift register bit 11, B6=shift register bit 10, B5=shift register bit 9, B4=shift register bit 8
        ctrlb |= ((value & 0b00001111_00000000) >> 4) as u8;
        timer.set_control_b(ctrlb);
    }

    #[inline]
    pub fn audio_shift_register(&self, timer: &Timer) -> u16 {
        self.shift_register as u16 | ((timer.control_b() as u16 & 0b11110000) << 4)
    }

    #[inline]
    pub fn audio_feedback_taps(&self, timer: &Timer) -> u16 {
        let mut fb = self.feedback as u16 & 0b00111111;
        fb |= (self.feedback as u16 & 0b11000000) << 4;
        fb |= (timer.control_a() & 0b10000000) as u16;
        fb
    }  

    pub fn done(&mut self, timer: &mut Timer) -> u8 {
        timer.set_control_b(timer.control_b() | CTRLB_TIMER_DONE_BIT | CTRLB_BORROW_OUT_BIT);          
        
        /* "
        The inversion of the output of the gate is used as the data input to the shift register. [...]
        This same inverted output is taken from the exclusive or gate and sent to the waveshape selector. [...]
        The repeat period is programmed by selecting the initial value in the shift register (set shifter) and by picking which feedback taps are connected.
        " */
        let taps = self.audio_feedback_taps(timer);
        let shift_reg = self.audio_shift_register(timer);        
        let par = (taps & shift_reg).count_ones() as u16 & 1 ^ 1;

        self.set_audio_shift_register(timer, (shift_reg << 1) | par);

        let volume = self.volume() as i8;

        self.set_output(match self.integrate(timer) {
            // "In integrate mode, instead of sending the volume register directly to the DAC it instead adds the volume register (or it's 2's complement) to a running total that is then sent to the DAC."
            // "In integrate mode, shift reg 0 = 1: add volume register to output."
            // "In integrate mode, shift reg 0 = 0: subtract volume register from output."
            true => match par {
                0 => self.output().saturating_add(volume),
                _ => self.output().saturating_sub(volume),
            }
            // "In normal nonintegrate mode, the bit selects either the value in the volume register or its 2's complement and sends it to the output DAC."
            // "In normal mode, shift reg 0 = 1: contains value of volume register."
            // "In normal mode, shift reg 0 = 0: contains 2's complement of volume register."
            false => match par {
                0 => volume,
                _ => -volume
            }
        });
        
        trace!("AudioTimerRegisters output:0x{:02x} {:?}.", self.output() as u8, self);
        0
    }
    
    #[inline]
    pub fn disabled(&self) -> bool {
        self.disabled
    }
}

impl fmt::Debug for AudioTimerRegisters {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AudioTimerRegisters volume:{}, feedback:{}, shift_register:{} output:{}, disabled:{}", 
            self.volume,
            self.feedback,
            self.shift_register, 
            self.output, 
            self.disabled)
    }
}