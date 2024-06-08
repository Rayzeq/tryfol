pub use super::raw::PlaybackStatus;
use super::raw::PlayerProxy;
use futures::StreamExt;
use gtk4::glib;
use log::{error, warn};
use std::collections::HashMap;
use zbus::{
    fdo::{PropertiesChangedArgs, PropertiesProxy},
    names::{InterfaceName, OwnedBusName},
    zvariant::{self, Value},
    Connection,
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

    pub fn app_name(&self) -> &str {
        let destination = self.proxy.inner().destination().as_str();
        destination
            .strip_prefix(PlayerProxy::DESTINATION_PREFIX)
            .unwrap_or(destination)
    }

    pub async fn playback_status(&self) -> zbus::Result<PlaybackStatus> {
        self.proxy.playback_status().await
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

    pub async fn toggle(&self) -> zbus::Result<()> {
        let can_control = self.proxy.can_control().await?;
        let can_play = self.proxy.can_play().await?;
        let can_pause = self.proxy.can_pause().await?;

        if !(can_control && can_play && can_pause) {
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

    pub fn connect_on_properties_changed<F>(&self, callback: F)
    where
        F: Fn(InterfaceName, HashMap<&str, Value>, Vec<&str>) + 'static,
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

                callback(interface_name, changed_properties, invalidated_properties);
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
