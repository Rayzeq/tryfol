use futures::StreamExt;
use gtk4::{
    self as gtk, Button, CheckButton, Image, Label, Orientation, Popover, PositionType, Separator,
    Widget,
    gdk::Texture,
    glib::{self, Bytes, clone},
    prelude::*,
};
use log::{error, warn};
use std::{
    collections::HashMap,
    future::Future,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;
use zbus::{
    Connection,
    zvariant::{self, OwnedValue},
};

mod proxy;
use proxy::DbusMenuProxy;

use crate::Hoverable;

#[derive(Debug, Clone)]
pub struct DBusMenu {
    proxy: DbusMenuProxy<'static>,
    menuitems: Rc<Mutex<HashMap<i32, MenuItem>>>,
    popover: Rc<Mutex<Option<Popover>>>,
    parent: Rc<Mutex<Option<Widget>>>,
}

#[derive(Debug, Clone)]
enum MenuItem {
    Standard(StandardMenuItem),
    Separator(Separator),
}

#[derive(Debug, Clone)]
struct StandardMenuItem {
    root: Button,
    indicator: CheckButton,
    icon: Image,
    label: Label,
    submenu: Rc<Mutex<Option<Menu>>>,
}

#[derive(Debug, Clone, Copy)]
enum CheckType {
    None,
    Check,
    Radio,
}

#[derive(Debug, Clone, Copy)]
enum CheckState {
    Unchecked,
    Checked,
    Indeterminate,
}

#[derive(Debug, Clone)]
struct Menu {
    parent: Option<Box<Menu>>,
    popover: Popover,
    r#box: gtk::Box,
    children: HashMap<i32, MenuItem>,
    open_submenu: Rc<Mutex<Option<Popover>>>,
}

impl DBusMenu {
    pub async fn new(
        connection: &Connection,
        destination: String,
        path: String,
    ) -> zbus::Result<Self> {
        let proxy = DbusMenuProxy::new(connection, destination, path).await?;
        let (rev_number, layout) = proxy.get_layout(0, -1, &[]).await.unwrap();

        let this = Self {
            proxy: proxy.clone(),
            menuitems: Rc::default(),
            popover: Rc::default(),
            parent: Rc::default(),
        };
        this.update(rev_number, layout).await;

        // client.connect_layout_updated(clone!(
        //     #[strong]
        //     this,
        //     move |client| {
        //         if let Some(root) = client.root() {
        //             this.rebuild(&root);
        //         }
        //     }
        // ));
        glib::spawn_future_local(clone!(
            #[strong]
            this,
            async move {
                let mut events = proxy.receive_layout_updated().await.unwrap();

                while let Some(_event) = events.next().await {
                    let (rev_number, layout) = proxy.get_layout(0, -1, &[]).await.unwrap();
                    this.update(rev_number, layout).await;
                }
            }
        ));

        Ok(this)
    }

    pub async fn set_parent(&mut self, parent: Option<&impl IsA<Widget>>) {
        *self.parent.lock().await = parent.map(|w| w.clone().upcast());
        if let Some(popover) = self.popover.lock().await.as_ref() {
            // gtk will segfault if a popover is re-parented without a call to unparent,
            // this is likely a bug
            popover.unparent();
            if let Some(parent) = parent {
                popover.set_parent(parent);
            }
        }
    }

    pub async fn popup(&self) {
        if let Some(popover) = self.popover.lock().await.as_ref() {
            popover.popup();
        }
    }

    async fn update(
        &self,
        revision_number: u32,
        mut layout: (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>),
    ) {
        self.menuitems.lock().await.clear();
        self.build_menuitem(None, layout.0, &mut layout.1, layout.2)
            .await;
        if let MenuItem::Standard(a) = &self.menuitems.lock().await[&0] {
            let new_popover = a.submenu().lock().await.as_ref().unwrap().root().clone();
            new_popover.set_position(PositionType::Bottom);

            if let Some(parent) = self.parent.lock().await.as_ref() {
                new_popover.unparent();
                new_popover.set_parent(parent);
            }

            self.popover.lock().await.replace(new_popover);
        }
    }

    async fn build_menuitem(
        &self,
        parent: Option<Menu>,
        id: i32,
        properties: &mut HashMap<String, OwnedValue>,
        children: Vec<OwnedValue>,
    ) -> MenuItem {
        properties
            .keys()
            .filter(|name| {
                ![
                    "children-display",
                    "label",
                    "type",
                    "icon-name",
                    "icon-data",
                    "enabled",
                    "visible",
                    "toggle-type",
                    "toggle-state",
                    // I just ignore this, I won't use it
                    "accessible-desc",
                ]
                .contains(&name.as_str())
            })
            .for_each(|name| {
                warn!(
                    "Unknown property `{name}` with a value of {:?}",
                    properties[name]
                );
            });

        let item = match properties.get("type").map(TryInto::try_into) {
            Some(Ok("separator")) => MenuItem::Separator(Separator::new(Orientation::Horizontal)),
            Some(Ok("standard")) | None => MenuItem::Standard(
                self.build_standard_menuitem(parent, id, properties, children)
                    .await,
            ),
            Some(Ok(value)) => {
                error!("Unknown `type` value: {value}");
                MenuItem::Standard(
                    self.build_standard_menuitem(parent, id, properties, children)
                        .await,
                )
            }
            Some(Err(e)) => {
                error!("Wrong type for `type`: {e:?}");
                MenuItem::Standard(
                    self.build_standard_menuitem(parent, id, properties, children)
                        .await,
                )
            }
        };

        match properties.get("visible").map(TryInto::try_into) {
            Some(Ok(visible)) => item.set_visible(visible),
            None => item.set_visible(true),
            Some(Err(e)) => error!("Wrong type for `visible`: {e:?}"),
        }

        self.menuitems.lock().await.insert(id, item.clone());

        item
    }

    async fn build_standard_menuitem(
        &self,
        parent: Option<Menu>,
        id: i32,
        properties: &mut HashMap<String, OwnedValue>,
        children: Vec<OwnedValue>,
    ) -> StandardMenuItem {
        let item = StandardMenuItem::new(parent.clone());

        match properties.get("enabled").map(TryInto::try_into) {
            Some(Ok(enabled)) => item.set_enabled(enabled),
            None => item.set_enabled(true),
            Some(Err(e)) => error!("Wrong type for `enabled`: {e:?}"),
        }

        match properties.get("children-display").map(TryInto::try_into) {
            Some(Ok("submenu")) | None if !children.is_empty() => {
                // Box::pin is used to allow async recursion
                item.set_submenu(Box::pin(self.build_menu(parent, children)).await)
                    .await;
            }
            Some(Ok(value)) => error!("Unknown `children-display` value: {value}"),
            Some(Err(e)) => error!("Wrong type for `children-display`: {e:?}"),
            None => (),
        }

        match properties.get("label").map(TryInto::try_into) {
            Some(Ok(text)) => item.set_label(text),
            None => item.set_label(""),
            Some(Err(e)) => error!("Wrong type for `label`: {e:?}"),
        }

        // using `remove` to gain ownership of the OwnedValue
        match properties
            .remove("icon-data")
            .map(|x| -> Result<Vec<u8>, _> { x.try_into() })
            .transpose()
        {
            Ok(data) => item.set_icon_data(data.as_deref()),
            Err(e) => error!("Wrong type for `icon-data`: {e:?}"),
        }

        match properties
            .get("icon-name")
            .map(TryInto::try_into)
            .transpose()
        {
            Ok(name) => item.set_icon_name(name),
            Err(e) => error!("Wrong type for `icon-name`: {e:?}"),
        }

        match properties.get("toggle-type").map(TryInto::try_into) {
            Some(Ok("checkmark")) => item.set_check_type(CheckType::Check),
            Some(Ok("radio")) => item.set_check_type(CheckType::Radio),
            Some(Ok("")) | None => item.set_check_type(CheckType::None),
            Some(Ok(value)) => error!("Unknown `toggle-type` value: {value}"),
            Some(Err(e)) => error!("Wrong type for `toggle-type`: {e:?}"),
        }

        match properties.get("toggle-state").map(TryInto::try_into) {
            Some(Ok(0)) => item.set_check_state(CheckState::Unchecked),
            Some(Ok(1)) => item.set_check_state(CheckState::Checked),
            Some(Ok(_)) | None => item.set_check_state(CheckState::Indeterminate),
            Some(Err(e)) => error!("Wrong type for `toggle-state`: {e:?}"),
        }

        item.connect_clicked(clone!(
            #[strong(rename_to=proxy)]
            self.proxy,
            #[strong(rename_to=popover)]
            self.popover,
            move || clone!(
                #[strong]
                proxy,
                #[strong]
                popover,
                async move {
                    if let Err(e) = proxy
                        .event(
                            id,
                            "clicked",
                            zvariant::Str::from("").into(),
                            SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .expect("Time went backwards")
                                .as_secs() as u32,
                        )
                        .await
                    {
                        error!("Cannot click on item: {e:?}");
                    } else if let Some(popover) = popover.lock().await.as_ref() {
                        popover.popdown();
                    }
                }
            )
        ));

        item
    }

    async fn build_menu(&self, parent: Option<Menu>, children: Vec<OwnedValue>) -> Menu {
        let mut menu = Menu::new(parent);

        for child in children {
            let (id, mut properties, children): (
                i32,
                HashMap<String, OwnedValue>,
                Vec<OwnedValue>,
            ) = match child.try_into() {
                Ok(x) => x,
                Err(e) => {
                    error!("Wrong type for menu item: {e:?}");
                    continue;
                }
            };
            let item = self
                .build_menuitem(Some(menu.clone()), id, &mut properties, children)
                .await;
            menu.add_child(id, item);
        }

        menu
    }
}

impl MenuItem {
    pub fn root(&self) -> &Widget {
        match self {
            Self::Standard(x) => x.root().upcast_ref(),
            Self::Separator(x) => x.upcast_ref(),
        }
    }

    pub fn set_visible(&self, visible: bool) {
        match self {
            Self::Standard(x) => x.root().set_visible(visible),
            Self::Separator(x) => x.set_visible(visible),
        }
    }
}

impl StandardMenuItem {
    pub fn new(parent: Option<Menu>) -> Self {
        let indicator = CheckButton::new();
        indicator.set_visible(false);

        let icon = Image::new();
        icon.set_visible(false);

        let label = Label::builder().use_underline(true).build();

        let hbox = gtk::Box::builder().spacing(3).build();
        hbox.append(&indicator);
        hbox.append(&icon);
        hbox.append(&label);

        let root = Button::builder()
            .css_name("modelbutton")
            .child(&hbox)
            .build();

        let submenu: Rc<Mutex<Option<Menu>>> = Rc::default();
        root.connect_hover_notify(clone!(
            #[strong]
            submenu,
            move |_, hovered| {
                glib::spawn_future_local(clone!(
                    #[strong]
                    parent,
                    #[strong]
                    submenu,
                    async move {
                        if let Some(submenu) = submenu.lock().await.as_ref() {
                            if hovered {
                                submenu.popup().await;
                            }
                        } else if let Some(parent) = parent {
                            parent.close_submenu().await;
                        }
                    }
                ));
            }
        ));

        Self {
            root,
            indicator,
            icon,
            label,
            submenu,
        }
    }

    pub const fn root(&self) -> &Button {
        &self.root
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.root.set_sensitive(enabled);
    }

    pub fn set_label(&self, text: &str) {
        self.label.set_label(text);
    }

    pub fn set_icon_name(&self, name: Option<&str>) {
        if name.is_none() && self.icon.paintable().is_some() {
            return;
        }

        self.icon.set_visible(name.is_some());
        self.icon.set_icon_name(name);
    }

    pub fn set_icon_data(&self, data: Option<&[u8]>) {
        if data.is_none() && self.icon.icon_name().is_some() {
            return;
        }

        self.icon.set_visible(data.is_some());
        if let Some(data) = data {
            match Texture::from_bytes(&Bytes::from(&data)) {
                Ok(x) => {
                    self.icon.set_paintable(Some(&x));
                }
                Err(e) => {
                    error!("Cannot load icon: {e}");
                }
            }
        } else {
            self.icon.set_paintable(None::<&Texture>);
        }
    }

    pub fn set_check_type(&self, check_type: CheckType) {
        match check_type {
            CheckType::None => self.indicator.set_visible(false),
            CheckType::Check => {
                self.indicator.set_visible(true);
                self.indicator.set_group(None::<&CheckButton>);
            }
            // using a dummy button to make a group, so the indicator is a radio button
            CheckType::Radio => {
                self.indicator.set_visible(true);
                self.indicator.set_group(Some(&CheckButton::new()));
            }
        }
    }

    pub fn set_check_state(&self, state: CheckState) {
        match state {
            CheckState::Unchecked => self.indicator.set_active(false),
            CheckState::Checked => self.indicator.set_active(true),
            CheckState::Indeterminate => self.indicator.set_inconsistent(true),
        }
    }

    pub const fn submenu(&self) -> &Rc<Mutex<Option<Menu>>> {
        &self.submenu
    }

    pub async fn set_submenu(&self, submenu: Menu) {
        if let Some(submenu) = self.submenu.lock().await.as_ref() {
            submenu.root().unparent();
        }

        submenu.root().unparent();
        submenu.root().set_parent(&self.root);
        submenu.root().set_position(PositionType::Right);
        self.root.queue_resize();
        self.submenu.lock().await.replace(submenu);
    }

    pub fn connect_clicked<F, T>(&self, f: F)
    where
        F: Fn() -> T + 'static,
        T: Future<Output = ()> + 'static,
    {
        self.root.connect_clicked(move |_| {
            glib::spawn_future_local(f());
        });
    }
}

impl Menu {
    pub fn new(parent: Option<Self>) -> Self {
        let r#box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .build();

        let popover = Popover::builder()
            .css_classes(["menu"])
            .has_arrow(false)
            .autohide(true)
            .child(&r#box)
            .build();

        let open_submenu: Rc<Mutex<Option<Popover>>> = Rc::default();

        popover.connect_closed(clone!(
            #[strong]
            open_submenu,
            move |_| {
                glib::spawn_future_local(clone!(
                    #[strong]
                    open_submenu,
                    async move {
                        if let Some(submenu) = open_submenu.lock().await.as_ref() {
                            submenu.popdown();
                        }
                    }
                ));
            }
        ));

        Self {
            parent: parent.map(Box::new),
            popover,
            r#box,
            children: HashMap::new(),
            open_submenu,
        }
    }

    pub const fn root(&self) -> &Popover {
        &self.popover
    }

    pub fn add_child(&mut self, id: i32, child: MenuItem) {
        self.r#box.append(child.root());
        self.children.insert(id, child);
    }

    pub async fn popup(&self) {
        if self.popover.get_visible() {
            return;
        }

        if let Some(parent) = &self.parent {
            parent.set_open(self.popover.clone()).await;
        }

        #[allow(deprecated)] // there is simply no other way to do it
        self.popover.connect_map(|popover| {
            let height = popover.preferred_size().1.height();
            let padding = popover.style_context().padding().top();
            let border = popover.style_context().border().top();
            // add a 1 pixel offset if the height is even, not sure this is the right fix
            let even_offset = (height % 2 == 0) as i32;

            popover.set_offset(
                0,
                height / 2
                    - popover.parent().unwrap().height() / 2
                    - padding as i32
                    - border as i32
                    - even_offset,
            );
        });
        self.popover.popup();
    }

    pub async fn set_open(&self, submenu: Popover) {
        let mut open_submenu = self.open_submenu.lock().await;
        if let Some(old_submenu) = open_submenu.replace(submenu) {
            old_submenu.popdown();
        }
    }

    pub async fn close_submenu(&self) {
        let submenu = self.open_submenu.lock().await.take();
        if let Some(submenu) = submenu {
            submenu.popdown();
        }
    }
}
