pub mod comlynx_cable;
pub mod redeye_status;

use std::time::Instant;
use comlynx_cable::ComlynxCable;
use redeye_status::RedeyeStatus;

use super::*;

macro_rules! bool_parity {
    ($bit: expr) => {
        match $bit {
            true => RedeyeStatus::High,
            false => RedeyeStatus::Low,
        }
    };
}

#[derive(Serialize, Deserialize)]
pub struct Uart {
    receive_register_len: u8,
    receive_register_buffer: u8,
    receive_register: Option<u8>,
    break_count: u64,
    transmit_register: Vec<RedeyeStatus>,
    transmit_holding_register: Option<u8>,
    redeye_pin: ComlynxCable,
    #[serde(skip)]
    #[serde(default="create_rand")]
    rand: u8,
}

fn create_rand() -> u8 {
    (Instant::now().elapsed().as_nanos() % 256) as u8
}

impl Uart {
    pub fn new() -> Self {
        Self {  
            receive_register_len: 0,
            receive_register: None,
            receive_register_buffer: 0,
            break_count: 0,
            transmit_register: vec![],
            transmit_holding_register: None,
            redeye_pin: ComlynxCable::default(),
            rand: create_rand(),
        }
    }

    pub fn reset(&mut self) {
        self.receive_register_len = 0;
        self.receive_register = None;
        self.receive_register_buffer = 0;
        self.break_count = 0;
        self.transmit_register.clear();
        self.transmit_holding_register = None;
    }

    pub fn tick(&mut self, regs: &mut MikeyRegisters) -> bool /* tx or rx ready interrupt */ {
        /* "
        Both the transmit and receive interrupts are 'level' sensitive, rather than 'edge' sensitive. 
        This means that an interrupt will be continuously generated as long as it is enabled and its UART buffer is ready.
        " */ 
        self.tx(regs);
        self.rx(regs);

        (regs.serctl_w_is_flag_set(SerCtlW::tx_int_en) && regs.serctl_r_is_flag_set(SerCtlR::tx_rdy)) | 
        (regs.serctl_w_is_flag_set(SerCtlW::rx_int_en) && regs.serctl_r_is_flag_set(SerCtlR::rx_rdy))
    }

    fn tx(&mut self, regs: &mut MikeyRegisters)  {

        if regs.serctl_w_is_flag_set(SerCtlW::tx_brk) {
            self.set_redeye_pin(RedeyeStatus::Low);
            return;    
        }

        if self.transmit_register.is_empty() {
            if let Some(data) = self.transmit_holding_register.take() { 
                trace!("[{}]Transmitting 0x{:02X}", self.rand, data);
                self.load_transmit_data(data, regs);
                regs.serctl_r_enable_flag(SerCtlR::tx_rdy);
                regs.serctl_r_disable_flag(SerCtlR::tx_empty);
            }
            if self.transmit_holding_register.is_none() {
                regs.serctl_r_enable_flag(SerCtlR::tx_empty);
            }
        }

        if let Some(to_send) = self.transmit_register.pop() {
            self.set_redeye_pin(to_send);
        }
    }

    fn load_transmit_data(&mut self, mut data: u8, regs: &mut MikeyRegisters) {
        self.transmit_register.clear();
        // stop bit 
        self.transmit_register.push(RedeyeStatus::High);
        // parity
        self.transmit_register.push(
            match regs.serctl_w_is_flag_set(SerCtlW::par_en) {
                true => {
                    let par = bool_parity!(data.count_ones() & 1 == 1);
                    match par {
                        RedeyeStatus::High => regs.serctl_r_enable_flag(SerCtlR::par_bit),
                        RedeyeStatus::Low => regs.serctl_r_disable_flag(SerCtlR::par_bit),
                    }
                    par
                },
                false => bool_parity!(regs.serctl_w_is_flag_set(SerCtlW::par_even))
            }
        );
        // data
        for _i in 0..8 {
            self.transmit_register.push(if data & 0x01 != 0 {RedeyeStatus::High} else {RedeyeStatus::Low});
            data >>= 1;
        }
        // start bit 
        self.transmit_register.push(RedeyeStatus::Low);
    }

    fn rx(&mut self, regs: &mut MikeyRegisters) {
        let redeye_status = self.redeye_pin.status();

        match redeye_status {
            RedeyeStatus::Low => {
                self.break_count += 1;
                if self.break_count >= 24 {
                    regs.serctl_r_enable_flag(SerCtlR::rx_brk);
                    return;
                }
            }
            RedeyeStatus::High => {
                self.break_count = 0;
                regs.serctl_r_disable_flag(SerCtlR::rx_brk);
            }
        }

        match self.receive_register_len {
            0 => if redeye_status == RedeyeStatus::Low { 
                self.receive_register_buffer = 0;
                self.receive_register_len = 1;
            }        
            1..=8 => {
                self.receive_register_buffer <<= 1;
                self.receive_register_buffer |= if redeye_status == RedeyeStatus::High { 1 } else { 0 };
                self.receive_register_len += 1;
            }
            9 => {                 
                let par = bool_parity!(self.receive_register_buffer.count_ones() & 1 == 1);
                if par == RedeyeStatus::High {
                    regs.serctl_r_enable_flag(SerCtlR::par_bit);
                } else {
                    regs.serctl_r_disable_flag(SerCtlR::par_bit);
                }
                if par != redeye_status {
                    trace!("[{}]Parity Error", self.rand);
                    regs.serctl_r_enable_flag(SerCtlR::par_err);
                } 
                self.receive_register_len += 1;
            }
            10 => {
                if self.receive_register_buffer != 0 && redeye_status == RedeyeStatus::Low { 
                    trace!("[{}]Frame Error", self.rand);
                    regs.serctl_r_enable_flag(SerCtlR::frame_err);
                } 
                trace!("[{}]Received 0x{:02X}", self.rand, self.receive_register_buffer); 
                if self.receive_register.is_some() {
                    trace!("[{}]Overrun", self.rand);  
                    regs.serctl_r_enable_flag(SerCtlR::overrun);
                } else {
                    self.receive_register = Some(self.receive_register_buffer);
                    regs.serctl_r_enable_flag(SerCtlR::rx_rdy);                                        
                }
                self.receive_register_len = 0;
            }
            _ => (),
        }        
    }

    pub fn get_data(&mut self, regs: &mut MikeyRegisters) -> u8 {
        regs.serctl_r_disable_flag(SerCtlR::rx_rdy); 
        match self.receive_register.take() {
            None => 0,
            Some(data) => {
                trace!("[{}]Get 0x{:02X}", self.rand, data);                
                data
            }
        } 
    }

    pub fn set_transmit_holding_buffer(&mut self, regs: &mut MikeyRegisters, data: u8) {
        trace!("[{}]set_transmit_holding_buffer {:02X}", self.rand, data);
        self.transmit_holding_register = Some(data);
        regs.serctl_r_disable_flag(SerCtlR::tx_rdy);
        regs.serctl_r_disable_flag(SerCtlR::tx_empty);
    }

    fn set_redeye_pin(&mut self, status: RedeyeStatus) {
        self.redeye_pin.set(status);
    }

    pub fn set_cable(&mut self, cable: &ComlynxCable) {
        self.redeye_pin = cable.clone();
    }
    
    pub fn cable(&self) -> &ComlynxCable {
        &self.redeye_pin
    }
}

impl Default for Uart {
    fn default() -> Self {
        Self::new()
    }
}