use std::{fs, io, path::Path};

use log::error;

pub type Percentage = u32;
pub type MicrowattHour = u32;
pub type Microvolt = u32;
pub type Microampere = u32;

pub struct Reader<'a> {
    path: &'a Path,
}

pub enum Status {
    Unknown,
    Charging,
    Discharging,
    /// The battery is not charging because it's full enough, but it's not entirely full
    NotCharging,
    Full,
}

impl<'a> Reader<'a> {
    pub const fn new(path: &'a Path) -> Self {
        Self { path }
    }

    pub fn status(&self) -> io::Result<Status> {
        Ok(match fs::read_to_string(self.path.join("status"))?.trim() {
            "Unknown" => Status::Unknown,
            "Charging" => Status::Charging,
            "Discharging" => Status::Discharging,
            "Not charging" => Status::NotCharging,
            "Full" => Status::Full,
            status => {
                error!("Unknown battery status: {status}");
                Status::Unknown
            }
        })
    }

    pub fn capacity(&self) -> anyhow::Result<Percentage> {
        Ok(fs::read_to_string(self.path.join("capacity"))?
            .trim()
            .parse()?)
    }

    pub fn energy_now(&self) -> anyhow::Result<MicrowattHour> {
        Ok(fs::read_to_string(self.path.join("energy_now"))?
            .trim()
            .parse()?)
    }

    pub fn energy_full(&self) -> anyhow::Result<MicrowattHour> {
        Ok(fs::read_to_string(self.path.join("energy_full"))?
            .trim()
            .parse()?)
    }

    pub fn energy_full_design(&self) -> anyhow::Result<MicrowattHour> {
        Ok(fs::read_to_string(self.path.join("energy_full_design"))?
            .trim()
            .parse()?)
    }

    pub fn voltage_now(&self) -> anyhow::Result<Microvolt> {
        Ok(fs::read_to_string(self.path.join("voltage_now"))?
            .trim()
            .parse()?)
    }

    pub fn power_now(&self) -> anyhow::Result<Microampere> {
        Ok(fs::read_to_string(self.path.join("power_now"))?
            .trim()
            .parse()?)
    }
}
