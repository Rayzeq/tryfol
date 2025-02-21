use super::raw::PlayerProxy;
pub use super::raw::{LoopStatus, PlaybackStatus, Time};
use futures::StreamExt;
use gtk4::glib;
use log::{error, warn};
use std::{borrow::Cow, collections::HashMap, future::Future};
use zbus::{
    Connection,
    fdo::{DBusProxy, PropertiesChangedArgs, PropertiesProxy},
    names::{InterfaceName, OwnedBusName},
    zvariant::{self, Value},
};

#[derive(Debug, Clone)]
pub struct Player {
    proxy: PlayerProxy<'static>,
    properties: PropertiesProxy<'static>,
}

impl Player {
    pub async fn new(connection: &Connection, name: OwnedBusName) -> zbus::Result<Self> {
        let proxy = PlayerProxy::new(connection, name.clone()).await?;
        let properties =
            PropertiesProxy::new(connection, name, proxy.inner().path().to_owned()).await?;
        Ok(Self { proxy, properties })
    }

    pub async fn pid(&self) -> zbus::Result<u32> {
        let dbus = DBusProxy::new(self.proxy.inner().connection()).await?;
        Ok(dbus
            .get_connection_unix_process_id(self.proxy.inner().destination().clone())
            .await?)
    }

    pub fn app_name(&self) -> &str {
        let destination = self.proxy.inner().destination().as_str();
        destination
            .strip_prefix(PlayerProxy::DESTINATION_PREFIX)
            .unwrap_or(destination)
    }

    pub async fn playback_status(&self) -> zbus::Result<PlaybackStatus> {
        self.proxy.playback_status().await
    }

