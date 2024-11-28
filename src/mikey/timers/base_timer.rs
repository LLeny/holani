use alloc::fmt;

use super::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct BaseTimer {
    id: u8,
    int: u8,
    backup: u8,
    control_a: u8,
    count: u8,
    control_b: u8,
    clock_ticks: Option<u32>,
    next_trigger_tick: u64,
    linked_timer: Option<usize>,
    triggered: bool,
    is_linked: bool,
    count_enabled: bool,
    reload_enabled: bool,
}

impl BaseTimer{
    pub fn new(id: u8, linked_timer: Option<usize>) -> Self {
        Self {
            id,
            int: 1 << id,
            backup: 0,
            control_a: 0,
            count: 0,
            control_b: 0,
            clock_ticks: None,
            next_trigger_tick: u64::MAX,
            linked_timer,
            triggered: false,
            is_linked: false,
            count_enabled: false,
            reload_enabled: false,
        }
    }

    pub fn linked_timer(&self) -> Option<usize> {
        self.linked_timer
    }

    #[allow(dead_code)]
    pub fn id(&self) -> u8 {
        self.id
    }

    #[allow(dead_code)]
    fn reset(&mut self) {
        trace!("Timer #{} reset.", self.id);
        self.backup = 0;
        self.count = 0;
        self.control_a = 0;
        self.control_b = 0;
    }

    pub fn backup(&self) -> u8 {
        self.backup
    }
    
    pub fn control_a(&self) -> u8 {
        self.control_a
    }
    
    pub fn count(&self) -> u8 {
        self.count
    }
    
    pub fn control_b(&self) -> u8 {
        self.control_b
    }

    pub fn reset_timer_done(&mut self) {
        self.control_b &= !CTRLB_TIMER_DONE_BIT;
    }

    pub fn set_control_a(&mut self, value: u8, current_tick: u64){
        self.control_a = value;
        self.clock_ticks = match self.period() {
            7 => { None },
            v => { Some(TIMER_TICKS_COUNT as u32 * u32::pow(2, v as u32)) },
        };       
        if value & CTRLA_RESET_DONE_BIT != 0 {
            self.reset_timer_done();
            self.control_a &= !CTRLA_RESET_DONE_BIT;
        }

        self.is_linked = self.period() == 7;
        self.count_enabled = value & CTRLA_ENABLE_COUNT_BIT != 0;
        self.reload_enabled = value & CTRLA_ENABLE_RELOAD_BIT != 0;

        if !self.is_linked && self.count_enabled {
            self.next_trigger_tick = current_tick + self.clock_ticks.unwrap() as u64;
            trace!("Timer #{} next trigger @ {}", self.id, self.next_trigger_tick);
        } else {
            self.next_trigger_tick = u64::MAX;
        }

        trace!("Timer {:?}", self);
    }

    pub fn set_control_b(&mut self, value: u8){
        trace!("Timer #{} ctrl_b = {}.", self.id, value);
        self.control_b = value;
    }

    pub fn set_backup(&mut self, value: u8){
        trace!("Timer #{} backup = {}.", self.id, value);
        self.backup = value;
    }

    pub fn set_count(&mut self, value: u8, current_tick: u64){
        trace!("Timer #{} count = {}.", self.id, value);
        self.count = value;
        if !self.is_linked && self.count_enabled && value != 0 {
            self.next_trigger_tick = current_tick + self.clock_ticks.unwrap() as u64;
            trace!("Timer #{} next trigger @ {}", self.id, self.next_trigger_tick);
        }
    }

    fn period(&self) -> u8 {
        self.control_a() & CTRLA_PERIOD_BIT
    }

    fn interrupt_enabled(&self) -> bool {
        self.control_a() & CTRLA_INTERRUPT_BIT != 0
    }
 
    pub fn is_linked(&self) -> bool {
        self.is_linked
    }  

    fn count_down(&mut self) -> (bool, u8) {
        self.control_b &= !CTRLB_BORROW_OUT_BIT;
        self.control_b |= CTRLB_BORROW_IN_BIT;
        match self.count.cmp(&0) {
            core::cmp::Ordering::Greater => self.count -= 1,
            core::cmp::Ordering::Equal => {
                if self.reload_enabled {
                    trace!("Timer #{} reload 0x{:02x} next trigger @ {}.", self.id, self.backup, self.next_trigger_tick);
                    self.count = self.backup;
                } else {
                    self.next_trigger_tick = u64::MAX;
                }
                return (true, self.done());
            }
            _ => ()
        }
        (false, 0)
    }

    pub fn tick_linked(&mut self) -> (bool, u8) {
        if !self.is_linked {
            return (false, 0);
        }
        
        if self.count_enabled { 
            return self.count_down();
        }
        (false, 0)
    }

    pub fn tick(&mut self, current_tick: u64) -> (bool, u8) {
        self.control_b &= !CTRLB_BORROW_IN_BIT;
        
        if !self.count_enabled { 
            self.next_trigger_tick = u64::MAX;
            return (false, 0);
        }

        self.next_trigger_tick = current_tick + self.clock_ticks.unwrap() as u64;

        self.count_down()
    }

    fn done(&mut self) -> u8 {
        trace!("Timer #{} done.", self.id);
        self.control_b |= CTRLB_TIMER_DONE_BIT | CTRLB_BORROW_OUT_BIT;
        self.triggered = true;
      
        if self.interrupt_enabled() {
            return self.int;
        }

        0
    }

    pub fn triggered(&self) -> bool {
        self.triggered
    }

    pub fn reset_triggered(&mut self) {
        self.triggered = false;
    }
    
    pub fn next_trigger_tick(&self) -> u64 {
        self.next_trigger_tick
    }
}

impl fmt::Debug for BaseTimer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Timer #:{}, backup:{}, period:{}, int:{} reload:{}, count:{}, islinked:{}", 
            self.id,
            self.backup,
            self.period(), 
            self.interrupt_enabled(), 
            self.reload_enabled, 
            self.count_enabled,
            self.is_linked())
    }
}