// gio's futures are not send, so ours can't be either
#![allow(clippy::future_not_send)]

use gtk::{
    Align, Application, ApplicationWindow, CssProvider, Orientation, gdk::Display, glib, prelude::*,
};
use gtk4 as gtk;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::env;
use things::StopFuckingWithMyClockLayout;

mod audio;
mod backend;
mod bluetooth;
mod extensions;
mod modules;
mod network;
mod things;
pub use extensions::*;

use modules::*;

fn left_modules() -> gtk::Box {
    let hyprland::Modules { workspaces, window } = hyprland::Modules::new();

    let modules = gtk::Box::new(Orientation::Horizontal, 0);
    modules.append(&workspaces);
    modules.append(&mpris::new());
    modules.append(&window);
    modules.append(&systray::new());

    modules
}

fn right_modules() -> gtk::Box {
    let psutil::Modules {
        cpu,
        memory,
        temperatures,
    } = psutil::Modules::new();

    let connectivity_module = gtk::Box::new(Orientation::Horizontal, 0);
    connectivity_module.set_widget_name("connectivity");
    connectivity_module.add_css_class("module");
    connectivity_module.append(&bluetooth::new());
    connectivity_module.append(&network::new());

    let modules = gtk::Box::new(Orientation::Horizontal, 0);
    modules.set_halign(Align::End);
    modules.append(&connectivity_module);
    modules.append(&temperatures);
    modules.append(&memory);
    modules.append(&cpu);
    modules.append(&power::new());
    modules.append(&audio::new());
    modules.append(&powerbutton::new());

    modules
}

fn create_window(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .height_request(40)
        .build();

    window.init_layer_shell();
    window.set_layer(Layer::Top);
    window.auto_exclusive_zone_enable();

    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_anchor(Edge::Top, true);

    let main = gtk::Box::builder()
        .name("main")
        .layout_manager(&StopFuckingWithMyClockLayout::new())
        .build();
    main.append(&left_modules());
    main.append(&clock::new());
    main.append(&right_modules());

    window.set_child(Some(&main));
    window.present();
}

fn load_css(_: &Application) {
    let css = grass::from_path(
        "/mnt/Storage/Projects/tryfol/tryfol-bar/src/style.scss",
        &grass::Options::default(),
    )
    .expect("invalid css");

    let provider = CssProvider::new();
    provider.load_from_string(&css);
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("could not connect to a display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn main() -> glib::ExitCode {
    unsafe {
        env::set_var("RUST_BACKTRACE", "1");
    }
    simple_logger::init_with_level(log::Level::Warn).unwrap();

    let app = Application::builder()
        .application_id("me.rayzeq.Tryfol")
        .build();

    app.connect_startup(load_css);
    app.connect_activate(create_window);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .thread_name("main-async-runtime")
        .enable_all()
        .build()
        .expect("Failed to initialize tokio runtime");
    let _g = rt.handle().enter();

    app.run()
}
