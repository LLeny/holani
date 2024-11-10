use std::fmt;
use redeye_status::RedeyeStatus;
use serde::{de::{self, Visitor}, Deserializer, Serializer};
use ::shared_memory::{Shmem, ShmemConf, ShmemError};

use super::*;

pub struct ComlynxCable {
    shmem: Shmem,
}

impl ComlynxCable {
    pub fn new() -> Self {
        let shmem = match ShmemConf::new().size(32).flink("redeye").create() {
            Ok(m) => {
                unsafe { *m.as_ptr() = RedeyeStatus::High.into() };
                m
            },
            Err(ShmemError::LinkExists) => ShmemConf::new().flink("redeye").open().unwrap(),
            Err(e) => panic!("Unable to create or open shmem flink 'redeye' : {}", e)
        };
        ComlynxCable { shmem }
    }

    pub fn status(&self) -> RedeyeStatus {
        unsafe { (*self.shmem.as_ptr()).into() }
    }

    pub fn set(&mut self, status: RedeyeStatus) {
        unsafe { *self.shmem.as_ptr() = status.into() };
    }
}

impl Default for ComlynxCable {
    fn default() -> Self {
        ComlynxCable::new()
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

impl<'de> Visitor<'de> for ComlynxCableVisitor {
    type Value = ComlynxCable;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an u8")
    }

    fn visit_u8<E>(self, _value: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let cable = ComlynxCable::new();
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