use anyhow::Context;
use core::fmt::{self, Display};
use serde::{Deserialize, de::Error};
use serde_repr::Deserialize_repr;
use std::path::{Path, PathBuf};

mod control;
mod events;
pub use control::*;
pub use events::{Event, events};

pub type WorkspaceId = i32;
pub type MonitorId = i128;

#[derive(Debug, Deserialize)]
struct RawWorkspace {
    id: WorkspaceId,
    name: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum Workspace {
    Regular { id: WorkspaceId, name: String },
    Special { id: WorkspaceId, name: String },
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct WorkspaceInfos {
    pub id: WorkspaceId,
    pub name: String,
    pub monitor: String,
    #[serde(rename = "monitorID")]
    pub monitor_id: MonitorId,
    pub windows: u16,
    #[serde(rename = "hasfullscreen")]
    pub has_fullscreen: bool,
    #[serde(rename = "lastwindow")]
    pub last_window: WindowAddress,
    #[serde(rename = "lastwindowtitle")]
    pub last_window_title: String,
}

#[allow(dead_code, clippy::struct_excessive_bools)]
#[derive(Debug, Deserialize)]
pub struct WindowInfos {
    pub address: WindowAddress,
    pub at: (i16, i16),
    pub size: (i16, i16),
    pub workspace: Workspace,
    pub floating: bool,
    pub fullscreen: FullscreenMode,
    #[serde(rename = "fullscreenClient")]
    pub fullscreen_client: FullscreenMode,
    pub monitor: MonitorId,
    #[serde(rename = "initialClass")]
    pub initial_class: String,
    pub class: String,
    #[serde(rename = "initialTitle")]
    pub initial_title: String,
    pub title: String,
    pub pid: i32,
    pub xwayland: bool,
    pub pinned: bool,
    // TODO: this is a temporary type because I don't know the real one
    pub grouped: Vec<()>,
    pub mapped: bool,
    pub swallowing: String,
    #[serde(rename = "focusHistoryID")]
    pub focus_history_id: i8,
    pub hidden: bool,
}

#[derive(Debug, Deserialize_repr)]
#[repr(u8)]
pub enum FullscreenMode {
    None = 0,
    Maximized = 1,
    Fullscreen = 2,
    MaximizedFullscreen = 3,
}

#[derive(Clone, Copy, Debug)]
pub struct WindowAddress(pub u32);

impl Workspace {
    pub const fn from(id: i32, name: String) -> Self {
        if id < 0 {
            Self::Special { id, name }
        } else {
            Self::Regular { id, name }
        }
    }

    pub fn from_raw(id: &str, name: String) -> anyhow::Result<Self> {
        let id = id.parse().context("Invalid workspace id")?;
        Ok(Self::from(id, name))
    }

    pub const fn id(&self) -> WorkspaceId {
        match self {
            Self::Regular { id, .. } | Self::Special { id, .. } => *id,
        }
    }
}

impl<'de> Deserialize<'de> for Workspace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawWorkspace::deserialize(deserializer)?;
        Ok(Self::from(raw.id, raw.name))
    }
}

impl WorkspaceInfos {
    pub const fn is_regular(&self) -> bool {
        !self.is_special()
    }

    pub const fn is_special(&self) -> bool {
        self.id < 0
    }
}

impl WindowAddress {
    pub fn from(mut address: &str) -> anyhow::Result<Self> {
        if address.starts_with("0x") {
            address = &address[2..];
        }
        Ok(Self(
            u32::from_str_radix(address, 16).context("Invalid window address")?,
        ))
    }
}

impl<'de> Deserialize<'de> for WindowAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let address = String::deserialize(deserializer)?;
        Self::from(&address).map_err(D::Error::custom)
    }
}

impl Display for WindowAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

fn get_hyprland_path() -> anyhow::Result<PathBuf> {
    Ok(Path::new(
        &std::env::var_os("XDG_RUNTIME_DIR")
            .context("Runtime directory is not set (missing $XDG_RUNTIME_DIR)")?,
    )
    .join("hypr")
    .join(
        std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .context("Can't find Hyprland directory (missing $HYPRLAND_INSTANCE_SIGNATURE)")?,
    ))
}
