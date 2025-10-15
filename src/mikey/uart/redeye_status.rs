use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum RedeyeStatus {
    Low = 0,
    High = 1,
}

impl From<u8> for RedeyeStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => RedeyeStatus::Low,
            _ => RedeyeStatus::High,
        }
    }
}

impl From<RedeyeStatus> for u8 {
    fn from(val: RedeyeStatus) -> Self {
        match val {
            RedeyeStatus::High => 1,
            RedeyeStatus::Low => 0,
        }
    }
}
