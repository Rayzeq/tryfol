use crate::{
    backend::status_notifier::{self, run_host, Orientation, Status},
    dbusmenu::DBusMenu,
    Clickable, HasTooltip, Scrollable,
};
use gtk::{
    gdk::Paintable,
    glib::{self, clone},
    prelude::*,
    Image, Widget,
};
use gtk4 as gtk;
use log::error;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

struct Host {
    root: gtk::Box,
    items: HashMap<String, Item>,
}

#[derive(Debug, Clone)]
struct Item {
    root: Image,
    #[allow(clippy::struct_field_names)]
    item: status_notifier::Item,
    menu: Rc<RefCell<Option<DBusMenu>>>,
}

pub fn new() -> gtk::Box {
    let root = gtk::Box::builder()
        .name("systray")
        .css_classes(["module"])
        .spacing(2)
        .build();

    let mut tray = Host {
        root: root.clone(),
        items: HashMap::new(),
    };

    let task = glib::spawn_future_local(async move {
        if let Err(e) = run_host(&mut tray).await {
            error!("Error while running the status notifier host: {e}");
        }
    });
    // stop the task when the widget is dropped
    root.connect_destroy(move |_| {
        task.abort();
    });

    root
}

impl status_notifier::Host for Host {
    async fn item_registered(&mut self, id: &str, item: status_notifier::Item) {
        let item = Item::new(id.to_owned(), item).await;

        let item_root = item.root.clone();
        if let Some(old_item) = self.items.insert(id.to_owned(), item) {
            // replace the old child with the new one
            self.root
                .insert_child_after(&item_root, Some(&old_item.root));
            self.root.remove(&old_item.root);
        } else {
            self.root.append(&item_root);
        }
    }

    async fn item_unregistered(&mut self, id: &str) {
        if let Some(item) = self.items.remove(id) {
            self.root.remove(&item.root);
        }
    }
}

impl Item {
    const SIZE: i32 = 18;

    async fn new(id: String, item: status_notifier::Item) -> Self {
        let root = Image::builder().pixel_size(Self::SIZE).build();
        let mut this = Self {
            root,
            item,
            menu: Rc::default(),
        };

        if let Err(e) = this.setup().await {
            error!("Error setting up systray item {id}: {e}");
        }

        this
    }

    async fn setup(&mut self) -> anyhow::Result<()> {
        Self::status_changed(&self.root, self.item.status().await?);
        Self::title_changed(&self.root, self.item.title().await?);
        Self::icon_changed(
            &self.root,
            &self.item.icon(Self::SIZE, self.root.scale_factor()).await?,
        );
        Self::menu_changed(self, self.item.menu().await?);
        self.connect_updaters();

        self.root
            .connect_left_clicked(clone!(@strong self as this => move |_, _, x, y| {
                glib::spawn_future_local(clone!(@strong this => async move {
                    #[allow(clippy::cast_possible_truncation)]
                    let (x, y) = (x as i32, y as i32);
                    let item_is_menu = match this.item.item_is_menu().await {
                        Ok(x) => x,
                        Err(e) => {
                            error!("Cannot check whether item is a menu: {e}");
                            false
                        }
                    };

                    let result = if item_is_menu {
                        this.popup_menu(x, y).await
                    } else {
                        match this.item.activate(x, y).await {
                            Ok(true) => Ok(()),
                            Ok(false) => this.popup_menu(x, y).await,
                            Err(e) => Err(e)
                        }
                    };
                    if let Err(e) = result {
                        error!("Error while handling left click: {e}");
                    }
                }));
            }));

        self.root
            .connect_middle_clicked(clone!(@strong self.item as item => move |_, _, x, y| {
                glib::spawn_future_local(clone!(@strong item => async move {
                    #[allow(clippy::cast_possible_truncation)]
                    if let Err(e) = item.secondary_activate(x as i32, y as i32).await {
                        error!("Error while handling middle click: {e}");
                    }
                }));
            }));

        self.root
            .connect_right_clicked(clone!(@strong self as this => move |_, _, x, y| {
                glib::spawn_future_local(clone!(@strong this => async move {
                    #[allow(clippy::cast_possible_truncation)]
                    if let Err(e) = this.popup_menu(x as i32, y as i32).await {
                        error!("Error while handling right click: {e}");
                    }
                }));
            }));

        self.root
            .connect_both_scroll(clone!(@strong self.item as item => move |_, dx, dy| {
                glib::spawn_future_local(clone!(@strong item => async move {
                    if dx != 0. {
                        #[allow(clippy::cast_possible_truncation)]
                        if let Err(e) = item.scroll(dx as i32, Orientation::Horizontal).await {
                            error!("Error while handling scroll event: {e}");
                        }
                    }
                    if dy != 0. {
                        #[allow(clippy::cast_possible_truncation)]
                        if let Err(e) = item.scroll(dy as i32, Orientation::Vertical).await {
                            error!("Error while handling scroll event: {e}");
                        }
                    }
                }));
            }));

        Ok(())
    }

    fn connect_updaters(&mut self) {
        self.item.connect_status_changed(clone!(@weak self.root as root => move |new_status| Self::status_changed(&root, new_status)));
        self.item.connect_title_changed(clone!(@weak self.root as root => move |new_title| Self::title_changed(&root, new_title)));
        self.item.connect_icon_changed(
            Self::SIZE,
            self.root.scale_factor(),
            clone!(@weak self.root as root => move |new_icon| Self::icon_changed(&root, &new_icon)),
        );
        self.item.connect_menu_changed(
            clone!(@strong self as this => move |new_menu| this.menu_changed(new_menu)),
        );
    }

    fn status_changed(root: &Image, new_status: Status) {
        match new_status {
            Status::Passive => root.set_visible(false),
            Status::Active | status_notifier::Status::NeedsAttention => root.set_visible(true),
        }
    }

    fn title_changed(root: &Image, new_title: String) {
        root.set_better_tooltip(Some(new_title));
    }

    fn icon_changed(root: &Image, new_icon: &Paintable) {
        root.set_paintable(Some(new_icon));
    }

    fn menu_changed(&self, new_menu: Option<DBusMenu>) {
        if let Some(ref new_menu) = new_menu {
            new_menu.set_parent(Some(&self.root));
        }
        if let Some(old_menu) = self.menu.replace(new_menu) {
            old_menu.set_parent(None::<&Widget>);
        }
    }

    async fn popup_menu(&self, x: i32, y: i32) -> zbus::Result<()> {
        if let Some(menu) = &*self.menu.borrow() {
            menu.popup();
            // early return to avoid the RefCell guard being held across an await point
            return Ok(());
        }
        self.item.context_menu(x, y).await.map_err(Into::into)
    }
}
