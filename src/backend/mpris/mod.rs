use futures::{future::join_all, StreamExt};
use log::error;
use zbus::{
    fdo::{DBusProxy, NameOwnerChangedArgs},
    Connection,
};

mod player;
mod raw;
pub use player::*;

pub struct Mpris {
    connection: Connection,
    dbus: DBusProxy<'static>,
}

impl Mpris {
    pub async fn new() -> zbus::Result<Self> {
        let connection = Connection::session().await?;
        let dbus = DBusProxy::new(&connection).await?;

        Ok(Self { connection, dbus })
    }

    pub async fn players(&self) -> zbus::Result<Vec<Player>> {
        join_all(
            self.dbus
                .list_names()
                .await?
                .into_iter()
                .filter(|name| {
                    name.starts_with(raw::PlayerProxy::DESTINATION_PREFIX)
                        && !name.contains("playerctld")
                })
                .map(|name| Player::new(&self.connection, name)),
        )
        .await
        .into_iter()
        .collect()
    }

    pub fn connect_players_changed(
        &self,
        added: impl Fn(Player) + Send + 'static,
        removed: impl Fn(Player) + Send + 'static,
    ) {
        let dbus = self.dbus.clone();
        tokio::spawn(async move {
            let mut events = match dbus.receive_name_owner_changed().await {
                Ok(x) => x,
                Err(e) => {
                    error!("Cannot receive events for name owner change: {e}");
                    return;
                }
            };

            while let Some(event) = events.next().await {
                let args = match event.args() {
                    Ok(x) => x,
                    Err(e) => {
                        error!("Cannot parse dbus event args: {e}");
                        return;
                    }
                };

                let NameOwnerChangedArgs {
                    name,
                    old_owner,
                    new_owner,
                    ..
                } = args;

                if !name.starts_with(raw::PlayerProxy::DESTINATION_PREFIX)
                    || name.contains("playerctld")
                {
                    continue;
                }

                let player = Player::new(dbus.inner().connection(), name.into())
                    .await
                    // this can only fail if `name.into()` is not a valid bus name (which is already checked)
                    .unwrap();
                if new_owner.is_some() && old_owner.is_none() {
                    added(player);
                } else if new_owner.is_none() && old_owner.is_some() {
                    removed(player);
                }
            }
        });
    }
}
