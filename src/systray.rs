use crate::{dbusmenu::DBusMenu, notifier_host, HasTooltip};
use futures::StreamExt;
use gtk::{
    gdk,
    glib::{self, clone},
    prelude::*,
    EventControllerMotion, Image, Orientation,
};
use gtk4 as gtk;
use std::{cell::RefCell, collections::HashMap, future::Future, rc::Rc};

pub fn new() -> gtk::Box {
    let container = gtk::Box::new(Orientation::Horizontal, 2);
    container.set_widget_name("systray");
    container.add_css_class("module");

    let props = Props::new();
    spawn_systray(&container, &props);

    let menu = DBusMenu::new("org.blueman.Tray", "/org/blueman/sni/menu");
    std::mem::forget(menu);

    container
}

// DBus state shared between systray instances, to avoid creating too many connections etc.
struct DBusSession {
    snw: notifier_host::proxy::StatusNotifierWatcherProxy<'static>,
}

async fn dbus_session() -> zbus::Result<&'static DBusSession> {
    // TODO make DBusSession reference counted so it's dropped when not in use?

    static DBUS_STATE: tokio::sync::OnceCell<DBusSession> = tokio::sync::OnceCell::const_new();
    DBUS_STATE
        .get_or_try_init(|| async {
            let con = zbus::Connection::session().await?;
            notifier_host::Watcher::new().attach_to(&con).await?;

            let (_, snw) = notifier_host::register_as_host(&con).await?;

            Ok(DBusSession { snw })
        })
        .await
}

fn run_async_task<F: Future>(f: F) -> F::Output {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to initialize tokio runtime");
    rt.block_on(f)
}

pub struct Props {
    pub prepend_new: Rc<RefCell<bool>>,
}

impl Props {
    pub fn new() -> Self {
        Self {
            prepend_new: Rc::new(RefCell::new(false)),
        }
    }
}

struct Tray {
    container: gtk::Box,
    items: HashMap<String, Item>,

    prepend_new: Rc<RefCell<bool>>,
}

pub fn spawn_systray(container: &gtk::Box, props: &Props) {
    let mut systray = Tray {
        container: container.clone(),
        items: HashMap::default(),
        prepend_new: props.prepend_new.clone(),
    };

    let task = glib::MainContext::default().spawn_local(async move {
        let s = match dbus_session().await {
            Ok(x) => x,
            Err(e) => {
                log::error!("could not initialise dbus connection for tray: {}", e);
                return;
            }
        };

        systray.container.set_visible(true);
        let e = notifier_host::run_host(&mut systray, &s.snw).await;
        log::error!("notifier host error: {}", e);
    });

    // stop the task when the widget is dropped
    container.connect_destroy(move |_| {
        task.abort();
    });
}

impl notifier_host::Host for Tray {
    fn add_item(&mut self, id: &str, item: notifier_host::Item) {
        let item = Item::new(id.to_owned(), item);
        if *self.prepend_new.borrow() {
            self.container.append(&item.widget);
        } else {
            self.container.prepend(&item.widget);
        }
        if let Some(old_item) = self.items.insert(id.to_string(), item) {
            self.container.remove(&old_item.widget);
        }
    }

    fn remove_item(&mut self, id: &str) {
        if let Some(item) = self.items.get(id) {
            self.container.remove(&item.widget);
            self.items.remove(id);
        } else {
            log::warn!("Tried to remove nonexistent item {:?} from systray", id);
        }
    }
}

/// Item represents a single icon being shown in the system tray.
struct Item {
    /// Main widget representing this tray item.
    widget: gtk::Box,

    /// Async task to stop when this item gets removed.
    task: Option<glib::JoinHandle<()>>,
}

impl Drop for Item {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

impl Item {
    const ICON_SIZE: i32 = 18;

    fn new(id: String, item: notifier_host::Item) -> Self {
        let gtk_widget = gtk::Box::new(Orientation::Horizontal, 0);

        // Support :hover selector (is this still necessary ?)
        let event_controller = EventControllerMotion::new();
        event_controller.connect_enter(clone!(@strong gtk_widget => move |_, _, _| {
                gtk_widget
                    .set_state_flags(gtk::StateFlags::PRELIGHT, false);
        }));
        event_controller.connect_leave(clone!(@strong gtk_widget => move |_| {
                gtk_widget
                    .unset_state_flags(gtk::StateFlags::PRELIGHT);
        }));
        gtk_widget.add_controller(event_controller);

        let out_widget = gtk_widget.clone(); // copy so we can return it

        let task = glib::MainContext::default().spawn_local(async move {
            if let Err(e) = Self::maintain(gtk_widget.clone(), item).await {
                log::error!("error for systray item {}: {}", id, e);
            }
        });

        Self {
            widget: out_widget,
            task: Some(task),
        }
    }

