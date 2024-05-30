use gtk::{
    glib::{self, clone},
    prelude::*,
    EventControllerMotion, Label, Orientation, Revealer, RevealerTransitionType,
};
use gtk4 as gtk;
use std::{fs, path::Path, time::Duration};

pub fn new() -> gtk::Box {
    let bat_path = Path::new("/sys/class/power_supply/BAT0");

    let inner_container = gtk::Box::new(Orientation::Horizontal, 0);
    let revealer = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideLeft)
        .transition_duration(500)
        .child(&inner_container)
        .build();

    let container = gtk::Box::new(Orientation::Horizontal, 0);
    let event_controller = EventControllerMotion::new();
    event_controller.connect_enter(clone!(@strong revealer => move |_, _, _| {
        revealer.set_reveal_child(true);
    }));
    event_controller.connect_leave(clone!(@strong revealer => move |_| {
        revealer.set_reveal_child(false);
    }));
    container.add_controller(event_controller);

    let percentage_label = Label::new(None);
    let health_label = Label::new(None);
    let time_to_label = Label::new(None);

    health_label.add_css_class("right");
    time_to_label.add_css_class("right");

    container.append(&percentage_label);
    container.append(&revealer);
    inner_container.append(&time_to_label);
    inner_container.append(&health_label);

    update(bat_path, &percentage_label, &health_label, &time_to_label);

    glib::timeout_add_local(Duration::from_secs(2), move || {
        update(bat_path, &percentage_label, &health_label, &time_to_label);
        glib::ControlFlow::Continue
    });

    container
}

fn update(bat_path: &Path, percentage_label: &Label, health_label: &Label, time_to_label: &Label) {
    let s = fs::read_to_string(bat_path.join("status")).unwrap();
    let status = s.trim();
    let capacity: u32 = fs::read_to_string(bat_path.join("capacity"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let energy_full: u32 = fs::read_to_string(bat_path.join("energy_full"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let energy_full_design: u32 = fs::read_to_string(bat_path.join("energy_full_design"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    let remaining_energy = fs::read_to_string(bat_path.join("energy_now"))
        .unwrap()
        .trim()
        .parse::<u32>()
        .unwrap();
    // present_rate is 0 for some seconds after the AC has been plugged
    let present_rate = fs::read_to_string(bat_path.join("power_now"))
        .unwrap()
        .trim()
        .parse::<u32>()
        .unwrap();
    let voltage = fs::read_to_string(bat_path.join("voltage_now"))
        .unwrap()
        .trim()
        .parse::<u32>()
        .unwrap()
        / 1000;

    let remaining_capacity = remaining_energy / voltage;
    let present_rate = present_rate / voltage;
    let last_capacity = energy_full / voltage;
    let health = (energy_full * 100) as f64 / energy_full_design as f64;

    let (icon, class, seconds) = match status {
        "Discharging" => {
            let icon = ["󰂎", "󰁺", "󰁻", "󰁼", "󰁽", "󰁾", "󰁿", "󰂀", "󰂁", "󰂂", "󰁹"]
                [(capacity as f64 / 10.).round() as usize];
            let seconds = if present_rate > 0 {
                3600 * remaining_capacity / present_rate
            } else {
                0
            };
            (icon, "discharging", seconds)
        }
        "Charging" => {
            let seconds = if present_rate > 0 {
                3600 * (last_capacity - remaining_capacity) / present_rate
            } else {
                0
            };
            ("󰂄", "charging", seconds)
        }
        "Full" => ("", "full", 0),
        "Not charging" => ("", "not-charging", 0),
        _ => {
            eprintln!("Unknown status: {status}");
            ("?", "", 0)
        }
    };
    percentage_label.remove_css_class("discharging");
    percentage_label.remove_css_class("charging");
    percentage_label.remove_css_class("full");
    percentage_label.remove_css_class("not-charging");
    percentage_label.add_css_class(class);

    percentage_label.remove_css_class("critical");
    percentage_label.remove_css_class("warning");
    if capacity <= 5 {
        percentage_label.add_css_class("critical");
    } else if capacity <= 15 {
        percentage_label.add_css_class("warning");
    }

    let minutes = seconds / 60;
    let (hours, minutes) = (minutes / 60, minutes % 60);

    if seconds == 0 {
        time_to_label.set_visible(false);
    } else {
        time_to_label.set_visible(true);
    }

    percentage_label.set_text(&format!("{icon}  {capacity}%"));
    health_label.set_text(&format!("󰣐  {health:.0}%"));
    if hours == 0 {
        time_to_label.set_text(&format!("󰅐  {minutes}min"));
    } else {
        time_to_label.set_text(&format!("󰅐  {hours}h {minutes}min"));
    }
}
