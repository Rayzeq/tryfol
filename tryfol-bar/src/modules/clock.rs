use chrono::{Local, Locale};
use gtk::{
    Button, Calendar, Label, Popover,
    glib::{self, ControlFlow, DateTime},
    prelude::*,
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

    button.connect_clicked(move |_| {
        match DateTime::now_local() {
            Ok(datetime) => calendar.select_day(&datetime),
            Err(e) => error!("Cannot get current date: {e}"),
        }
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
