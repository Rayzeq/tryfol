use crate::{FormatFixed, HasTooltip, Hoverable};
use gtk::{Label, Orientation, Revealer, RevealerTransitionType, glib, prelude::*};
use gtk4 as gtk;
use log::error;
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

struct CpuModule {
    pub root: gtk::Box,
    percent: Label,
    frequency: Label,
    collector: Option<CpuPercentCollector>,
}

struct MemoryModule {
    pub root: gtk::Box,
    percent: Label,
    absolute: Label,
    cache: Label,
    swap: Label,
}

struct TemperaturesModule {
    pub root: gtk::Box,
    cpu: Label,
    disk: Label,
}

impl Modules {
    pub fn new() -> Self {
        let mut cpu = CpuModule::new();
        let memory = MemoryModule::new();
        let temperatures = TemperaturesModule::new();

        let this = Self {
            cpu: cpu.root.clone(),
            memory: memory.root.clone(),
            temperatures: temperatures.root.clone(),
        };

        Self::update(&mut cpu, &memory, &temperatures);
        glib::timeout_add_local(Duration::from_secs(1), move || {
            Self::update(&mut cpu, &memory, &temperatures);
            glib::ControlFlow::Continue
        });

        this
    }

    fn update(cpu: &mut CpuModule, mem: &MemoryModule, temps: &TemperaturesModule) {
        if let Err(e) = (|| {
            cpu.update()?;
            mem.update()?;
            temps.update()?;

            Ok::<_, anyhow::Error>(())
        })() {
            error!("Cannot update modules: {e}");
        }
    }
}

impl CpuModule {
    pub fn new() -> Self {
        let percent = Label::new(None);
        let frequency = Label::new(None);
        let revealer = Revealer::builder()
            .transition_type(RevealerTransitionType::SlideLeft)
            .transition_duration(500)
            .child(&frequency)
            .build();

        let root = gtk::Box::builder()
            .name("cpu")
            .css_classes(["module"])
            .build();
        root.append(&percent);
        root.append(&revealer);

        root.connect_hover_notify(move |_, hovered| {
            revealer.set_reveal_child(hovered);
        });

        let collector = CpuPercentCollector::new()
            .map_err(|e| {
                error!("Cannot create CPU percent collector: {e}");
            })
            .ok();

        Self {
            root,
            percent,
            frequency,
            collector,
        }
    }

    pub fn update(&mut self) -> psutil::Result<()> {
        let Some(ref mut collector) = self.collector else {
            return Ok(());
        };

        let percentage = collector.cpu_percent()?;
        self.percent.set_text(&format!("󰻠  {percentage:.0}%"));
        if percentage > 90. {
            self.percent.set_css_classes(&["critical"]);
        } else if percentage > 70. {
            self.percent.set_css_classes(&["warning"]);
        } else {
            self.percent.set_css_classes(&[]);
        }

        if let Some(frequencey) = psutil::cpu::cpu_freq()? {
            self.frequency.set_visible(true);
            self.frequency
                .set_text(&format!(" @ {:.2}GHz", frequencey.current() / 1000.));
        } else {
            self.frequency.set_visible(false);
        }

        self.root.set_better_tooltip_markup(Some(
            collector
                .cpu_percent_percpu()?
                .into_iter()
                .enumerate()
                .map(|(i, percent)| format!("Cpu <tt>{i:2}</tt>: <tt>{percent:3.0}</tt>%"))
                .collect::<Vec<_>>()
                .join("\n"),
        ));

        Ok(())
    }
}

impl MemoryModule {
    pub fn new() -> Self {
        let percent = Label::new(None);
        let absolute = Label::builder().css_classes(["left"]).build();
        let cache = Label::new(None);
        let swap = Label::builder().css_classes(["right"]).build();

        let absolutes_values = gtk::Box::new(Orientation::Horizontal, 0);
        absolutes_values.append(&absolute);
        absolutes_values.append(&cache);
        absolutes_values.append(&swap);

        let left_revealer = Revealer::builder()
            .transition_type(RevealerTransitionType::SlideRight)
            .transition_duration(500)
            .reveal_child(true)
            .child(&percent)
            .build();
        let right_revealer = Revealer::builder()
            .transition_type(RevealerTransitionType::SlideLeft)
            .transition_duration(500)
            .child(&absolutes_values)
            .build();

        let root = gtk::Box::builder()
            .name("memory")
            .css_classes(["module"])
            .build();
        root.append(&Label::new(Some(" ")));
        root.append(&left_revealer);
        root.append(&right_revealer);

        root.connect_hover_notify(move |_, hovered| {
            left_revealer.set_reveal_child(!hovered);
            right_revealer.set_reveal_child(hovered);
        });

        Self {
            root,
            percent,
            absolute,
            cache,
            swap,
        }
    }

