use gtk::{
    gio::{self, BusType, DBusCallFlags, DBusConnection, DBusProxy, DBusProxyFlags},
    glib::{self, clone, Error, SignalHandlerId, Variant, VariantTy, VariantType},
    pango::EllipsizeMode,
    prelude::*,
    Button, EventControllerMotion, Label, Orientation, Revealer, RevealerTransitionType,
};
use gtk4 as gtk;
use std::{collections::HashMap, future::Future, rc::Rc, sync::Mutex};

pub fn new() -> gtk::Box {
    let label = Label::new(None);
    label.set_ellipsize(EllipsizeMode::End);

    let button = Button::new();
    button.set_child(Some(&label));

    let previous_button = Button::with_label("󰒫");
    previous_button.add_css_class("left");

    let next_button = Button::with_label("󰒬");
    next_button.add_css_class("right");

    let left_revealer = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideLeft)
        .transition_duration(500)
        .child(&previous_button)
        .build();
    let right_revealer = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideRight)
        .transition_duration(500)
        .child(&next_button)
        .build();

    let container = gtk::Box::new(Orientation::Horizontal, 0);
    container.set_widget_name("mpris");
    container.add_css_class("module");
    container.set_visible(false);
    container.append(&left_revealer);
    container.append(&button);
    container.append(&right_revealer);

    let event_controller = EventControllerMotion::new();
    event_controller.connect_enter(
        clone!(@strong left_revealer, @strong right_revealer => move |_, _, _| {
            left_revealer.set_reveal_child(true);
            right_revealer.set_reveal_child(true);
        }),
    );
    event_controller.connect_leave(
        clone!(@strong left_revealer, @strong right_revealer => move |_| {
            left_revealer.set_reveal_child(false);
            right_revealer.set_reveal_child(false);
        }),
    );
    container.add_controller(event_controller);

    let current_player: Rc<Mutex<Option<MprisPlayer>>> = Rc::new(Mutex::new(None));

    button.connect_clicked(clone!(@strong current_player => move |_| {
        glib::spawn_future_local(clone!(@strong current_player => async move {
            let player = current_player.lock().unwrap().as_ref().cloned();
            if let Some(player) = player {
                player.toggle().await;
            }
        }));
    }));
    previous_button.connect_clicked(clone!(@strong current_player => move |_| {
        glib::spawn_future_local(clone!(@strong current_player => async move {
            let player = current_player.lock().unwrap().as_ref().cloned();
            if let Some(player) = player {
                player.previous().await;
            }
        }));
    }));
    next_button.connect_clicked(clone!(@strong current_player => move |_| {
        glib::spawn_future_local(clone!(@strong current_player => async move {
            let player = current_player.lock().unwrap().as_ref().cloned();
            if let Some(player) = player {
                player.next().await;
            }
        }));
    }));

    glib::spawn_future_local(
        clone!(@strong container, @strong current_player => async move {
            let dbus = gio::bus_get_future(BusType::Session)
                .await
                .expect("connect connect to session bus");
            let message_bus = MessageBus::new(&dbus).await.unwrap();

            let mut players = HashMap::new();

            for name in message_bus.list_names().await.unwrap() {
                if name.starts_with(MprisPlayer::INTERFACE_BASE) && !name.contains("playerctld") {
                    let player = make_player(&dbus, &name, &container, &label, &current_player).await;
                    players.insert(name, player);
                }
            }

            find_suitable_player(&players, &current_player, &container, &label).await;
            let players = Rc::new(Mutex::new(players));

            message_bus.connect_name_owner_changed(move |name, previous_owner, new_owner| {
                clone!(@strong dbus, @strong players, @strong container, @strong label, @strong current_player => async move {
                    if name.starts_with(MprisPlayer::INTERFACE_BASE) && !name.contains("playerctld") {
                        if previous_owner.is_empty() {
                            let player = make_player(&dbus, &name, &container, &label, &current_player).await;
                            players.lock().unwrap().insert(name, player);
                        } else if new_owner.is_empty() {
                            let Some((player, handler_id)) = players.lock().unwrap().remove(&name) else {
                                return;
                            };
                            player.proxy.disconnect(handler_id);
                            find_suitable_player(&players.lock().unwrap(), &current_player, &container, &label).await;

                        }
                    }
                })
            });
            // leak the message bus, so it's not dropped and the signals can be received
            std::mem::forget(message_bus);
        }),
    );

    container
}

