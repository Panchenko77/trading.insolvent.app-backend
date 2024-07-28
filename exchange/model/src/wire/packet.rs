use crate::Time;
use std::ops::Deref;

pub struct Packet<T> {
    pub received_time: Time,
    pub data: T,
}

impl<T> Packet<T> {
    pub fn new_with_time(data: T, received_time: Time) -> Self {
        Self { received_time, data }
    }
    pub fn new_now(data: T) -> Self {
        Self::new_with_time(data, Time::now())
    }
    pub fn new(data: T) -> Self {
        Self::new_with_time(data, Time::NULL)
    }
    pub fn into_inner(self) -> T {
        self.data
    }
}

impl<T> Deref for Packet<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

pub type PacketStr<'a> = Packet<&'a str>;
pub type PacketString = Packet<String>;
pub type PacketSlice<'a> = Packet<&'a [u8]>;
pub type PacketBytes = Packet<Vec<u8>>;
