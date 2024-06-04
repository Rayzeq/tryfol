use crate::widget_ext::HasTooltip;
use gtk::{
    glib::{self, clone},
    prelude::*,
    EventControllerMotion, Label, Orientation, Revealer, RevealerTransitionType,
};
use gtk4 as gtk;
use psutil::{
    cpu::CpuPercentCollector,
    memory::{os::linux::VirtualMemoryExt, swap_memory, virtual_memory},
    sensors::temperatures,
};
use std::time::Duration;

#[derive(Debug)]
pub struct Modules {
    pub cpu: gtk::Box,
    pub memory: gtk::Box,
    pub temperatures: gtk::Box,
}

pub fn new() -> Modules {
    let cpu_percent = Label::new(None);
    let cpu_freq = Label::new(None);
    let cpu_revealer = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideLeft)
        .transition_duration(500)
        .child(&cpu_freq)
        .build();

    let cpu = gtk::Box::new(Orientation::Horizontal, 0);
    cpu.set_widget_name("cpu");
    cpu.add_css_class("module");
    cpu.append(&cpu_percent);
    cpu.append(&cpu_revealer);

    let event_controller = EventControllerMotion::new();
    event_controller.connect_enter(clone!(@strong cpu_revealer => move |_, _, _| {
        cpu_revealer.set_reveal_child(true);
    }));
    event_controller.connect_leave(clone!(@strong cpu_revealer => move |_| {
        cpu_revealer.set_reveal_child(false);
    }));
    cpu.add_controller(event_controller);

    let memory_percent = Label::new(None);
    let memory_absolutes = gtk::Box::new(Orientation::Horizontal, 0);
    let memory_left = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideRight)
        .transition_duration(500)
        .reveal_child(true)
        .child(&memory_percent)
        .build();
    let memory_right = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideLeft)
        .transition_duration(500)
        .child(&memory_absolutes)
        .build();

    let memory_absolute = Label::new(None);
    let memory_cached = Label::new(None);
    let swap = Label::new(None);
    memory_absolute.add_css_class("left");
    swap.add_css_class("right");
    memory_absolutes.append(&memory_absolute);
    memory_absolutes.append(&memory_cached);
    memory_absolutes.append(&swap);

    let memory = gtk::Box::new(Orientation::Horizontal, 0);
    memory.set_widget_name("memory");
    memory.add_css_class("module");
    memory.append(&Label::new(Some(" ")));
    memory.append(&memory_left);
    memory.append(&memory_right);

    let event_controller = EventControllerMotion::new();
    event_controller.connect_enter(
        clone!(@strong memory_left, @strong memory_right => move |_, _, _| {
            memory_left.set_reveal_child(false);
            memory_right.set_reveal_child(true);
        }),
    );
    event_controller.connect_leave(
        clone!(@strong memory_left, @strong memory_right => move |_| {
            memory_left.set_reveal_child(true);
            memory_right.set_reveal_child(false);
        }),
    );
    memory.add_controller(event_controller);

    let cpu_temperature = Label::new(None);
    let disk_temperature = Label::new(None);
    let hidden_temps = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideLeft)
        .transition_duration(500)
        .child(&disk_temperature)
        .build();
    disk_temperature.add_css_class("right");

    let temperatures = gtk::Box::new(Orientation::Horizontal, 0);
    temperatures.set_widget_name("temperatures");
    temperatures.add_css_class("module");
    temperatures.append(&Label::builder().css_classes(["left"]).label("").build());
    temperatures.append(&cpu_temperature);
    temperatures.append(&hidden_temps);

    let event_controller = EventControllerMotion::new();
    event_controller.connect_enter(clone!(@strong hidden_temps => move |_, _, _| {
        hidden_temps.set_reveal_child(true);
    }));
    event_controller.connect_leave(clone!(@strong hidden_temps => move |_| {
        hidden_temps.set_reveal_child(false);
    }));
    temperatures.add_controller(event_controller);

    let mut cpu_collector = CpuPercentCollector::new().unwrap();
    update(
        &cpu,
        &cpu_percent,
        &cpu_freq,
        &mut cpu_collector,
        &memory_percent,
        &memory_absolute,
        &memory_cached,
        &swap,
        &cpu_temperature,
        &disk_temperature,
    );

    glib::timeout_add_local(
        Duration::from_secs(1),
        clone!(@strong cpu => move || {
            update(
                &cpu,
                &cpu_percent,
                &cpu_freq,
                &mut cpu_collector,
                &memory_percent,
                &memory_absolute,
                &memory_cached,
                &swap,
                &cpu_temperature,
                &disk_temperature,
            );
            glib::ControlFlow::Continue
        }),
    );

    Modules {
        cpu,
        memory,
        temperatures,
    }
}

