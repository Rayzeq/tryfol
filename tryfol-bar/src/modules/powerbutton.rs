use gtk::{Button, glib, prelude::*};
use gtk4 as gtk;
use log::error;
use tokio::process::Command;

pub fn new() -> Button {
    let button = Button::builder()
        .name("power")
        .css_classes(["module"])
        .label("")
        .build();
    button.connect_clicked(|_| {
        match Command::new("wlogout").arg("-p").arg("layer-shell").spawn() {
            Ok(mut child) => {
                // Prevent zombie process
                glib::spawn_future_local(async move { child.wait().await });
            }
            Err(e) => error!("Failed to start wlogout: {}", e),
        }
    });

    button
}
