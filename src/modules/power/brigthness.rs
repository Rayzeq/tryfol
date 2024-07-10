use crate::{backend::logind::SessionProxy, Scrollable};
use anyhow::Context;
use gtk::{
    gio::{Cancellable, Socket},
    glib::{self, clone, IOCondition, Priority},
    prelude::*,
    Label,
};
use gtk4::{self as gtk, glib::JoinHandle};
use log::{error, warn};
use std::{convert::Infallible, io, os::fd::AsRawFd};
use udev::{Enumerator, MonitorBuilder};
use zbus::Connection;

#[derive(Debug, Clone)]
struct Device {
    inner: udev::Device,
}

pub fn new() -> Label {
    let label = Label::new(None);

    // use the first found device
    let device = match Device::get_all().map(|devices| devices.into_iter().next()) {
        Ok(Some(x)) => x,
        Ok(None) => {
            warn!("No backlight device found");
            label.set_visible(false);
            return label;
        }
        Err(e) => {
            error!("Cannot get backlight devices: {e}");
            label.set_text("ERROR");
            return label;
        }
    };
    update(&device, &label);

    let device_cpy = device.clone();
    label.connect_vertical_scroll(move |_, dy| {
        let device = device_cpy.clone();
        glib::spawn_future_local(async move {
            let current_brightness = match device.brightness() {
                Ok(x) => x as f64,
                Err(e) => {
                    error!("Cannot get device brightness: {e}");
                    return;
                }
            };
            let max_brightness = match device.max_brightness() {
                Ok(x) => x as f64,
                Err(e) => {
                    error!("Cannot get device max brightness: {e}");
                    return;
                }
            };

            let five_percent = max_brightness * 0.05;
            let new_brightness = five_percent.mul_add(-dy, current_brightness);

            // correct the value to always be on a 5% multiple
            let new_brightness = ((new_brightness / five_percent).round() * five_percent)
                .round()
                .clamp(0., max_brightness);

            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let result = device.set_brightness(new_brightness as u32).await;
            if let Err(e) = result {
                error!("Cannot set device brightness: {e}");
            }
        });
    });

    let task = Device::connect_on_update(clone!(@weak label => move |updated_device| {
        if updated_device == device {
            update(&updated_device, &label);
        }
    }));
    label.connect_destroy(move |_| task.abort());

    label
}

fn update(device: &Device, label: &Label) {
    let current_brightness = match device.brightness() {
        Ok(x) => x as f64,
        Err(e) => {
            error!("Cannot get device brightness: {e}");
            label.set_text("ERROR");
            return;
        }
    };
    let max_brightness = match device.max_brightness() {
        Ok(x) => x as f64,
        Err(e) => {
            error!("Cannot get device max brightness: {e}");
            label.set_text("ERROR");
            return;
        }
    };
    let percentage = (current_brightness * 100.) / max_brightness;

    label.set_text(&format!("󰖙  {percentage:.0}%"));
}

impl Device {
    pub fn get_all() -> io::Result<Vec<Self>> {
        let mut enumerator = Enumerator::new()?;
        enumerator.match_subsystem("backlight")?;
        Ok(enumerator
            .scan_devices()?
            .map(|device| Self { inner: device })
            .collect())
    }

    pub fn brightness(&self) -> anyhow::Result<u32> {
        Ok(self
            .inner
            .attribute_value("actual_brightness")
            .context("Missing attribute `actual_brightness` on device")?
            .to_str()
            .context("Invalid utf8")?
            .parse()?)
    }

    pub async fn set_brightness(&self, value: u32) -> anyhow::Result<()> {
        // we need to use dbus and go through logind because we don't have permission to directly change the brightness
        let session = SessionProxy::new(&Connection::system().await?).await?;
        session
            .set_brightness(
                self.inner
                    .subsystem()
                    .context("Missing subsystem for device")?
                    .to_str()
                    .context("Invalid utf8")?,
                self.inner.sysname().to_str().context("Invalid utf8")?,
                value,
            )
            .await?;

        Ok(())
    }

    pub fn max_brightness(&self) -> anyhow::Result<u32> {
        Ok(self
            .inner
            .attribute_value("max_brightness")
            .context("Missing attribute `max_brightness` on device")?
            .to_str()
            .context("Invalid utf8")?
            .parse()?)
    }

    pub fn connect_on_update<F>(f: F) -> JoinHandle<()>
    where
        F: Fn(Self) + 'static,
    {
        glib::spawn_future_local(async move {
            if let Err(e) = Self::listen(f).await {
                error!("Cannot listen backlight changes: {e}");
            }
        })
    }

    async fn listen<F>(f: F) -> anyhow::Result<Infallible>
    where
        F: Fn(Self),
    {
        let socket = MonitorBuilder::new()?
            .match_subsystem("backlight")?
            .listen()?;
        let gio_socket = unsafe { Socket::from_fd(socket.as_raw_fd()) }?;

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

            f(Self {
                inner: event.device(),
            });
        }
    }
}

impl PartialEq for Device {
    fn eq(&self, other: &Self) -> bool {
        self.inner.sysname() == other.inner.sysname()
    }
}

impl Eq for Device {}
