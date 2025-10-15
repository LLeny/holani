use ::shared_memory::{Shmem, ShmemConf, ShmemError};
use alloc::{fmt, string::String};
use redeye_status::RedeyeStatus;
use serde::{
    de::{self, Visitor},
    Deserializer, Serializer,
};

use super::{alloc, redeye_status, Deserialize, Serialize};

pub struct ComlynxCable {
    shmem: Shmem,
}

impl ComlynxCable {
    /// Creates a new `ComlynxCable` instance.
    ///
    /// # Errors
    ///
    /// Returns an error if unable to create or open the shared memory link 'redeye'.
    pub fn new() -> Result<Self, String> {
        let shmem = match ShmemConf::new().size(32).flink("redeye").create() {
            Ok(m) => {
                unsafe { *m.as_ptr() = RedeyeStatus::High.into() };
                m
            }
            Err(ShmemError::LinkExists) => match ShmemConf::new().flink("redeye").open() {
                Ok(s) => s,
                Err(_) => match ShmemConf::new()
                    .size(32)
                    .flink("redeye")
                    .force_create_flink()
                    .create()
                {
                    Ok(m) => {
                        unsafe { *m.as_ptr() = RedeyeStatus::High.into() };
                        m
                    }
                    Err(e) => {
                        return Err(format!(
                            "Unable to create or open shmem flink 'redeye' : {e}"
                        ))
                    }
                },
            },
            Err(e) => {
                return Err(format!(
                    "Unable to create or open shmem flink 'redeye' : {e}"
                ))
            }
        };
        Ok(ComlynxCable { shmem })
    }

    #[must_use]
    pub fn status(&self) -> RedeyeStatus {
        unsafe { (*self.shmem.as_ptr()).into() }
    }

    pub fn set(&mut self, status: RedeyeStatus) {
        unsafe { *self.shmem.as_ptr() = status.into() };
    }
}

impl Default for ComlynxCable {
    fn default() -> Self {
        ComlynxCable::new().unwrap()
    }
}

impl Serialize for ComlynxCable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let v = unsafe { *self.shmem.as_ptr() };
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

    fn visit_u8<E>(self, _value: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match ComlynxCable::new() {
            Ok(cable) => Ok(cable),
            Err(e) => Err(de::Error::custom(e)),
        }
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
