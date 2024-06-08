// Since this contains all the spec, some things may not be used
#![allow(dead_code)]

use std::collections::HashMap;
use zbus::{
    proxy,
    zvariant::{ObjectPath, OwnedValue, Structure},
};

/// An time in microseconds
pub type Time = i64;
pub type TrackId<'a> = ObjectPath<'a>;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, strum::EnumString, strum::Display,
)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, strum::EnumString, strum::Display,
)]
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

    #[zbus(property)]
    fn playback_status(&self) -> zbus::Result<PlaybackStatus>;

    // TODO: optional
    #[zbus(property)]
    fn loop_status(&self) -> zbus::Result<LoopStatus>;
    #[zbus(property)]
    fn set_loop_status(&self, value: LoopStatus) -> zbus::Result<()>;

    #[zbus(property)]
    fn rate(&self) -> zbus::Result<f64>;
    #[zbus(property)]
    fn set_rate(&self, value: f64) -> zbus::Result<()>;

    #[zbus(property)]
    fn minimum_rate(&self) -> zbus::Result<f64>;
    #[zbus(property)]
    fn maximum_rate(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn shuffle(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_shuffle(&self, value: bool) -> zbus::Result<()>;

    #[zbus(property)]
    fn metadata(&self) -> zbus::Result<HashMap<String, OwnedValue>>;

    #[zbus(property)]
    fn volume(&self) -> zbus::Result<f64>;
    #[zbus(property)]
    fn set_volume(&self, value: f64) -> zbus::Result<()>;

    #[zbus(property)]
    fn position(&self) -> zbus::Result<Time>;

    #[zbus(property)]
    fn can_go_next(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn can_go_previous(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn can_play(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn can_pause(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn can_seek(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn can_control(&self) -> zbus::Result<bool>;
}

impl TryFrom<OwnedValue> for PlaybackStatus {
    type Error = zbus::zvariant::Error;

    fn try_from(value: OwnedValue) -> Result<Self, Self::Error> {
        let value: String = value.try_into()?;
        value
            .parse()
            .map_err(|e: strum::ParseError| zbus::zvariant::Error::Message(e.to_string()))
    }
}

impl TryFrom<OwnedValue> for LoopStatus {
    type Error = zbus::zvariant::Error;

    fn try_from(value: OwnedValue) -> Result<Self, Self::Error> {
        let value: String = value.try_into()?;
        value
            .parse()
            .map_err(|e: strum::ParseError| zbus::zvariant::Error::Message(e.to_string()))
    }
}

impl From<LoopStatus> for Structure<'_> {
    fn from(value: LoopStatus) -> Self {
        Structure::from((value.to_string(),))
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
