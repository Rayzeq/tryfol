use crate::{
    backend::battery::{self, Status},
    Hoverable,
};
use gtk::{
    glib::{self, ControlFlow},
    prelude::*,
    Label, Orientation, Revealer, RevealerTransitionType,
};
use gtk4 as gtk;
use lazy_static::lazy_static;
use log::error;
use std::{path::Path, time::Duration};

lazy_static! {
    static ref BATTERY_PATH: &'static Path = &Path::new("/sys/class/power_supply/BAT0");
}

pub fn new() -> gtk::Box {
    let percentage = Label::new(None);
    let health = Label::builder().css_classes(["right"]).build();
    let time_to = Label::builder().css_classes(["right"]).build();

    let details = gtk::Box::new(Orientation::Horizontal, 0);
    details.append(&time_to);
    details.append(&health);
    let revealer = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideLeft)
        .transition_duration(500)
        .child(&details)
        .build();

    let root = gtk::Box::new(Orientation::Horizontal, 0);
    root.append(&percentage);
    root.append(&revealer);
    root.connect_hover_notify(move |_, hovered| {
        revealer.set_reveal_child(hovered);
    });

    update(&percentage, &health, &time_to);
    glib::timeout_add_local(Duration::from_secs(2), move || {
        update(&percentage, &health, &time_to);
        ControlFlow::Continue
    });

    root
}

fn update(percentage: &Label, health: &Label, time_to: &Label) {
    if let Err(e) = try_update(percentage, health, time_to) {
        error!("Error updating battery module: {e}");
    }
}

fn try_update(
    percentage_label: &Label,
    health_label: &Label,
    time_to_label: &Label,
) -> anyhow::Result<()> {
    let reader = battery::Reader::new(*BATTERY_PATH);

    let capacity = reader.capacity()?;
    let energy_full = reader.energy_full()?;
    // present_rate is 0 for some seconds after the AC has been plugged
    let present_rate = reader.power_now()?;
    let voltage = reader.voltage_now()? / 1000;

    let remaining_capacity = reader.energy_now()? / voltage;
    let present_rate = present_rate / voltage;
    let last_capacity = energy_full / voltage;
    let health = (energy_full * 100) as f64 / reader.energy_full_design()? as f64;

    let (icon, class, seconds) = match reader.status()? {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Status::Discharging => {
            let icon = ["󰂎", "󰁺", "󰁻", "󰁼", "󰁽", "󰁾", "󰁿", "󰂀", "󰂁", "󰂂", "󰁹"]
                [(capacity as f64 / 10.).round() as usize];
            let seconds = if present_rate > 0 {
                Some(3600 * remaining_capacity / present_rate)
            } else {
                None
            };
            (icon, "discharging", seconds)
        }
        Status::Charging => {
            let seconds = if present_rate > 0 {
                Some(3600 * (last_capacity - remaining_capacity) / present_rate)
            } else {
                None
            };
            ("󰂄", "charging", seconds)
        }
        Status::Full => ("", "full", None),
        Status::NotCharging => ("", "not-charging", None),
        Status::Unknown => ("?", "", None),
    };

    percentage_label.set_css_classes(&[]);
    percentage_label.add_css_class(class);
    if capacity <= 5 {
        percentage_label.add_css_class("critical");
    } else if capacity <= 15 {
        percentage_label.add_css_class("warning");
    }

    percentage_label.set_text(&format!("{icon}  {capacity}%"));
    health_label.set_text(&format!("󰣐  {health:.0}%"));
    if let Some(seconds) = seconds {
        let minutes = seconds / 60;
        let (hours, minutes) = (minutes / 60, minutes % 60);

        if hours == 0 {
            time_to_label.set_text(&format!("󰅐  {minutes}min"));
        } else {
            time_to_label.set_text(&format!("󰅐  {hours}h {minutes}min"));
        }

        time_to_label.set_visible(true);
    } else {
        time_to_label.set_visible(false);
    }

    Ok(())
}
