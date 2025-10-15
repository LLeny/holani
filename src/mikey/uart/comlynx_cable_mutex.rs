use alloc::{fmt, sync::Arc};
use parking_lot::Mutex;
use redeye_status::RedeyeStatus;
use serde::{
    de::{self, Visitor},
    Deserializer, Serializer,
};

use super::{alloc, redeye_status, Deserialize, Serialize};

pub struct ComlynxCable {
    redeye_pin: Arc<Mutex<RedeyeStatus>>,
}

impl ComlynxCable {
    #[must_use]
    pub fn new(cable: Option<Arc<Mutex<RedeyeStatus>>>) -> Self {
        if let Some(redeye_pin) = cable {
            Self { redeye_pin }
        } else {
            Self {
                redeye_pin: Arc::new(Mutex::new(RedeyeStatus::High)),
            }
        }
    }

    #[must_use]
    pub fn status(&self) -> RedeyeStatus {
        *self.redeye_pin.lock()
    }

    pub fn set(&mut self, status: RedeyeStatus) {
        *self.redeye_pin.lock() = status;
    }
}

impl Default for ComlynxCable {
    fn default() -> Self {
        ComlynxCable::new(None)
    }
}

impl Clone for ComlynxCable {
    fn clone(&self) -> Self {
        Self {
            redeye_pin: self.redeye_pin.clone(),
        }
    }
}

impl Serialize for ComlynxCable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let v = *self.redeye_pin.lock() as u8;
        serializer.serialize_u8(v)
    }
}

struct ComlynxCableVisitor;

impl ComlynxCableVisitor {
    fn new() -> Self {
        Self {}
    }
}

impl Visitor<'_> for ComlynxCableVisitor {
    type Value = ComlynxCable;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an u8")
    }

    fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let cable = ComlynxCable::new(Some(Arc::new(Mutex::new(value.into()))));
        Ok(cable)
    }
}

impl<'de> Deserialize<'de> for ComlynxCable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_u8(ComlynxCableVisitor::new())
    }
}
