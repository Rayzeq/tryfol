use super::proxy::{ItemProxy, WatcherProxy};
use futures::StreamExt;
use log::error;
use std::{collections::HashSet, sync::Arc};
use tokio::sync::Mutex;
use zbus::{
    fdo::{DBusProxy, RequestNameFlags, RequestNameReply},
    interface,
    message::Header,
    names::{BusName, OwnedBusName},
    object_server::SignalEmitter,
    zvariant::OwnedObjectPath,
    Connection,
};

#[derive(Debug, Clone, Default)]
#[allow(clippy::module_name_repetitions)]
pub struct Watcher {
    hosts: Arc<Mutex<HashSet<String>>>,
    items: Arc<Mutex<HashSet<String>>>,
}

impl Watcher {
    const PATH: &'static str = "/StatusNotifierWatcher";
    const WELL_KNOWN_NAME: &'static str = "org.kde.StatusNotifierWatcher";

    pub async fn get_or_start(connection: &Connection) -> zbus::Result<WatcherProxy<'static>> {
        let this = Self::default();
        if !connection.object_server().at(Self::PATH, this).await? {
            error!("A StatusNotifierWatcher object is already running on this connection");
        }

        // TODO: remove DoNotQueue, and wait for the name to be acquired
        // this would be useful if there is already a watcher when we start,
        // but it disappears while were're running
        let flags = RequestNameFlags::DoNotQueue.into();
        match connection
            .request_name_with_flags(Self::WELL_KNOWN_NAME, flags)
            .await
        {
            Ok(
                RequestNameReply::AlreadyOwner
                | RequestNameReply::PrimaryOwner
                | RequestNameReply::Exists,
            )
            // I hate dbus
            | Err(zbus::Error::NameTaken) => Ok(WatcherProxy::new(connection).await?),
            Ok(RequestNameReply::InQueue) => unreachable!(),
            Err(e) => Err(e),
        }
    }

    fn get_bus_name(
        service: &str,
        header: &Header<'_>,
    ) -> zbus::Result<(OwnedBusName, OwnedObjectPath)> {
        // see https://github.com/KDE/plasma-workspace/blob/master/statusnotifierwatcher/statusnotifierwatcher.cpp
        if let Ok(bus_name) = service.try_into() {
            return Ok((bus_name, OwnedObjectPath::try_from(ItemProxy::PATH)?));
        }
        if let (Some(sender), Ok(path)) = (header.sender(), service.try_into()) {
            return Ok((BusName::Unique(sender.to_owned()).into(), path));
        }

        error!(
            "Got invalid service name: {service} (sender is: {:?})",
            header.sender()
        );
        Err(zbus::fdo::Error::InvalidArgs("Unknown bus address".to_owned()).into())
    }

    async fn wait_for_service_exit(
        connection: &Connection,
        service: BusName<'_>,
    ) -> zbus::Result<()> {
        let dbus = DBusProxy::new(connection).await?;
        let mut events = dbus
            // only get signals where args[0] == service (args[0] is the name)
            .receive_name_owner_changed_with_args(&[(0, &service)])
            .await?;

        if !dbus.name_has_owner(service.as_ref()).await? {
            // service has already disappeared
            return Ok(());
        }

        while let Some(event) = events.next().await {
            if event.args()?.new_owner().is_none() {
                break;
            }
        }

        Ok(())
    }
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
// the macro does not use Self
#[allow(clippy::use_self)]
impl Watcher {
    async fn register_status_notifier_host(
        &self,
        service: &str,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        let (bus_name, path) = Self::get_bus_name(service, &header)?;
        let host_id = bus_name.as_str().to_owned() + path.as_str();
        let mut hosts = self.hosts.lock().await;

        if !hosts.insert(host_id.clone()) {
            return Ok(());
        }

        let hosts_count = hosts.len();
        // self.is_status_notifier_host_registered_changed tries to lock the hosts, we must drop it here or we'll deadlock
        drop(hosts);

        if hosts_count == 1 {
            self.is_status_notifier_host_registered_changed(&emitter)
                .await?;
        }
        Self::status_notifier_host_registered(&emitter).await?;

        let this = self.clone();
        let connection = connection.to_owned();
        let emitter = emitter.to_owned();
        tokio::spawn(async move {
            if let Err(e) = Self::wait_for_service_exit(&connection, bus_name.as_ref()).await {
                error!("Error while waiting for {bus_name} to disappear: {e}");
            }

            let mut hosts = this.hosts.lock().await;
            if !hosts.remove(&host_id) {
                return;
            }

            let hosts_count = hosts.len();
            // self.is_status_notifier_host_registered_changed tries to lock the hosts, we must drop it here or we'll deadlock
            drop(hosts);

            if hosts_count == 0 {
                if let Err(e) = this
                    .is_status_notifier_host_registered_changed(&emitter)
                    .await
                {
                    error!("Error while sending is_status_notifier_host_registered signal: {e}");
                }
            }
            if let Err(e) = Self::status_notifier_host_unregistered(&emitter).await {
                error!("Error while sending status_notifier_host_unregistered signal: {e}");
            }
        });

        Ok(())
    }

    async fn register_status_notifier_item(
        &self,
        service: &str,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        let (bus_name, path) = Self::get_bus_name(service, &header)?;
        let item_id = bus_name.as_str().to_owned() + path.as_str();
        let mut items = self.items.lock().await;

        if !items.insert(item_id.clone()) {
            return Ok(());
        }
        // we want to be sure the mutex guard is dropped here and not later
        drop(items);

        self.registered_status_notifier_items_changed(&emitter)
            .await?;
        Self::status_notifier_item_registered(&emitter, &item_id).await?;

        let this = self.clone();
        let connection = connection.to_owned();
        let emitter = emitter.to_owned();
        tokio::spawn(async move {
            if let Err(e) = Self::wait_for_service_exit(&connection, bus_name.as_ref()).await {
                error!("Error while waiting for {bus_name} to disappear: {e}");
            }

            if !this.items.lock().await.remove(&item_id) {
                return;
            }

            if let Err(e) = this
                .registered_status_notifier_items_changed(&emitter)
                .await
            {
                error!("Error while sending registered_status_notifier_items signal: {e}");
            }
            if let Err(e) = Self::status_notifier_item_unregistered(&emitter, &item_id).await {
                error!("Error while sending status_notifier_item_unregistered signal: {e}");
            }
        });

        Ok(())
    }

    #[zbus(property)]
    async fn registered_status_notifier_items(&self) -> Vec<String> {
        self.items
            .lock()
            .await
            .iter()
            .map(|item| item.as_str().to_owned())
            .collect()
    }

    #[zbus(property)]
    async fn is_status_notifier_host_registered(&self) -> bool {
        !self.hosts.lock().await.is_empty()
    }

    #[zbus(property)]
    #[allow(clippy::unused_self)]
    const fn protocol_version(&self) -> i32 {
        0
    }

    #[zbus(signal)]
    async fn status_notifier_host_registered(
        signal_emitter: &SignalEmitter<'_>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_unregistered(
        signal_emitter: &SignalEmitter<'_>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_item_registered(
        signal_emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        signal_emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;
}
