mod battery;
mod brigthness;

use gtk4::{self as gtk, prelude::*};

pub fn new() -> gtk::Box {
    let brightness = brigthness::new();
    brightness.add_css_class("right");

    let root = gtk::Box::builder()
        .name("battery")
        .css_classes(["module"])
        .build();
    root.append(&battery::new());
    root.append(&brightness);

    root
}