async fn make_player(
    dbus: &DBusConnection,
    name: &str,
    container: &gtk::Box,
    label: &Label,
    current_player: &Rc<Mutex<Option<MprisPlayer>>>,
) -> (MprisPlayer, SignalHandlerId) {
    let player = MprisPlayer::new(dbus, name).await.unwrap();
    unsafe { player.proxy.set_data("is_displayed", false) };

    let handler_id = player
        .proxy
        .connect_properties_changed(clone!(@strong label, @strong container, @strong current_player, @strong player => move |proxy, properties| {
            clone!(@strong label, @strong container, @strong current_player, @strong player => async move {
                let mut is_displayed: bool = unsafe { *proxy.data("is_displayed").unwrap().as_ref() };

                let status: String = properties.get("PlaybackStatus").map_or_else(|| "?".to_string(), |x| x.get().unwrap());
                if status == "Playing" {
                    if let Some(ref current) = *current_player.lock().unwrap() {
                        unsafe { current.proxy.set_data("is_displayed", false) };
                    }
                    unsafe { proxy.set_data("is_displayed", true) };
                    container.set_visible(true);
                    is_displayed = true;
                    current_player.lock().unwrap().replace(player.clone());
                }

                if is_displayed {
                    update_state(&proxy, &label).await;
                }
            })
        }));

    (player, handler_id)
}

// this function is only called when there is no player displayed, so we don"t need to change `is_displayed` on the current player
async fn find_suitable_player(
    players: &HashMap<String, (MprisPlayer, SignalHandlerId)>,
    current_player: &Rc<Mutex<Option<MprisPlayer>>>,
    container: &gtk::Box,
    label: &Label,
) {
    for (player, _) in players.values() {
        let properties = player.proxy.get_all(MprisPlayer::INTERFACE).await.unwrap();
        let status = properties["PlaybackStatus"].get::<String>().unwrap();
        if status == "Playing" {
            unsafe { player.proxy.set_data("is_displayed", true) };
            container.set_visible(true);
            current_player.lock().unwrap().replace(player.clone());
            update_state(&player.proxy, label).await;
            return;
        }
    }

    if let Some((player, _)) = players.values().next() {
        unsafe { player.proxy.set_data("is_displayed", true) };
        container.set_visible(true);
        current_player.lock().unwrap().replace(player.clone());
        update_state(&player.proxy, label).await;
    } else {
        container.set_visible(false);
        current_player.lock().unwrap().take();
    }
}

async fn update_state(proxy: &DBusProxy, label: &Label) {
    let properties = proxy.get_all(MprisPlayer::INTERFACE).await.unwrap();
    let metadata: HashMap<String, Variant> = properties["Metadata"].get().unwrap();

    let status = properties["PlaybackStatus"].get::<String>().unwrap();
    let mut text = if status == "Playing" {
        "󰏤  ".to_owned()
    } else {
        "󰐊  ".to_owned()
    };
    text += &metadata
        .get("xesam:title")
        .map_or_else(|| "Unknown".to_string(), |x| x.get::<String>().unwrap());
    let artist = metadata
        .get("xesam:artist")
        .map_or_else(Vec::new, |x| x.get::<Vec<String>>().unwrap());
    if !(artist.is_empty() || artist.iter().all(String::is_empty)) {
        text += " - ";
        text += &artist.join(", ");
    }
    label.set_text(&text);
}

#[derive(Debug)]
struct MessageBus {
    proxy: DBusProxy,
}

#[derive(Debug, Clone)]
struct MprisPlayer {
    proxy: DBusProxy,
}

impl MessageBus {
    async fn new(dbus: &DBusConnection) -> Result<Self, glib::Error> {
        let proxy = DBusProxy::new_future(
            dbus,
            DBusProxyFlags::NONE,
            None,
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
        )
        .await?;

        Ok(Self { proxy })
    }

    async fn list_names(&self) -> Result<Vec<String>, glib::Error> {
        let names = self
            .proxy
            .call_future("ListNames", None, DBusCallFlags::empty(), -1)
            .await?;

        let (names,): (Vec<String>,) = names
            .get()
            .expect("invalid data received from dbus session server");

        Ok(names)
    }