    pub fn update(&self) -> psutil::Result<()> {
        let vmem = virtual_memory()?;
        let swap_mem = swap_memory()?;

        let cache = vmem.buffers() + vmem.cached();
        let absolute = (vmem.total() - vmem.free()) - cache;
        let percent = absolute * 100 / vmem.total();
        let swap = swap_mem.used();

        self.percent.set_text(&format!("{percent}%"));
        self.absolute.set_text(&format_bytes(absolute));
        self.cache.set_text(&format!("  {}", format_bytes(cache)));
        self.swap.set_text(&format!("󰋊  {}", format_bytes(swap)));

        if percent > 90 {
            self.percent.set_css_classes(&["critical"]);
        } else if percent > 70 {
            self.percent.set_css_classes(&["warning"]);
        } else {
            self.percent.set_css_classes(&[]);
        }

        Ok(())
    }
}

impl TemperaturesModule {
    pub fn new() -> Self {
        let cpu = Label::new(None);
        let disk = Label::builder().css_classes(["right"]).build();
        let hidden_temperatures = Revealer::builder()
            .transition_type(RevealerTransitionType::SlideLeft)
            .transition_duration(500)
            .child(&disk)
            .build();

        let root = gtk::Box::builder()
            .name("temperatures")
            .css_classes(["module"])
            .build();
        root.append(&Label::builder().css_classes(["left"]).label("").build());
        root.append(&cpu);
        root.append(&hidden_temperatures);

        root.connect_hover_notify(move |_, hovered| {
            hidden_temperatures.set_reveal_child(hovered);
        });

        Self { root, cpu, disk }
    }

    pub fn update(&self) -> anyhow::Result<()> {
        let temperatures = temperatures();

        let mut cpu = None;
        let mut disk = None;
        for temp in temperatures {
            let temp = temp?;
            if temp.unit() == "acpitz" {
                cpu = Some(temp);
            } else if temp.unit() == "nvme" {
                disk = Some(temp);
            }

            if cpu.is_some() && disk.is_some() {
                break;
            }
        }

        if let Some(cpu) = cpu {
            self.cpu
                .set_text(&format!("󰘚  {:.0}°C", cpu.current().celsius()));

            self.cpu.remove_css_class("critical");
            self.cpu.remove_css_class("high");
            let critical_set = match cpu.critical() {
                Some(critical) if cpu.current().celsius() > critical.celsius() => {
                    self.cpu.add_css_class("critical");
                    true
                }
                Some(_) | None => false,
            };
            if let Some(high) = cpu.high()
                && !critical_set
                && cpu.current().celsius() > high.celsius()
            {
                self.cpu.add_css_class("high");
            }
        } else {
            error!("CPU temperature not found");
        }

        if let Some(disk) = disk {
            self.disk
                .set_text(&format!("󰋊  {:.0}°C", disk.current().celsius()));

            self.disk.remove_css_class("critical");
            self.disk.remove_css_class("high");
            let critical_set = match disk.critical() {
                Some(critical) if disk.current().celsius() > critical.celsius() => {
                    self.disk.add_css_class("critical");
                    true
                }
                Some(_) | None => false,
            };
            if let Some(high) = disk.high()
                && !critical_set
                && disk.current().celsius() > high.celsius()
            {
                self.disk.add_css_class("high");
            }
        } else {
            error!("Disk temperature not found");
        }

        Ok(())
    }
}

// The maximum exact value a f64 can hold is 8192 TB, I doubt I'll ever have this much RAM
#[allow(clippy::cast_precision_loss)]
fn format_bytes(quantity: u64) -> String {
    const UNIT_MULTIPLIER: f64 = 1024.;
    const UNIT_PREFIXES: &[&str] = &["", "k", "M", "G", "T"];

    let mut quantity = quantity as f64;
    let mut i = 0;
    while quantity > 1000. {
        quantity /= UNIT_MULTIPLIER;
        i += 1;
    }

    format!("{}{}B", quantity.format_fixed(3), UNIT_PREFIXES[i])
}
