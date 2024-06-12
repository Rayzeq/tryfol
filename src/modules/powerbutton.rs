use gtk::{prelude::*, Button};
use gtk4 as gtk;
use log::error;
use std::process::Command;

pub fn new() -> Button {
    let button = Button::builder()
        .name("power")
        .css_classes(["module"])
        .label("")
        .build();
    button.connect_clicked(|_| {
        if let Err(e) = Command::new("wlogout").arg("-p").arg("layer-shell").spawn() {
            error!("Failed to start wlogout: {}", e);
        }
    });

    button
}
