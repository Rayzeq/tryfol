use crate::widget_ext::HasTooltip;
use gtk::{
    glib::{self, clone, prelude::ObjectExt, Propagation, Value, Variant, VariantDict},
    prelude::*,
    Button, EventControllerScroll, EventControllerScrollFlags, Label, Orientation,
};
use gtk4 as gtk;
use std::borrow::Cow;
use wireplumber::{
    core::ObjectFeatures,
    plugin::{Plugin, PluginFeatures},
    prelude::*,
    pw::{Device, GlobalProxy, Node},
    registry::{ConstraintType, ConstraintVerb, Interest, ObjectInterest, ObjectManager},
    spa::{libspa::utils::Id, SpaPod},
};

pub fn new() -> gtk::Box {
    let input_label = Label::new(None);
    let output_label = Label::new(None);

    let input = Button::new();
    input.set_icon_name("input");
    input.add_css_class("left");
    input.set_child(Some(&input_label));

    let output = Button::new();
    output.set_icon_name("output");
    output.set_child(Some(&output_label));

    let container = gtk::Box::new(Orientation::Horizontal, 0);
    container.set_widget_name("audio");
    container.add_css_class("module");
    container.append(&input);
    container.append(&output);

    glib::spawn_future_local(async move {
        wireplumber::Core::init();
        let core = wireplumber::Core::new(None, None, None);

        let manager = ObjectManager::new();
        manager.add_interest(ObjectInterest::new(Node::static_type()));
        manager.add_interest(ObjectInterest::new(Device::static_type()));
        manager.request_object_features(GlobalProxy::static_type(), ObjectFeatures::with_bits(17));
        manager.request_object_features(Node::static_type(), ObjectFeatures::with_bits(17));
        manager.request_object_features(Device::static_type(), ObjectFeatures::with_bits(17));

        core.connect_future().await.unwrap();

        core.load_component_future(
            Some(Cow::Borrowed("libwireplumber-module-default-nodes-api")),
            "module",
            None,
            None,
        )
        .await
        .unwrap();
        core.load_component_future(
            Some(Cow::Borrowed("libwireplumber-module-mixer-api")),
            "module",
            None,
            None,
        )
        .await
        .unwrap();

        let def_nodes_api = Plugin::find(&core, "default-nodes-api").unwrap();
        let mixer_api = Plugin::find(&core, "mixer-api").unwrap();
        #[allow(non_snake_case)] // it is technically a type, which is why it's in CamelCase
        let WpMixerApiVolumeScale = mixer_api.property_type("scale").unwrap();
        mixer_api.set_property_from_value(
            "scale",
            &Value::from(1)
                .transform_with_type(WpMixerApiVolumeScale)
                .unwrap(),
        );

        // this MUST be before def_nodes_api.activate_future(...), otherwise it won't work
        def_nodes_api.connect_local(
            "changed",
            true,
            clone!(@strong def_nodes_api, @strong manager, @strong mixer_api, @strong input_label, @strong output_label => move |_| {
                glib::spawn_future_local(clone!(@strong def_nodes_api, @strong manager, @strong mixer_api, @strong input_label, @strong output_label => async move {
                    refresh(
                        &def_nodes_api,
                        &mixer_api,
                        &manager,
                        &input_label,
                        &output_label,
                    ).await;
                }));
                None
            }),
        );
        mixer_api.connect_local(
            "changed",
            true,
            clone!(@strong def_nodes_api, @strong manager, @strong mixer_api, @strong input_label, @strong output_label => move |_| {
                glib::spawn_future_local(clone!(@strong def_nodes_api, @strong manager, @strong mixer_api, @strong input_label, @strong output_label => async move {
                    refresh(
                        &def_nodes_api,
                        &mixer_api,
                        &manager,
                        &input_label,
                        &output_label,
                    ).await;
                }));
                None
            }),
        );

        def_nodes_api
            .activate_future(PluginFeatures::ENABLED)
            .await
            .unwrap();
        mixer_api
            .activate_future(PluginFeatures::ENABLED)
            .await
            .unwrap();

        core.install_object_manager(&manager);
        manager.installed_future().await.unwrap();

        refresh(
            &def_nodes_api,
            &mixer_api,
            &manager,
            &input_label,
            &output_label,
        )
        .await;

        input.connect_clicked(
            clone!(@strong manager, @strong def_nodes_api, @strong mixer_api => move |_| {
                if let Some(input) = get_default_input(&def_nodes_api, &manager) {
                    toggle_mute(&mixer_api, &input);
                }
            }),
        );
        let scroll_detector = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
        scroll_detector.connect_scroll(
            clone!(@strong manager, @strong def_nodes_api, @strong mixer_api => move |_, _, dy| {
                if let Some(input) = get_default_input(&def_nodes_api, &manager) {
                    change_volume(&mixer_api, &input, -dy);
                }
                Propagation::Proceed
            }),
        );
        input.add_controller(scroll_detector);

        output.connect_clicked(
            clone!(@strong manager, @strong def_nodes_api, @strong mixer_api => move |_| {
                if let Some(output) = get_default_output(&def_nodes_api, &manager) {
                    toggle_mute(&mixer_api, &output);
                }
            }),
        );
        let scroll_detector = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
        scroll_detector.connect_scroll(
            clone!(@strong manager, @strong def_nodes_api, @strong mixer_api => move |_, _, dy| {
                if let Some(output) = get_default_output(&def_nodes_api, &manager) {
                    change_volume(&mixer_api, &output, -dy);
                }
                Propagation::Proceed
            }),
        );
        output.add_controller(scroll_detector);

        // leak core to allow signals to still work
        std::mem::forget(core);
    });

    container
}