    async fn maintain(widget: gtk::Box, mut item: notifier_host::Item) -> zbus::Result<()> {
        // init icon
        let icon = gtk::Image::new();
        widget.append(&icon);

        // init menu
        if let Err(e) = item.set_menu(&widget).await {
            log::warn!("failed to set menu: {}", e);
        }

        // TODO this is a lot of code duplication unfortunately, i'm not really sure how to
        // refactor without making the borrow checker angry

        // set status
        match item.status().await? {
            notifier_host::Status::Passive => widget.set_visible(false),
            notifier_host::Status::Active | notifier_host::Status::NeedsAttention => {
                widget.set_visible(true);
            }
        }

        // set title
        widget.set_better_tooltip(Some(item.sni.title().await?));

        // set icon
        let scale = icon.scale_factor();
        load_icon_for_item(&icon, &item, Self::ICON_SIZE, scale).await;

        let item_real = Rc::new(item);

        let item = item_real.clone();
        let gesture = gtk::GestureClick::new();
        gesture.set_button(gdk::BUTTON_PRIMARY);
        gesture.connect_pressed(move |gesture, _, x, y| {
            gesture.set_state(gtk::EventSequenceState::Claimed);

            let (x, y) = (x as i32, y as i32);
            let item_is_menu = run_async_task(async { item.sni.item_is_menu().await });
            let have_item_is_menu = item_is_menu.is_ok();
            let item_is_menu = item_is_menu.unwrap_or(false);
            log::debug!(
                "mouse click button=primary, x={}, y={}, have_item_is_menu={}, item_is_menu={}",
                x,
                y,
                have_item_is_menu,
                item_is_menu
            );
            let result = if !item_is_menu {
                let result = run_async_task(async { item.sni.activate(x, y).await });
                if result.is_err() && !have_item_is_menu {
                    log::debug!("fallback to context menu due to: {}", result.unwrap_err());
                    // Some applications are in fact menu-only (don't have Activate method)
                    // but don't report so through ItemIsMenu property. Fallback to menu if
                    // activate failed in this case.
                    run_async_task(async { item.popup_menu(x, y).await })
                } else {
                    result
                }
            } else {
                run_async_task(async { item.popup_menu(x, y).await })
            };
            if let Err(result) = result {
                log::error!("failed to handle primary mouse click: {}", result);
            }
        });
        widget.add_controller(gesture);

        let item = item_real.clone();
        let gesture = gtk::GestureClick::new();
        gesture.set_button(gdk::BUTTON_MIDDLE);
        gesture.connect_pressed(move |gesture, _, x, y| {
            gesture.set_state(gtk::EventSequenceState::Claimed);

            let (x, y) = (x as i32, y as i32);
            log::debug!("mouse click button=middle, x={}, y={}", x, y,);

            if let Err(result) = run_async_task(async { item.sni.secondary_activate(x, y).await }) {
                log::error!("failed to handle middle mouse click: {}", result);
            }
        });
        widget.add_controller(gesture);

        let item = item_real.clone();
        let gesture = gtk::GestureClick::new();
        gesture.set_button(gdk::BUTTON_SECONDARY);
        gesture.connect_pressed(clone!(@strong item => move |gesture, _, x, y| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            run_async_task(async { item.popup_menu(x as i32, y as i32).await }).unwrap();
        }));
        widget.add_controller(gesture);

        // updates
        let mut status_updates = item.sni.receive_new_status().await?;
        let mut title_updates = item.sni.receive_new_title().await?;
        let mut icon_updates = item.sni.receive_new_icon().await?;

        loop {
            tokio::select! {
                Some(_) = status_updates.next() => {
                    // set status
                    match item.status().await? {
                        notifier_host::Status::Passive => widget.set_visible(false),
                        notifier_host::Status::Active | notifier_host::Status::NeedsAttention => widget.set_visible(true),
                    }
                }
                Some(_) = title_updates.next() => {
                    // set title
                    widget.set_better_tooltip(Some(item.sni.title().await?));
                }
                Some(_) = icon_updates.next() => {
                    // set icon
                    load_icon_for_item(&icon, &item, Self::ICON_SIZE, scale).await;
                }
            }
        }
    }
}

async fn load_icon_for_item(icon: &Image, item: &notifier_host::Item, size: i32, scale: i32) {
    icon.set_pixel_size(size);
    icon.set_icon_name(None);
    if let Some(pixbuf) = item.icon(icon, size, scale).await {
        icon.set_paintable(Some(&pixbuf));
    }
}