fn update(
    cpu: &gtk::Box,
    cpu_percent: &Label,
    cpu_freq: &Label,
    cpu_collector: &mut CpuPercentCollector,
    memory_percent: &Label,
    memory_absolute: &Label,
    memory_cached: &Label,
    swap: &Label,
    cpu_temperature: &Label,
    disk_temperature: &Label,
) {
    let cpu_p = cpu_collector.cpu_percent().unwrap();
    cpu_percent.set_text(&format!("󰻠  {cpu_p:.0}%"));
    cpu_percent.remove_css_class("warning");
    cpu_percent.remove_css_class("critical");
    if cpu_p > 90. {
        cpu_percent.add_css_class("critical");
    } else if cpu_p > 70. {
        cpu_percent.add_css_class("warning");
    }
    cpu_freq.set_text(&format!(
        " @ {:.2}GHz",
        psutil::cpu::cpu_freq().unwrap().unwrap().current() / 1000.
    ));
    cpu_freq.set_text(&format!(
        " @ {:.2}GHz",
        psutil::cpu::cpu_freq().unwrap().unwrap().current() / 1000.
    ));
    cpu.set_better_tooltip_markup(Some(
        cpu_collector
            .cpu_percent_percpu()
            .unwrap()
            .into_iter()
            .enumerate()
            .map(|(i, percent)| format!("Cpu <tt>{i:2}</tt>: <tt>{percent:3.0}</tt>%"))
            .collect::<Vec<_>>()
            .join("\n"),
    ));

    let vmem = virtual_memory().unwrap();
    let swap_mem = swap_memory().unwrap();

    let mem_cached = vmem.buffers() + vmem.cached();
    let mem_absolute = (vmem.total() - vmem.free()) - mem_cached;
    let mem_percent = mem_absolute * 100 / vmem.total();
    let swap_used = swap_mem.used();

    memory_percent.set_text(&format!("{mem_percent}%"));
    memory_absolute.set_text(&format_bytes(mem_absolute));
    memory_cached.set_text(&format!("  {}", format_bytes(mem_cached)));
    swap.set_text(&format!("󰋊  {}", format_bytes(swap_used)));

    memory_percent.remove_css_class("warning");
    memory_percent.remove_css_class("critical");
    if mem_percent > 90 {
        memory_percent.add_css_class("critical");
    } else if mem_percent > 70 {
        memory_percent.add_css_class("warning");
    }

    let temps = temperatures();
    let cpu_temp = temps
        .iter()
        .find(|temp| temp.as_ref().map_or(false, |temp| temp.unit() == "acpitz"))
        .unwrap()
        .as_ref()
        .unwrap();
    let disk_temp = temps
        .iter()
        .find(|temp| temp.as_ref().map_or(false, |temp| temp.unit() == "nvme"))
        .unwrap()
        .as_ref()
        .unwrap();
    cpu_temperature.set_text(&format!("󰘚  {:.0}°C", cpu_temp.current().celsius()));
    disk_temperature.set_text(&format!("󰋊  {:.0}°C", disk_temp.current().celsius()));

    cpu_temperature.remove_css_class("high");
    cpu_temperature.remove_css_class("critical");
    disk_temperature.remove_css_class("high");
    disk_temperature.remove_css_class("critical");
    if let Some(high) = cpu_temp.high() {
        if cpu_temp.current().celsius() > high.celsius() {
            cpu_temperature.add_css_class("high");
        }
    }
    if let Some(critical) = cpu_temp.critical() {
        if cpu_temp.current().celsius() > critical.celsius() {
            cpu_temperature.add_css_class("critical");
        }
    }
    if let Some(high) = disk_temp.high() {
        if disk_temp.current().celsius() > high.celsius() {
            disk_temperature.add_css_class("high");
        }
    }
    if let Some(critical) = disk_temp.critical() {
        if disk_temp.current().celsius() > critical.celsius() {
            disk_temperature.add_css_class("critical");
        }
    }
}

fn format_bytes(quantity: u64) -> String {
    const UNIT_MULTIPLIER: f64 = 1024.;
    const UNIT_PREFIXES: &[&str] = &["", "k", "M", "G", "T"];

    let mut quantity = quantity as f64;
    let mut i = 0;
    while quantity > 1000. {
        quantity /= UNIT_MULTIPLIER;
        i += 1;
    }

    format!("{}{}B", fixed_width(quantity, 3), UNIT_PREFIXES[i])
}

fn fixed_width(number: f64, width: usize) -> String {
    let int_part = number as usize;
    let int_len = int_part.to_string().len();
    if int_len >= width {
        format!("{number:.0}")
    } else if int_len + 1 == width {
        format!(" {number:.0}")
    } else {
        format!("{number:.0$}", width - (int_len + 1))
    }
}
