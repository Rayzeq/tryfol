use gtk::{
    gio::{self, BusType, Cancellable, DBusCallFlags, Socket},
    glib::{self, clone, IOCondition, Priority},
    prelude::*,
    EventControllerScroll, EventControllerScrollFlags, Label,
};
use gtk4 as gtk;
use std::{io, os::fd::AsRawFd};
use udev::{Device, Enumerator, MonitorBuilder, MonitorSocket};

pub fn new() -> Label {
    let label = Label::new(None);
    label.add_css_class("right");

    update(&get_device(), &label);

    let scroll_detector = EventControllerScroll::new(
        EventControllerScrollFlags::VERTICAL | EventControllerScrollFlags::DISCRETE,
    );
    scroll_detector.connect_scroll(move |_, _, dy| {
        glib::spawn_future_local(async move {
            let device = get_device();
            let actual_brightness: u32 = device
                .attribute_value("actual_brightness")
                .unwrap()
                .to_str()
                .unwrap()
                .parse()
                .unwrap();
            let max_brightness: u32 = device
                .attribute_value("max_brightness")
                .unwrap()
                .to_str()
                .unwrap()
                .parse()
                .unwrap();

            let five_percent = max_brightness as f64 * 0.05;
            let new_brightness = five_percent.mul_add(-dy, actual_brightness as f64);

            // correct the value to always be on a 5% multiple
            let new_brightness = ((new_brightness / five_percent).round() * five_percent)
                .round()
                .clamp(0., max_brightness as f64) as u32;

            // we need to use dbus and go through logind because we don't have permission to directly change the brightness
            let system_bus = gio::bus_get_future(BusType::System).await.unwrap();
            system_bus
                .call_future(
                    Some("org.freedesktop.login1"),
                    "/org/freedesktop/login1/session/auto",
                    "org.freedesktop.login1.Session",
                    "SetBrightness",
                    Some(
                        &(
                            device.subsystem().unwrap().to_str().unwrap(),
                            device.sysname().to_str().unwrap(),
                            new_brightness,
                        )
                            .into(),
                    ),
                    None,
                    DBusCallFlags::empty(),
                    -1,
                )
                .await
                .unwrap();
        });
        glib::Propagation::Proceed
    });
    label.add_controller(scroll_detector);

    glib::spawn_future_local(clone!(@strong label => async move {
        let socket = MonitorBuilder::new()
            .unwrap()
            .match_subsystem("backlight")
            .unwrap()
            .listen()
            .unwrap();

        poll(&socket, &label).await.unwrap();
    }));

    label
}

fn get_device() -> Device {
    let mut enumerator = Enumerator::new().unwrap();
    enumerator.match_subsystem("backlight").unwrap();
    enumerator.scan_devices().unwrap().next().unwrap()
}

async fn poll(socket: &MonitorSocket, label: &Label) -> io::Result<()> {
    let gio_socket = unsafe { Socket::from_fd(socket.as_raw_fd()) }.unwrap();

    loop {
        SocketExtManual::create_source_future::<Cancellable>(
            &gio_socket,
            IOCondition::IN,
            None,
            Priority::HIGH_IDLE,
        )
        .await;

        let Some(event) = socket.iter().next() else {
            continue;
        };

        update(&event.device(), label);
    }
}

fn update(device: &Device, label: &Label) {
    let actual_brightness: u32 = device
        .attribute_value("actual_brightness")
        .unwrap()
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    let max_brightness: u32 = device
        .attribute_value("max_brightness")
        .unwrap()
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    let percentage = (actual_brightness * 100) as f64 / max_brightness as f64;

    label.set_text(&format!("󰖙  {percentage:.0}%"));
}
