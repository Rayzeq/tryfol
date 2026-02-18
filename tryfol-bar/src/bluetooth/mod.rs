use crate::HasTooltip;
use futures::StreamExt;
use gtk::{
    Button, EventControllerMotion, Label, Orientation, Revealer, RevealerTransitionType, gdk,
    glib::{self, clone},
    prelude::*,
};
use gtk4 as gtk;
use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    hash::Hash,
    process::Stdio,
    sync::atomic::{AtomicUsize, Ordering},
};
use tokio::process::Command;
use zbus::{
    Connection,
    fdo::{ObjectManagerProxy, PropertiesProxy},
    zvariant::ObjectPath,
};

mod dbus;
use dbus::DeviceProxy;

lazy_static! {
    static ref ICONS: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        map.insert("audio-headset", "󰋎");
        map
    };
}

const BLUEZ_SERVICE: &str = "org.bluez";
static CONNECTED_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn new() -> gtk::Box {
    let label = Label::new(Some("󰂯"));
    let button = Button::builder().child(&label).build();
    let connected_devices = gtk::Box::new(Orientation::Horizontal, 10);
    connected_devices.add_css_class("right");
    connected_devices.set_visible(false);
    let devices = gtk::Box::new(Orientation::Horizontal, 10);
    devices.add_css_class("right");
    let revealer = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideLeft)
        .transition_duration(500)
        .child(&devices)
        .build();
    let container = gtk::Box::new(Orientation::Horizontal, 0);
    container.append(&button);
    container.append(&connected_devices);
    container.append(&revealer);
    container.set_widget_name("bluetooth");
    container.add_css_class("left");

    button.connect_clicked(|_| {
        let mut child = Command::new("blueman-manager")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        glib::spawn_future_local(async move { child.wait().await });
    });

    let event_controller = EventControllerMotion::new();
    event_controller.connect_enter(clone!(
        #[strong]
        revealer,
        move |_, _, _| {
            revealer.set_reveal_child(true);
        }
    ));
    event_controller.connect_leave(clone!(
        #[strong]
        revealer,
        move |_| {
            revealer.set_reveal_child(false);
        }
    ));
    container.add_controller(event_controller);

    glib::spawn_future_local(run_manager(label, connected_devices, devices));

    container
}

async fn run_manager(label: Label, connected_container: gtk::Box, container: gtk::Box) {
    let connection = Connection::system().await.unwrap();
    let object_manager = ObjectManagerProxy::new(&connection, BLUEZ_SERVICE, "/")
        .await
        .unwrap();

    let mut added_events: zbus::fdo::InterfacesAddedStream =
        object_manager.receive_interfaces_added().await.unwrap();
    let mut removed_events = object_manager.receive_interfaces_removed().await.unwrap();
    let mut devices = HashMap::new();

    for (object, interfaces) in object_manager.get_managed_objects().await.unwrap() {
        if let Some(device) = new_interface(
            &label,
            &connected_container,
            &container,
            &connection,
            &object,
            &interfaces,
        )
        .await
        {
            devices.insert(object, device);
        }
    }

    loop {
        tokio::select! {
            message = added_events.next() => {
                let message = message.unwrap();
                let args = message.args().unwrap();
                if let Some(device) = new_interface(&label, &connected_container, &container, &connection, args.object_path(), args.interfaces_and_properties()).await {
                    if let Some(old) = devices.insert(args.object_path().to_owned().into(), device) {
                        let parent: gtk::Box = old.parent().unwrap().downcast().unwrap();
                        parent.remove(&old);

                        if parent == connected_container {
                            let old_count = CONNECTED_COUNT.fetch_sub(1, Ordering::SeqCst);
                            if old_count == 1 {
                                label.set_text("󰂯");
                                label.remove_css_class("connected");
                                connected_container.set_visible(false);
                            }
                        }
                    }
                }
            }
            message = removed_events.next() => {
                let message = message.unwrap();
                let args = message.args().unwrap();
                // I shouldn't have to clone here, I'm pretty sure it's a rust bug
                // or is it because of tokio::select! ?
                if let Some(device) = devices.remove(&args.object_path().to_owned()) {
                    let parent: gtk::Box = device.parent().unwrap().downcast().unwrap();
                    parent.remove(&device);

                    if parent == connected_container {
                        let old_count = CONNECTED_COUNT.fetch_sub(1, Ordering::SeqCst);
                        if old_count == 1 {
                            label.set_text("󰂯");
                            label.remove_css_class("connected");
                            connected_container.set_visible(false);
                        }
                    }
                };
            }
        }
    }
}