    pub async fn loop_status(&self) -> zbus::Result<Option<LoopStatus>> {
        match self.proxy.loop_status().await {
            Ok(x) => Ok(Some(x)),
            // No such property “LoopStatus”
            Err(zbus::Error::FDO(e)) if matches!(*e, zbus::fdo::Error::InvalidArgs(_)) => Ok(None),
            Err(zbus::Error::FDO(e)) if matches!(*e, zbus::fdo::Error::UnknownProperty(_)) => {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn set_loop_status(&self, status: LoopStatus) -> zbus::Result<()> {
        self.proxy.set_loop_status(status).await
    }

    pub async fn is_shuffled(&self) -> zbus::Result<Option<bool>> {
        match self.proxy.shuffle().await {
            Ok(x) => Ok(Some(x)),
            // No such property “Shuffle”
            Err(zbus::Error::FDO(e)) if matches!(*e, zbus::fdo::Error::InvalidArgs(_)) => Ok(None),
            Err(zbus::Error::FDO(e)) if matches!(*e, zbus::fdo::Error::UnknownProperty(_)) => {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn set_shuffled(&self, shuffle: bool) -> zbus::Result<()> {
        self.proxy.set_shuffle(shuffle).await
    }

    pub async fn title(&self) -> zbus::Result<Option<String>> {
        Ok(self
            .proxy
            .metadata()
            .await?
            .remove("xesam:title")
            .map(TryInto::try_into)
            .transpose()?)
    }

    pub async fn album(&self) -> zbus::Result<Option<String>> {
        Ok(self
            .proxy
            .metadata()
            .await?
            .remove("xesam:album")
            .map(TryInto::try_into)
            .transpose()?
            .filter(|s: &String| !s.is_empty()))
    }

    pub async fn artists(&self) -> zbus::Result<Vec<String>> {
        Ok(self
            .proxy
            .metadata()
            .await?
            .remove("xesam:artist")
            .map(|artists| -> zvariant::Result<Vec<String>> { artists.try_into() })
            .transpose()?
            .unwrap_or_default()
            .into_iter()
            .filter(|artist| !artist.is_empty())
            .collect())
    }

    pub async fn art_url(&self) -> zbus::Result<Option<String>> {
        let url = self
            .proxy
            .metadata()
            .await?
            .remove("mpris:artUrl")
            .map(TryInto::try_into)
            .transpose()?;

        if matches!(url.as_deref(), Some("")) {
            return Ok(None);
        }
        Ok(url)
    }

    pub async fn can_control(&self) -> zbus::Result<bool> {
        self.proxy.can_control().await
    }

    pub async fn can_go_previous(&self) -> zbus::Result<bool> {
        Ok(self.proxy.can_control().await? && self.proxy.can_go_previous().await?)
    }

    pub async fn can_go_next(&self) -> zbus::Result<bool> {
        Ok(self.proxy.can_control().await? && self.proxy.can_go_next().await?)
    }

    pub async fn can_play(&self) -> zbus::Result<bool> {
        Ok(self.proxy.can_control().await? && self.proxy.can_play().await?)
    }

    pub async fn can_pause(&self) -> zbus::Result<bool> {
        Ok(self.proxy.can_control().await? && self.proxy.can_pause().await?)
    }

    pub async fn can_toggle(&self) -> zbus::Result<bool> {
        Ok(self.proxy.can_control().await?
            && self.proxy.can_play().await?
            && self.proxy.can_pause().await?)
    }

    pub async fn position(&self) -> zbus::Result<Option<Time>> {
        match self.proxy.position().await {
            Ok(x) => Ok(Some(x)),
            // "Position is not supported"
            Err(zbus::Error::FDO(e)) if matches!(*e, zbus::fdo::Error::NotSupported(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub async fn length(&self) -> zbus::Result<Option<Time>> {
        Ok(self
            .proxy
            .metadata()
            .await?
            .remove("mpris:length")
            .map(TryInto::try_into)
            .transpose()?)
    }

    pub async fn rate(&self) -> zbus::Result<Option<f64>> {
        match self.proxy.rate().await {
            Ok(x) => Ok(Some(x)),
            Err(zbus::Error::FDO(e)) if matches!(*e, zbus::fdo::Error::NotSupported(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub async fn toggle(&self) -> zbus::Result<()> {
        if !self.can_toggle().await? {
            warn!("Mpris player does not support toggling play state");
            return Ok(());
        }

        self.proxy.play_pause().await?;

        Ok(())
    }

    pub async fn previous(&self) -> zbus::Result<()> {
        let can_control = self.proxy.can_control().await?;
        let can_go_previous = self.proxy.can_go_previous().await?;

        if !(can_control && can_go_previous) {
            warn!("Mpris player does not support going to previous track");
            return Ok(());
        }

        self.proxy.previous().await?;

        Ok(())
    }

    pub async fn next(&self) -> zbus::Result<()> {
        let can_control = self.proxy.can_control().await?;
        let can_go_next = self.proxy.can_go_next().await?;

        if !(can_control && can_go_next) {
            println!("Mpris player does not support going to next track");
            return Ok(());
        }

        self.proxy.next().await?;

        Ok(())
    }

    pub fn connect_on_properties_changed<F, G>(&self, callback: F)
    where
        F: Fn(InterfaceName, HashMap<&str, Value>, Cow<[&str]>) -> G + 'static,
        G: Future,
    {
        let proxy = self.properties.clone();
        glib::spawn_future_local(async move {
            let mut events = match proxy.receive_properties_changed().await {
                Ok(x) => x,
                Err(e) => {
                    error!("Cannot receive events for properties changes: {e:?}");
                    return;
                }
            };

            while let Some(event) = events.next().await {
                let args = match event.args() {
                    Ok(x) => x,
                    Err(e) => {
                        error!("Cannot parse dbus event args: {e:?}");
                        return;
                    }
                };

                let PropertiesChangedArgs {
                    interface_name,
                    changed_properties,
                    invalidated_properties,
                    ..
                } = args;

                callback(interface_name, changed_properties, invalidated_properties).await;
            }
        });
    }

    pub fn connect_seeked<F, G>(&self, callback: F)
    where
        F: Fn(Time) -> G + 'static,
        G: Future,
    {
        let proxy = self.proxy.clone();
        glib::spawn_future_local(async move {
            let mut events = match proxy.receive_seeked().await {
                Ok(x) => x,
                Err(e) => {
                    error!("Cannot receive events for properties changes: {e:?}");
                    return;
                }
            };

            while let Some(event) = events.next().await {
                let args = match event.args() {
                    Ok(x) => x,
                    Err(e) => {
                        error!("Cannot parse dbus event args: {e:?}");
                        return;
                    }
                };

                callback(args.position).await;
            }
        });
    }
}

impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        self.proxy == other.proxy
    }
}

impl Eq for Player {}
