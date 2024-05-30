use gtk::prelude::*;
use gtk4 as gtk;
use std::process::Command;

pub fn new() -> gtk::Button {
    let button = gtk::Button::with_label("");
    button.set_widget_name("power");
    button.add_css_class("module");
    button.connect_clicked(|_| {
        Command::new("wlogout")
            .arg("-p")
            .arg("layer-shell")
            .spawn()
            .unwrap();
    });

    button
}