    fn connect_name_owner_changed<F, T>(&self, callback: F)
    where
        F: Fn(String, String, String) -> T + 'static,
        T: Future<Output = ()> + 'static,
    {
        self.proxy
            .connect_local("g-signal::NameOwnerChanged", true, move |args| {
                let args: Variant = args[3].get().expect("invalid data from dbus");
                let (name, old_owner, new_owner): (String, String, String) =
                    args.get().expect("invalid data from dbus");

                glib::spawn_future_local(callback(name, old_owner, new_owner));
                None
            });
    }
}

impl MprisPlayer {
    const INTERFACE_BASE: &'static str = "org.mpris.MediaPlayer2";
    const INTERFACE: &'static str = "org.mpris.MediaPlayer2.Player";

    async fn new(dbus: &DBusConnection, name: &str) -> Result<Self, glib::Error> {
        let proxy = DBusProxy::new_future(
            dbus,
            DBusProxyFlags::NONE,
            None,
            Some(name),
            "/org/mpris/MediaPlayer2",
            Self::INTERFACE,
        )
        .await?;

        Ok(Self { proxy })
    }

    async fn toggle(&self) {
        let properties = self.proxy.get_all(Self::INTERFACE).await.unwrap();
        let can_control = properties["CanControl"].get::<bool>().unwrap();
        let can_play = properties["CanPlay"].get::<bool>().unwrap();
        let can_pause = properties["CanPause"].get::<bool>().unwrap();

        if !(can_control && can_play && can_pause) {
            println!("Warning: mpris player does not support going to toggling play state");
            return;
        }
        self.proxy
            .call_future("PlayPause", None, DBusCallFlags::empty(), -1)
            .await
            .unwrap();
    }

    async fn previous(&self) {
        let properties = self.proxy.get_all(Self::INTERFACE).await.unwrap();
        let can_control = properties.get("CanControl").unwrap().get::<bool>().unwrap();
        let can_go_previous = properties["CanGoPrevious"].get::<bool>().unwrap();

        if !(can_control && can_go_previous) {
            println!("Warning: mpris player does not support going to previous track");
            return;
        }
        self.proxy
            .call_future("Previous", None, DBusCallFlags::empty(), -1)
            .await
            .unwrap();
    }

    async fn next(&self) {
        let properties = self.proxy.get_all(Self::INTERFACE).await.unwrap();
        let can_control = properties["CanControl"].get::<bool>().unwrap();
        let can_go_next = properties["CanGoNext"].get::<bool>().unwrap();

        if !(can_control && can_go_next) {
            println!("Warning: mpris player does not support going to next track");
            return;
        }
        self.proxy
            .call_future("Next", None, DBusCallFlags::empty(), -1)
            .await
            .unwrap();
    }
}

trait DbusProperties {
    const INTERFACE: &'static str = "org.freedesktop.DBus.Properties";

    async fn get_all(&self, interface: &str) -> Result<HashMap<String, Variant>, Error>;
    fn connect_properties_changed<F, T>(&self, callback: F) -> SignalHandlerId
    where
        F: Fn(DBusProxy, HashMap<String, Variant>) -> T + 'static,
        T: Future<Output = ()> + 'static;
}

impl<P: DBusProxyExt> DbusProperties for P {
    async fn get_all(&self, interface: &str) -> Result<HashMap<String, Variant>, Error> {
        Ok(self
            .connection()
            .call_future(
                self.name().as_deref(),
                &self.object_path(),
                Self::INTERFACE,
                "GetAll",
                Some(&(interface,).into()),
                Some(&VariantType::new_tuple([&VariantType::new_array(
                    &VariantType::new_dict_entry(VariantTy::STRING, VariantTy::VARIANT),
                )])),
                DBusCallFlags::empty(),
                -1,
            )
            .await?
            .get::<(HashMap<String, Variant>,)>()
            .unwrap()
            .0)
    }

    fn connect_properties_changed<F, T>(&self, callback: F) -> SignalHandlerId
    where
        F: Fn(DBusProxy, HashMap<String, Variant>) -> T + 'static,
        T: Future<Output = ()> + 'static,
    {
        self.connect_local("g-properties-changed", true, move |args| {
            let proxy: DBusProxy = args[0].get().unwrap();
            let args: Variant = args[1].get().expect("invalid data from dbus");
            let changed: HashMap<String, Variant> = args.get().expect("invalid data from dbus");

            // it might be empty if invalidated_properties (args[2]) is not
            if !changed.is_empty() {
                glib::spawn_future_local(callback(proxy, changed));
            }

            None
        })
    }
}