async fn refresh(
    def_nodes_api: &Plugin,
    mixer_api: &Plugin,
    manager: &ObjectManager,
    input_label: &Label,
    output_label: &Label,
) {
    if let Some(input) = get_default_input(def_nodes_api, manager) {
        let (volume, muted) = get_volume(mixer_api, &input);
        if muted {
            input_label.set_text("");
            input_label.add_css_class("muted");
        } else {
            let device =
                get_object_with_id::<Device>(manager, input.device_id().unwrap().unwrap()).unwrap();

            let mut icons = device
                .get_pw_property("device.form-factor")
                .map_or_else(|| "", |x| form_factor_to_icon(&x))
                .to_owned();
            icons.insert_str(
                0,
                bus_to_icon(device.get_pw_property("device.bus").as_deref()),
            );
            icons.push(' ');

            input_label.set_text(&format!("{icons} {volume:.0}%"));
            input_label.remove_css_class("muted");
        }
        input_label.set_better_tooltip(
            input
                .get_pw_property("node.description")
                .as_deref()
                .map(ToOwned::to_owned),
        );
    } else {
        input_label.set_text("?");
        input_label.add_css_class("muted");
        input_label.set_better_tooltip(None);
    }

    if let Some(output) = get_default_output(def_nodes_api, manager) {
        let (volume, muted) = get_volume(mixer_api, &output);
        if muted {
            output_label.set_text("󰖁");
            output_label.add_css_class("muted");
        } else {
            let device =
                get_object_with_id::<Device>(manager, output.device_id().unwrap().unwrap())
                    .unwrap();

            let form_factor = if let Some(ff) = device.get_pw_property("device.form-factor") {
                Some(ff)
            } else {
                async {
                    Some(
                        device
                            // get the current routes (input & output)
                            .params_future(Some("Route"), None)
                            .await
                            .ok()?
                            // 2 is Route:direction, 1 is Direction:Output
                            .find(|pod| pod.spa_property::<Id, _>(&2).unwrap().0 == 1)?
                            // 8 is Route:info
                            .spa_property::<SpaPod, _>(&8)?
                            .iterator()
                            .into_value_iterator()
                            // index 1 is "port.type", index 2 is its value
                            .nth(2)?
                            .string()?
                            .to_string(),
                    )
                }
                .await
            };
            let mut icons = form_factor
                .map_or_else(|| "󰕾", |x| form_factor_to_icon(&x))
                .to_owned();
            icons.insert_str(
                0,
                bus_to_icon(device.get_pw_property("device.bus").as_deref()),
            );

            output_label.set_text(&format!("{volume:.0}% {icons}"));
            output_label.remove_css_class("muted");
        }
        output_label.set_better_tooltip(
            output
                .get_pw_property("node.description")
                .as_deref()
                .map(ToOwned::to_owned),
        );
    } else {
        output_label.set_text("?");
        output_label.add_css_class("muted");
        output_label.set_better_tooltip(None);
    }
}

