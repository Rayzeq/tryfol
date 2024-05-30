use core::fmt;
use std::{fmt::Debug, fs::File, io::Read, os::fd::AsRawFd};

use libc::{fcntl, F_GETFL, F_SETFL, O_NONBLOCK};

#[repr(u8)]
#[derive(Clone, Default, Debug, Copy, PartialEq, Eq)]
pub enum Type {
    All = 0,
    Wlan,
    Bluetooth,
    Uwb,
    Wimax,
    Wwan,
    Gps,
    Fm,
    Nfc,
    #[default]
    ThisShouldNotBeHere,
}

#[repr(u8)]
#[derive(Clone, Default, Debug, Copy, PartialEq, Eq)]
pub enum Operation {
    Add = 0,
    Del,
    Change,
    ChangeAll,
    #[default]
    ThisShouldNotBeHere,
}

#[repr(packed)]
#[derive(Default)]
pub struct Event {
    pub index: u32,
    pub r#type: Type,
    pub operation: Operation,
    pub soft: bool,
    pub hard: bool,
}

impl Debug for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self {
            index,
            r#type,
            operation,
            soft,
            hard,
        } = *self;
        f.debug_struct("Event")
            .field("index", &index)
            .field("r#type", &r#type)
            .field("operation", &operation)
            .field("soft", &soft)
            .field("hard", &hard)
            .finish()
    }
}

pub fn list() -> Vec<Event> {
    let mut file = File::open("/dev/rfkill").unwrap();
    let flags = unsafe { fcntl(file.as_raw_fd(), F_GETFL) };
    unsafe { fcntl(file.as_raw_fd(), F_SETFL, flags | O_NONBLOCK) };

    let mut result = Vec::new();
    loop {
        let mut event = Event::default();
        let Ok(()) = file.read_exact(unsafe { any_as_u8_slice(&mut event) }) else {
            break;
        };

        result.push(event);
    }

    result
}

unsafe fn any_as_u8_slice<T: Sized>(p: &mut T) -> &mut [u8] {
    ::core::slice::from_raw_parts_mut((p as *mut T).cast::<u8>(), ::core::mem::size_of::<T>())
}
