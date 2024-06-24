use crate::{
    backend::status_notifier::{self, run_host, Event, Orientation, Status},
    dbusmenu::DBusMenu,
    Clickable, HasTooltip, Scrollable,
};
use anyhow::Context;
use futures::StreamExt;
use gtk4::{
    self as gtk,
    gdk::Paintable,
    glib::JoinHandle,
    glib::{self, clone},
    prelude::*,
    Image, Widget,
};
use log::error;
use std::{cell::RefCell, collections::HashMap, convert::Infallible, rc::Rc};

struct Host {
    root: gtk::Box,
    items: HashMap<String, (Image, JoinHandle<()>)>,
}

#[derive(Debug)]
struct Item {
    root: Image,
    #[allow(clippy::struct_field_names)]
    item: status_notifier::Item,

    // cache
    status: Status,
    icon: Paintable,
    attention_icon: Paintable,
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
        let (item, task) = match Item::new(id.to_owned(), item).await {
            Ok(x) => x,
            Err(e) => {
                error!("Error while handling new item: {e}");
                return;
            }
        };

        if let Some((old_item, old_task)) = self.items.insert(id.to_owned(), (item.clone(), task)) {
            // replace the old child with the new one
            self.root.insert_child_after(&item, Some(&old_item));
            self.root.remove(&old_item);
            old_task.abort();
        } else {
            self.root.append(&item);
        }
    }

    async fn item_unregistered(&mut self, id: &str) {
        if let Some((item, task)) = self.items.remove(id) {
            self.root.remove(&item);
            task.abort();
        }
    }
}

impl Item {
    const SIZE: i32 = 18;

    /// Create an item, setup listeners, and returns the root widget.
    #[allow(clippy::new_ret_no_self)]
    async fn new(
        id: String,
        item: status_notifier::Item,
    ) -> anyhow::Result<(Image, JoinHandle<()>)> {
        let root = Image::builder().pixel_size(Self::SIZE).build();

        let initial_status = item
            .status()
            .await
            .context("While getting the initial status")?;
        let initial_icon = item
            .icon(Self::SIZE, root.scale_factor())
            .await
            .context("While getting the initial icon")?;
        let initial_attention_icon = item
            .attention_icon(Self::SIZE, root.scale_factor())
            .await
            .context("While getting the initial attention icon")?;

        let mut this = Self {
            root,
            item,

            status: initial_status,
            icon: initial_icon,
            attention_icon: initial_attention_icon,
            menu: Rc::default(),
        };
        let root = this.root.clone();

        if let Err(e) = this.setup().await {
            error!("Error setting up systray item {id}: {e}");
        }
        let task = glib::spawn_future_local(async move {
            if let Err(e) = this.listen().await {
                error!("Error while handling events: {e}");
            }
        });

        Ok((root, task))
    }

    async fn setup(&mut self) -> anyhow::Result<()> {
        self.title_changed().await;
        // this will set the right icon
        self.status_changed(self.item.status().await?);
        self.menu_changed().await;

        self.root.connect_left_clicked(
            clone!(@strong self.item as item, @strong self.menu as menu => move |_, _, x, y| {
                glib::spawn_future_local(clone!(@strong item, @strong menu => async move {
                    #[allow(clippy::cast_possible_truncation)]
                    let (x, y) = (x as i32, y as i32);
                    let item_is_menu = match item.item_is_menu().await {
                        Ok(x) => x,
                        Err(e) => {
                            error!("Cannot check whether item is a menu: {e}");
                            false
                        }
                    };

                    let result = if item_is_menu {
                        Self::popup_menu(item, menu, x, y).await
                    } else {
                        match item.activate(x, y).await {
                            Ok(true) => Ok(()),
                            Ok(false) => Self::popup_menu(item, menu, x, y).await,
                            Err(e) => Err(e)
                        }
                    };
                    if let Err(e) = result {
                        error!("Error while handling left click: {e}");
                    }
                }));
            }),
        );

        self.root
            .connect_middle_clicked(clone!(@strong self.item as item => move |_, _, x, y| {
                glib::spawn_future_local(clone!(@strong item => async move {
                    #[allow(clippy::cast_possible_truncation)]
                    if let Err(e) = item.secondary_activate(x as i32, y as i32).await {
                        error!("Error while handling middle click: {e}");
                    }
                }));
            }));

        self.root.connect_right_clicked(
            clone!(@strong self.item as item, @strong self.menu as menu => move |_, _, x, y| {
                glib::spawn_future_local(clone!(@strong item, @strong menu => async move {
                    #[allow(clippy::cast_possible_truncation)]
                    if let Err(e) = Self::popup_menu(item, menu, x as i32, y as i32).await {
                        error!("Error while handling right click: {e}");
                    }
                }));
            }),
        );

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

    async fn listen(mut self) -> zbus::Result<Infallible> {
        let mut events = self.item.events().await?;

        while let Some(event) = events.next().await {
            match event {
                Event::NewTitle => self.title_changed().await,
                Event::NewStatus(new_status) => self.status_changed(new_status),
                Event::NewIcon => self.icon_changed().await,
                Event::NewAttentionIcon => self.attention_icon_changed().await,
                Event::NewOverlayIcon => println!("Overlay icons aren't supported"),
                Event::NewMenu => self.menu_changed().await,
            }
        }

        unreachable!()
    }

    async fn title_changed(&self) {
        match self.item.title().await {
            Ok(new_title) => self.root.set_better_tooltip(Some(new_title)),
            Err(e) => error!("Cannot get item title: {e}"),
        }
    }

    fn status_changed(&mut self, new_status: Status) {
        self.status = new_status;
        match self.status {
            Status::Passive => self.root.set_visible(false),
            Status::Active => {
                self.root.set_paintable(Some(&self.icon));
                self.root.set_visible(true);
            }
            Status::NeedsAttention => {
                self.root.set_paintable(Some(&self.attention_icon));
                self.root.set_visible(true);
            }
        }
    }

    async fn icon_changed(&mut self) {
        self.icon = match self.item.icon(Self::SIZE, self.root.scale_factor()).await {
            Ok(x) => x,
            Err(e) => {
                error!("Cannot get item icon: {e}");
                return;
            }
        };
        if self.status == Status::Active {
            self.root.set_paintable(Some(&self.icon));
        }
    }

    async fn attention_icon_changed(&mut self) {
        self.attention_icon = match self
            .item
            .attention_icon(Self::SIZE, self.root.scale_factor())
            .await
        {
            Ok(x) => x,
            Err(e) => {
                error!("Cannot get item attention icon: {e}");
                return;
            }
        };
        if self.status == Status::NeedsAttention {
            self.root.set_paintable(Some(&self.attention_icon));
        }
    }

    async fn menu_changed(&self) {
        match self.item.menu().await {
            Ok(new_menu) => {
                if let Some(ref new_menu) = new_menu {
                    new_menu.set_parent(Some(&self.root));
                }
                if let Some(old_menu) = self.menu.replace(new_menu) {
                    old_menu.set_parent(None::<&Widget>);
                }
            }
            Err(e) => error!("Cannot get item menu: {e}"),
        }
    }

    async fn popup_menu(
        item: status_notifier::Item,
        menu: Rc<RefCell<Option<DBusMenu>>>,
        x: i32,
        y: i32,
    ) -> zbus::Result<()> {
        if let Some(menu) = &*menu.borrow() {
            menu.popup();
            // early return to avoid the RefCell guard being held across an await point
            return Ok(());
        }
        item.context_menu(x, y).await.map_err(Into::into)
    }
}
