// Since this contains all the spec, some things may not be used
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::EnumString;
use zbus::{
    proxy,
    zvariant::{self, ObjectPath, OwnedValue},
};

/// An time in microseconds
pub type Time = i64;
pub type TrackId<'a> = ObjectPath<'a>;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    EnumString,
    strum::Display,
    Serialize,
    Deserialize,
    zvariant::Type,
)]
#[serde(into = "String", try_from = "String")]
#[zvariant(signature = "s")]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    EnumString,
    strum::Display,
    Serialize,
    Deserialize,
    zvariant::Type,
)]
#[serde(into = "String", try_from = "String")]
#[zvariant(signature = "s")]
pub enum LoopStatus {
    None,
    Track,
    Playlist,
}

#[proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_path = "/org/mpris/MediaPlayer2"
)]
pub trait Player {
    fn next(&self) -> zbus::Result<()>;
    fn previous(&self) -> zbus::Result<()>;
    fn pause(&self) -> zbus::Result<()>;
    fn play_pause(&self) -> zbus::Result<()>;
    fn stop(&self) -> zbus::Result<()>;
    fn play(&self) -> zbus::Result<()>;
    fn seek(&self, offset: Time) -> zbus::Result<()>;
    fn set_position(&self, track: TrackId<'_>, position: Time) -> zbus::Result<()>;
    fn open_uri(&self, uri: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    fn seeked(&self, position: Time) -> fdo::Result<()>;

    #[dbus_proxy(property)]
    fn playback_status(&self) -> zbus::Result<PlaybackStatus>;

    // TODO: optional
    #[dbus_proxy(property)]
    fn loop_status(&self) -> zbus::Result<LoopStatus>;
    #[dbus_proxy(property)]
    fn set_loop_status(&self, value: LoopStatus) -> zbus::Result<()>;

    #[dbus_proxy(property)]
    fn rate(&self) -> zbus::Result<f64>;
    #[dbus_proxy(property)]
    fn set_rate(&self, value: f64) -> zbus::Result<()>;

    #[dbus_proxy(property)]
    fn minimum_rate(&self) -> zbus::Result<f64>;
    #[dbus_proxy(property)]
    fn maximum_rate(&self) -> zbus::Result<f64>;

    #[dbus_proxy(property)]
    fn shuffle(&self) -> zbus::Result<bool>;
    #[dbus_proxy(property)]
    fn set_shuffle(&self, value: bool) -> zbus::Result<()>;

    #[dbus_proxy(property)]
    fn metadata(&self) -> zbus::Result<HashMap<String, OwnedValue>>;

    #[dbus_proxy(property)]
    fn volume(&self) -> zbus::Result<f64>;
    #[dbus_proxy(property)]
    fn set_volume(&self, value: f64) -> zbus::Result<()>;

    #[dbus_proxy(property)]
    fn position(&self) -> zbus::Result<Time>;

    #[dbus_proxy(property)]
    fn can_go_next(&self) -> zbus::Result<bool>;

    #[dbus_proxy(property)]
    fn can_go_previous(&self) -> zbus::Result<bool>;

    #[dbus_proxy(property)]
    fn can_play(&self) -> zbus::Result<bool>;

    #[dbus_proxy(property)]
    fn can_pause(&self) -> zbus::Result<bool>;

    #[dbus_proxy(property)]
    fn can_seek(&self) -> zbus::Result<bool>;

    #[dbus_proxy(property)]
    fn can_control(&self) -> zbus::Result<bool>;
}

impl From<PlaybackStatus> for String {
    fn from(s: PlaybackStatus) -> Self {
        s.to_string()
    }
}

impl TryFrom<String> for PlaybackStatus {
    type Error = <Self as std::str::FromStr>::Err;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<LoopStatus> for String {
    fn from(s: LoopStatus) -> Self {
        s.to_string()
    }
}

impl TryFrom<String> for LoopStatus {
    type Error = <Self as std::str::FromStr>::Err;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl PlayerProxy<'static> {
    pub const DESTINATION_PREFIX: &'static str = "org.mpris.MediaPlayer2.";
}

impl PartialEq for PlayerProxy<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.inner().destination() == other.inner().destination()
    }
}

impl Eq for PlayerProxy<'_> {}