async fn new_interface<'a, K, V>(
    global_label: &Label,
    connected_devices: &gtk::Box,
    devices: &gtk::Box,
    connection: &Connection,
    object_path: &ObjectPath<'_>,
    interfaces: &HashMap<K, V>,
) -> Option<Label>
where
    K: Eq + Hash + TryFrom<&'a str>,
{
    let Ok(ifname) = K::try_from("org.bluez.Device1") else {
        panic!("Someting went wrong, and debug is fucked");
    };
    if !interfaces.contains_key(&ifname) {
        return None;
    }

    let device = DeviceProxy::builder(connection)
        .path(object_path.to_owned())
        .unwrap()
        .build()
        .await
        .unwrap();

    let widget = Label::new(None);
    // this needs to be here because update_device needs the widget to have a parent
    devices.append(&widget);
    let mut is_connected = false;
    // blocked: device.Blocked().await?,
    // bonded: device.Bonded().await?,
    // connected: device.Connected().await?,
    // paired: device.Paired().await?,
    // trusted: device.Trusted().await?,
    // address: device.Address().await?,

    let properties_proxy = PropertiesProxy::new(
        connection,
        device.inner().destination().to_owned(),
        device.inner().path().to_owned(),
    )
    .await
    .unwrap();
    let mut events = properties_proxy.receive_properties_changed().await.unwrap();
    let task = glib::spawn_future_local(clone!(
        #[strong]
        global_label,
        #[strong]
        connected_devices,
        #[strong]
        devices,
        #[strong]
        widget,
        #[strong]
        device,
        async move {
            while let Some(_event) = events.next().await {
                update_device(
                    &global_label,
                    &connected_devices,
                    &devices,
                    &device,
                    &widget,
                    &mut is_connected,
                )
                .await;
            }
        }
    ));
    // todo: is it needed ?
    std::mem::forget(properties_proxy);
    widget.connect_destroy(move |_| task.abort());

    let gesture = gtk::GestureClick::new();
    gesture.set_button(gdk::BUTTON_PRIMARY);
    gesture.connect_released(clone!(
        #[strong]
        device,
        move |_, n_press, _, _| {
            glib::spawn_future_local(clone!(
                #[strong]
                device,
                async move {
                    if n_press == 2 {
                        if device.Connected().await.unwrap() {
                            device.Disconnect().await.unwrap();
                        } else {
                            device.Connect().await.unwrap();
                        }
                    }
                }
            ));
        }
    ));
    widget.add_controller(gesture);

    // let gesture = gtk::GestureClick::new();
    // gesture.set_button(gdk::BUTTON_SECONDARY);
    // gesture.connect_released(clone!(@strong device => move |_, n_press, _, _| {
    //     glib::spawn_future_local(clone!(@strong device => async move {

    //     }));
    // }));
    // widget.add_controller(gesture);

    update_device(
        global_label,
        connected_devices,
        devices,
        &device,
        &widget,
        &mut is_connected,
    )
    .await;

    Some(widget)
}

async fn update_device(
    global_label: &Label,
    connected_devices: &gtk::Box,
    devices: &gtk::Box,
    device: &DeviceProxy<'_>,
    widget: &Label,
    is_connected: &mut bool,
) {
    widget.set_text(
        ICONS
            .get(device.Icon().await.unwrap().as_str())
            .unwrap_or(&"?"),
    );
    widget.set_better_tooltip(Some(device.Alias().await.unwrap()));

    let is_now_connected = device.Connected().await.unwrap();

    // TODO: this does not work if a device is removed without being disconnected
    // now it works, but i'm not sure it is very clean
    if !*is_connected && is_now_connected {
        devices.remove(widget);
        connected_devices.append(widget);

        let old_count = CONNECTED_COUNT.fetch_add(1, Ordering::SeqCst);
        if old_count == 0 {
            global_label.set_text("󰂱");
            global_label.add_css_class("connected");
            connected_devices.set_visible(true);
        }
    } else if *is_connected && !is_now_connected {
        connected_devices.remove(widget);
        devices.append(widget);

        let old_count = CONNECTED_COUNT.fetch_sub(1, Ordering::SeqCst);
        if old_count == 1 {
            global_label.set_text("󰂯");
            global_label.remove_css_class("connected");
            connected_devices.set_visible(false);
        }
    }
    *is_connected = is_now_connected;
}
