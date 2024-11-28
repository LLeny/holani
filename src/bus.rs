use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BusStatus {
    None,
    PeekCart0,
    PeekCart1,
    PokeCart0,
    PokeCart1,
    PeekIncCartRipple,
    PokeIncCartRipple,
    PeekCore,
    PokeCore,
    Peek,
    Poke,
    PeekRAM,
    PeekDone,
    PokeDone,
}

#[derive(Serialize, Deserialize)]
pub struct Bus {
    data: u8,
    addr: u16,
    status: BusStatus,
    request: bool,
    grant: bool,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            data: 0,
            addr: 0,
            status: BusStatus::None,
            request: false,
            grant: true,
        }
    }
        
    pub fn data(&self) -> u8 {
        self.data
    }
    
    pub fn addr(&self) -> u16 {
        self.addr
    }
    
    pub fn status(&self) -> BusStatus {
        self.status
    }
    
    pub fn request(&self) -> bool {
        self.request
    }
    
    pub fn grant(&self) -> bool {
        self.grant
    }
    
    pub fn set_data(&mut self, data: u8) {
        self.data = data;
    }
    
    pub fn set_addr(&mut self, addr: u16) {
        self.addr = addr;
    }
    
    pub fn set_status(&mut self, status: BusStatus) {
        self.status = status;
    }
    
    pub fn set_request(&mut self, request: bool) {
        self.request = request;
    }
    
    pub fn set_grant(&mut self, grant: bool) {
        self.grant = grant;
    }
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for Bus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ addr:{:04x} data:{:04x} status:{:?} request:{:?} grant:{:?} }}", self.addr, self.data, self.status, self.request, self.grant)
    }
}