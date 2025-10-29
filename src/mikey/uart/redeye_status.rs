use core::ops::Not;
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

impl From<bool> for RedeyeStatus {
    fn from(value: bool) -> Self {
        match value {
            true => RedeyeStatus::High,
            false => RedeyeStatus::Low,
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

impl Not for RedeyeStatus {
    type Output = Self;

    fn not(self) -> Self {
        match self {
            RedeyeStatus::High => RedeyeStatus::Low,
            RedeyeStatus::Low => RedeyeStatus::High,
        }
    }
}
