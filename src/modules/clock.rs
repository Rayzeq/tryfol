use chrono::{Local, Locale};
use gtk::{
    glib::{self, ControlFlow, DateTime},
    prelude::*,
    Button, Calendar, Label, Popover,
};
use gtk4 as gtk;
use log::error;
use std::time::Duration;

pub fn new() -> Button {
    let label = Label::new(Some(&get_time()));
    let button = Button::builder()
        .name("clock")
        .css_classes(["module"])
        .child(&label)
        .build();

    let calendar = Calendar::builder().show_week_numbers(true).build();
    let popover = Popover::builder()
        .name("calendar")
        .has_arrow(false)
        .child(&calendar)
        .build();
    popover.set_parent(&button);

    button.connect_clicked(move |this| {
        match DateTime::now_local() {
            Ok(datetime) => calendar.select_day(&datetime),
            Err(e) => error!("Cannot get current date: {e}"),
        }
        // 8 is a magic value because I don't want to read the positionning code for popovers,
        // so I have no idea where it comes from
        popover.set_offset(-this.width() + 8, 0);
        popover.popup();
    });

    glib::timeout_add_local(Duration::from_secs(1), move || {
        label.set_text(&get_time());
        ControlFlow::Continue
    });

    button
}

fn get_time() -> String {
    Local::now()
        .format_localized("%d %b %Y %T", Locale::fr_FR)
        .to_string()
}