fn form_factor_to_icon(form_factor: &str) -> &'static str {
    match form_factor {
        "internal" | "handset" | "tv" | "webcam" | "microphone" | "car" | "hifi" | "computer"
        | "portable" | "hands-free" => "UFF",
        "speaker" => "󰕾",
        "headphone" |
        // this is not a pipewire form-factor, but a alsa port type
        "headphones" => "󰋋",
        "headset" => "󰋎",
        _ => unreachable!(),
    }
}

fn bus_to_icon(form_factor: Option<&str>) -> &'static str {
    match form_factor {
        None | Some("pci") => "",
        Some("isa" | "firewire" | "usb") => "UB",
        Some("bluetooth") => "󰂯 ",
        Some(_) => unreachable!(),
    }
}

fn get_object_with_id<T: IsA<GlobalProxy>>(manager: &ObjectManager, id: u32) -> Option<T> {
    let interest: Interest<GlobalProxy> = Interest::new();
    interest.add_constraint(
        ConstraintType::PwGlobalProperty,
        "object.id",
        ConstraintVerb::Equals,
        Some(&id.into()),
    );
    let proxy = manager.lookup(interest)?;
    Some(proxy.downcast().unwrap())
}

fn get_default_input(def_nodes_api: &Plugin, manager: &ObjectManager) -> Option<Node> {
    get_object_with_id(
        manager,
        def_nodes_api.emit_by_name("get-default-node", &[&"Audio/Source"]),
    )
}

fn get_default_output(def_nodes_api: &Plugin, manager: &ObjectManager) -> Option<Node> {
    get_object_with_id(
        manager,
        def_nodes_api.emit_by_name("get-default-node", &[&"Audio/Sink"]),
    )
}

fn get_volume(mixer_api: &Plugin, node: &Node) -> (f64, bool) {
    let id = node.bound_id();

    let result: Variant = mixer_api.emit_by_name("get-volume", &[&id]);
    let result: VariantDict = result.get().unwrap();
    let volume: f64 = result.lookup("volume").unwrap().unwrap();
    let muted = result.lookup("mute").unwrap().unwrap();
    (volume * 100., muted)
}

fn toggle_mute(mixer_api: &Plugin, node: &Node) {
    let id = node.bound_id();

    let result: Variant = mixer_api.emit_by_name("get-volume", &[&id]);
    let result: VariantDict = result.get().unwrap();
    let is_muted: bool = result.lookup("mute").unwrap().unwrap();

    let value = VariantDict::new(None);
    value.insert("mute", !is_muted);
    let value: Variant = value.into();
    let _: bool = mixer_api.emit_by_name("set-volume", &[&id, &value]);
}

fn change_volume(mixer_api: &Plugin, node: &Node, delta: f64) {
    let id = node.bound_id();

    let result: Variant = mixer_api.emit_by_name("get-volume", &[&id]);
    let result: VariantDict = result.get().unwrap();
    let mut volume: f64 = result.lookup("volume").unwrap().unwrap();

    volume += delta / 100.;
    if volume < 0. {
        volume = 0.;
    }

    let value = VariantDict::new(None);
    value.insert("volume", volume);
    let value: Variant = value.into();
    let _: bool = mixer_api.emit_by_name("set-volume", &[&id, &value]);
}
