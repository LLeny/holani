use core::cell::UnsafeCell;
use core::ops::{Index, IndexMut};
use alloc::fmt;
use alloc::vec::Vec;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub struct SharedMemory {
    data: UnsafeCell<Vec<u8>>
}

// To share the RAM with Libretro, shouldn't have concurrent accesses
impl SharedMemory {
    pub fn new(len: usize, fill_with: u8) -> Self {
        Self {
            data: UnsafeCell::new(vec![fill_with; len])
        }
    }
    
    pub fn get_mut(&mut self) -> &mut Self {
        self
    }
    
    pub fn fill(&mut self, v: u8) {
        let ptr = self.data.get_mut();
        (*ptr).fill(v);
    }

    pub fn copy(&mut self, dest: u16, buf: &[u8]) {
        let d = dest as usize;
        let ptr = self.data.get_mut();
        (*ptr)[d..(d + buf.len())].copy_from_slice(buf);
    }

    // Libretro only
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn as_mut_slice(&self) -> &mut [u8] {
        let ptr = self.data.get();
        unsafe { &mut (*ptr) }
    }

    pub unsafe fn as_slice(&self) -> &[u8] {
        let ptr = self.data.get();
        unsafe { &(*ptr) }
    }
}

impl Default for SharedMemory {
    fn default() -> Self {
        SharedMemory::new(0, 0xFF)
    }
}

impl Index<usize> for SharedMemory {
    type Output = u8;
    fn index(&self, i: usize) -> &u8 {
        let ptr = self.data.get();
        unsafe { &(*ptr)[i] }
    }
}

impl IndexMut<usize> for SharedMemory {
    fn index_mut(&mut self, i: usize) -> &mut u8 {
        let ptr = self.data.get_mut();
        &mut (*ptr)[i]
    }
}

impl Serialize for SharedMemory {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let ptr = self.data.get();
        let mut seq = serializer.serialize_seq(Some(unsafe{(*ptr).len()}))?;
        unsafe {
            for e in (*ptr).iter() {
                seq.serialize_element(&e)?;
            }
        }
        seq.end()
    }
}

struct SharedMemoryVisitor;

impl SharedMemoryVisitor {
    fn new() -> Self {
        Self {}
    }
}

impl<'de> Visitor<'de> for SharedMemoryVisitor {
    type Value = SharedMemory;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("struct SharedMemory")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>, 
    {
        let mut mem = SharedMemory::default();
        let ptr = mem.data.get_mut();        
        while let Some(value) = seq.next_element()? {
            (*ptr).push(value);
        }
        Ok(mem)
    }
}

impl<'de> Deserialize<'de> for SharedMemory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(SharedMemoryVisitor::new())
    }
}