use zbus::{names::BusName, proxy};

#[proxy(
    default_service = "org.kde.StatusNotifierWatcher",
    interface = "org.kde.StatusNotifierWatcher",
    default_path = "/StatusNotifierWatcher"
)]
trait Watcher {
    /// Register a StatusNotifierHost into the StatusNotifierWatcher, in the form of its full name on the session bus,
    /// for instance org.freedesktop.StatusNotifierHost-4005.
    /// Every NotficationHost instance that intends to display StatusNotifierItem representations
    /// should register to StatusNotifierWatcher with this method.
    /// The StatusNotifierWatcher should automatically notice if an instance of StatusNotifierHost goes away.
    fn register_status_notifier_host(&self, service: BusName<'_>) -> zbus::Result<()>;

    /// Register a StatusNotifierItem into the StatusNotifierWatcher, in the form of its full name on the session bus,
    /// for instance org.freedesktop.StatusNotifierItem-4077-1.
    /// A StatusNotifierItem instance must be registered to the watcher in order to be noticed from
    /// both the watcher and the StatusNotifierHost instances.
    /// If the registered StatusNotifierItem goes away from the session bus,
    /// the StatusNotifierWatcher should automatically notice it and remove it from the list of registered services.
    fn register_status_notifier_item(&self, service: BusName<'_>) -> zbus::Result<()>;

    /// List containing all the registered instances of StatusNotifierItem.
    ///
    /// All elements of the array should correspond to services actually running on the session bus
    /// at the moment of the method call.
    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> zbus::Result<Vec<String>>;

    /// Whether there is at leat onr StatusNotifierHost registered and running.
    ///
    /// If no StatusNotifierHost are registered and running, all StatusNotifierItem instances
    /// should fall back using the Freedesktop System tray specification.
    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> zbus::Result<bool>;

    /// The version of the protocol the StatusNotifierWatcher instance implements.
    #[zbus(property)]
    fn protocol_version(&self) -> zbus::Result<i32>;

    /// A new StatusNotifierHost has been registered, the StatusNotifierItem instances
    /// knows that they can use this protocol instead of the Freedesktop System tray protocol.
    #[zbus(signal)]
    fn status_notifier_host_registered(&self) -> zbus::Result<()>;

    /// A StatusNotifierHost instance has disappeared from the bus, the StatusNotifierItem instances
    /// may change the protocol they use.
    #[zbus(signal)]
    fn status_notifier_host_unregistered(&self) -> zbus::Result<()>;

    /// A new StatusNotifierItem has been registered, the argument of the signal is the session bus name of the instance.
    /// StatusNotifierHost implementation should listen this signal to know
    /// when they should update their representation of the items.
    #[zbus(signal)]
    fn status_notifier_item_registered(&self, service: &str) -> zbus::Result<()>;

    /// A StatusNotifierItem instance has disappeared from the bus,
    /// the argument of the signal is the session bus name of the instance.
    /// StatusNotifierHost implementation should listen this signal to know
    /// when they should update their representation of the items.
    #[zbus(signal)]
    fn status_notifier_item_unregistered(&self, service: &str) -> zbus::Result<()>;
}
