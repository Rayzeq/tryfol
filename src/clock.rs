use chrono::{Datelike, Local, Locale};
use gtk::{
    glib::{self, clone},
    prelude::*,
    Button, Calendar, Label, Popover,
};
use gtk4 as gtk;
use std::time::Duration;

fn get_time() -> String {
    Local::now()
        .format_localized("%d %b %Y %T", Locale::fr_FR)
        .to_string()
}

pub fn new() -> Button {
    let button = Button::new();

    let label = Label::new(Some(&get_time()));
    label.set_widget_name("clock");
    label.add_css_class("module");
    button.set_child(Some(&label));

    let popover = Popover::new();
    popover.set_has_arrow(false);
    popover.set_parent(&button);
    popover.set_widget_name("calendar");
    let calendar = Calendar::new();
    calendar.set_show_week_numbers(true);
    popover.set_child(Some(&calendar));

    button.connect_clicked(move |_| {
        let now = chrono::offset::Local::now();
        calendar.set_day(now.day() as i32);
        calendar.set_month(now.month0() as i32);
        calendar.set_year(now.year());
        popover.popup();
    });

    glib::timeout_add_local(
        Duration::from_secs(1),
        clone!(@strong label => move || {
            label.set_text(&get_time());
            glib::ControlFlow::Continue
        }),
    );

    button
}
