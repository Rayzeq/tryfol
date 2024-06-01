use anyhow::Context;
use core::fmt::{self, Display};

mod events;
pub use events::{Event, EventSocket};

#[derive(Clone, Debug)]
pub enum Workspace {
    Regular { id: u32, name: String },
    Special { id: i32, name: String },
}

#[derive(Clone, Copy, Debug)]
pub struct WindowAddress(pub u32);

impl Workspace {
    pub fn from(id: i32, name: String) -> Self {
        match u32::try_from(id) {
            Ok(id) => Self::Regular { id, name },
            Err(_) => Self::Special { id, name },
        }
    }

    pub fn from_raw(id: &str, name: String) -> anyhow::Result<Self> {
        let id = id.parse().context("Invalid workspace id")?;
        Ok(Self::from(id, name))
    }
}

impl WindowAddress {
    pub fn from(address: &str) -> anyhow::Result<Self> {
        Ok(Self(
            u32::from_str_radix(address, 16).context("Invalid workspace id")?,
        ))
    }
}

impl Display for WindowAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}
