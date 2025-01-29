use gtk4::{self as gtk, prelude::*};

mod battery;
mod brigthness;
mod darkman;

pub fn new() -> gtk::Box {
    let darkman = darkman::new();
    let battery = battery::new();
    battery.add_css_class("left");
    darkman.add_css_class("right");

    let root = gtk::Box::builder()
        .name("battery")
        .css_classes(["module"])
        .build();
    root.append(&battery);
    root.append(&brigthness::new());
    root.append(&darkman);

    root
}
