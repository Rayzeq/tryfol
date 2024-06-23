use super::proxy::{Category, ItemProxy, Orientation, Pixmap, Status};
use crate::dbusmenu::DBusMenu;
use anyhow::bail;
use futures::StreamExt;
use gtk4::{
    gdk::{Display, Paintable, Texture},
    gdk_pixbuf::{Colorspace, InterpType, Pixbuf},
    glib::{self, Bytes},
    IconLookupFlags, IconPaintable, IconTheme, TextDirection,
};
use log::error;
use paste::paste;
use std::{
    cmp::Ordering,
    path::{Path, PathBuf},
};
use zbus::{fdo, names::BusName, Connection};

#[derive(Debug)]
pub struct Item {
    proxy: ItemProxy<'static>,
    tasks: Vec<glib::JoinHandle<()>>,
}

macro_rules! connect_function {
    ($signal_name:ident, $arg_type:ty) => {
        paste! {
            /// Callbacks registered with this method will only be called as long as the instance is not dropped.
            /// Cloned instances won't allow the callbacks to work.
            pub fn [<connect_ $signal_name _changed>]<F>(&mut self, callback: F)
            where
                F: Fn($arg_type) + 'static,
            {
                let this = self.clone();
                let task = glib::spawn_future_local(async move {
                    let mut events = match this.proxy.[<receive_new_ $signal_name>]().await {
                        Ok(x) => x,
                        Err(e) => {
                            error!(concat!("Failed to receive new ", stringify!($signal_name), " events: {}"), e);
                            return;
                        }
                    };

                    while events.next().await.is_some() {
                        let new_value = match this.$signal_name().await {
                            Ok(x) => x,
                            Err(e) => {
                                error!(concat!("Failed to get ", stringify!($signal_name), ": {}"), e);
                                continue;
                            }
                        };
                        callback(new_value);
                    }
                });
                self.tasks.push(task);
            }
        }
    };
    ($signal_name:ident, $arg_type:ty, $($additial_args_name:ident: $additial_args_type:ty),*) => {
        paste! {
            pub fn [<connect_ $signal_name _changed>]<F>(&self, $($additial_args_name: $additial_args_type),*, callback: F)
            where
                F: Fn($arg_type) + 'static,
            {
                let this = self.clone();
                glib::spawn_future_local(async move {
                    let mut events = match this.proxy.[<receive_new_ $signal_name>]().await {
                        Ok(x) => x,
                        Err(e) => {
                            error!(concat!("Failed to receive new ", stringify!($signal_name), " events: {}"), e);
                            return;
                        }
                    };

                    while events.next().await.is_some() {
                        let new_value = match this.$signal_name($($additial_args_name),*).await {
                            Ok(x) => x,
                            Err(e) => {
                                error!(concat!("Failed to get ", stringify!($signal_name), ": {}"), e);
                                continue;
                            }
                        };
                        callback(new_value);
                    }
                });
            }
        }
    };
}

impl Item {
    pub async fn from_id(connection: &Connection, id: &str) -> anyhow::Result<Self> {
        // see https://github.com/KDE/plasma-workspace/blob/master/applets/systemtray/statusnotifieritemsource.cpp#L70
        let Some((bus_name, path)) = id.split_once('/') else {
            bail!("Invalid item id: {id}");
        };
        let path = "/".to_owned() + path;

        let proxy = ItemProxy::builder(connection)
            .destination(bus_name.to_owned())?
            .path(path)?
            .build()
            .await?;

        Ok(Self {
            proxy,
            tasks: Vec::new(),
        })
    }

    pub fn destination(&self) -> &BusName {
        self.proxy.inner().destination()
    }

    /// Return whether this method was actually called.
    ///
    /// This is useful because some menu-only items doesn't have an `activate` method, and doesn't report themselves as menu-only
    /// through [`Self::item_is_menu`]. nm-applet is an example of this.
    pub async fn activate(&self, x: i32, y: i32) -> zbus::Result<bool> {
        match self.proxy.activate(x, y).await {
            Ok(()) => Ok(true),
            Err(zbus::Error::MethodError(name, _, _))
                if name == "org.freedesktop.DBus.Error.UnknownMethod" =>
            {
                Ok(false)
            }
            Err(e) => {
                println!("{e:?}");
                Err(e)
            }
        }
    }

    pub async fn secondary_activate(&self, x: i32, y: i32) -> zbus::Result<()> {
        self.proxy.secondary_activate(x, y).await
    }

    /// Prefer using the provided menu via [`Self::menu`] instead of this
    pub async fn context_menu(&self, x: i32, y: i32) -> zbus::Result<()> {
        self.proxy.context_menu(x, y).await
    }

    pub async fn scroll(&self, delta: i32, orientation: Orientation) -> zbus::Result<()> {
        self.proxy.scroll(delta, orientation).await
    }

    pub async fn provide_xdg_activation_token(&self, token: &str) -> zbus::Result<()> {
        self.proxy.provide_xdg_activation_token(token).await
    }

    pub async fn category(&self) -> zbus::Result<Category> {
        self.proxy.category().await
    }

    pub async fn id(&self) -> zbus::Result<String> {
        self.proxy.id().await
    }

    pub async fn title(&self) -> zbus::Result<String> {
        self.proxy.title().await
    }

    pub async fn status(&self) -> zbus::Result<Status> {
        self.proxy.status().await
    }

    pub async fn window_id(&self) -> zbus::Result<i32> {
        self.proxy.window_id().await
    }

    pub async fn menu(&self) -> zbus::Result<Option<DBusMenu>> {
        match self.proxy.menu().await {
            Ok(path) if path.is_empty() => Ok(None),
            Ok(path) => Ok(Some(DBusMenu::new(self.destination(), &path))),
            // error is likely "No such property “Menu”"
            Err(zbus::Error::FDO(error)) if matches!(*error, fdo::Error::InvalidArgs(_)) => {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn item_is_menu(&self) -> zbus::Result<bool> {
        match self.proxy.item_is_menu().await {
            Ok(x) => Ok(x),
            // error is likely "No such property “ItemIsMenu”"
            Err(zbus::Error::FDO(error)) if matches!(*error, fdo::Error::InvalidArgs(_)) => {
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn icon(&self, size: i32, scale: i32) -> anyhow::Result<Paintable> {
        self.get_icon(
            self.proxy.icon_name().await.ok().as_deref(),
            self.proxy
                .icon_pixmap()
                .await
                .unwrap_or_else(|_| Vec::new()),
            size,
            scale,
        )
        .await
    }

    pub async fn overlay_icon(&self, size: i32, scale: i32) -> anyhow::Result<Paintable> {
        self.get_icon(
            self.proxy.overlay_icon_name().await.ok().as_deref(),
            self.proxy
                .overlay_icon_pixmap()
                .await
                .unwrap_or_else(|_| Vec::new()),
            size,
            scale,
        )
        .await
    }

    pub async fn attention_icon(&self, size: i32, scale: i32) -> anyhow::Result<Paintable> {
        self.get_icon(
            self.proxy.attention_icon_name().await.ok().as_deref(),
            self.proxy
                .attention_icon_pixmap()
                .await
                .unwrap_or_else(|_| Vec::new()),
            size,
            scale,
        )
        .await
    }

    // #[zbus(property(emits_changed_signal = "false"))]
    // fn attention_movie_name(&self) -> zbus::fdo::Result<String>;

    // /// Arguments:
    // ///   - icon name
    // ///   - icon pixmap
    // ///   - tooltip title
    // ///   - tooltip body (may contain [basic markup])
    // ///
    // ///   [basic markup]: https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/Markup/
    // #[zbus(property(emits_changed_signal = "false"))]
    // fn tool_tip(&self) -> zbus::fdo::Result<(String, Vec<Pixmap>, String, String)>;

    connect_function!(title, String);
    connect_function!(status, Status);
    connect_function!(icon, Paintable, size: i32, scale: i32);
    connect_function!(attention_icon, Paintable, size: i32, scale: i32);
    connect_function!(overlay_icon, Paintable, size: i32, scale: i32);
    connect_function!(menu, Option<DBusMenu>);

    // #[zbus(signal)]
    // fn new_tool_tip(&self) -> zbus::fdo::Result<()>;
}

/// Icon things
impl Item {
    async fn get_icon(
        &self,
        name: Option<&str>,
        pixmaps: Vec<Pixmap>,
        size: i32,
        scale: i32,
    ) -> anyhow::Result<Paintable> {
        if let Some(name) = name {
            let icon_path = Path::new(&name);
            if icon_path.is_absolute() && icon_path.is_file() {
                return Self::load_icon_from_path(icon_path, size, scale);
            }

            if let Some(icon) = self.get_icon_by_name(name, size, scale).await? {
                return Ok(icon.into());
            }
        }

        if let Some(icon) = Self::icon_from_pixmaps(pixmaps, size * scale) {
            return Ok(Texture::for_pixbuf(&icon).into());
        }

        let Some(display) = Display::default() else {
            bail!("No default display found");
        };
        let icon_theme = IconTheme::for_display(&display);
        Ok(icon_theme
            .lookup_icon(
                "image-missing",
                &[],
                size,
                scale,
                TextDirection::Ltr,
                IconLookupFlags::PRELOAD,
            )
            .into())
    }

    fn load_icon_from_path(path: &Path, size: i32, scale: i32) -> anyhow::Result<Paintable> {
        let scaled_size = size * scale;
        Ok(Texture::for_pixbuf(&Pixbuf::from_file_at_size(path, scaled_size, scaled_size)?).into())
    }

    async fn get_icon_by_name(
        &self,
        name: &str,
        size: i32,
        scale: i32,
    ) -> anyhow::Result<Option<IconPaintable>> {
        let Some(display) = Display::default() else {
            bail!("No default display found");
        };
        let icon_theme = IconTheme::for_display(&display);

        let old_search_path = icon_theme.search_path();
        if let Ok(additional_search_path) = self.proxy.icon_theme_path().await {
            icon_theme.add_search_path(additional_search_path);
        }

        let icon = if icon_theme.has_icon(name) {
            Some(icon_theme.lookup_icon(
                name,
                &[],
                size,
                scale,
                TextDirection::Ltr,
                IconLookupFlags::PRELOAD,
            ))
        } else {
            None
        };

        icon_theme.set_search_path(
            &old_search_path
                .iter()
                .map(PathBuf::as_path)
                .collect::<Vec<_>>(),
        );

        Ok(icon)
    }

    fn icon_from_pixmaps(pixmaps: Vec<Pixmap>, size: i32) -> Option<Pixbuf> {
        pixmaps
            .into_iter()
            .max_by(|pix1, pix2| {
                // take smallest one bigger than requested size, otherwise take biggest
                let a = size * size;
                let a1 = pix1.width * pix1.height;
                let a2 = pix2.width * pix2.height;
                match (a1 >= a, a2 >= a) {
                    (true, true) => a2.cmp(&a1),
                    (true, false) => Ordering::Greater,
                    (false, true) => Ordering::Less,
                    (false, false) => a1.cmp(&a2),
                }
            })
            .and_then(|pixmap| {
                let (width, height) = (pixmap.width, pixmap.height);
                let pixbuf = Self::icon_from_pixmap(pixmap);
                if width != size || height != size {
                    pixbuf.scale_simple(size, size, InterpType::Bilinear)
                } else {
                    Some(pixbuf)
                }
            })
    }

    /// Load a pixbuf from `StatusNotifierItem`'s [icon format].
    ///
    /// [icon format]: https://freedesktop.org/wiki/Specifications/StatusNotifierItem/Icons/
    fn icon_from_pixmap(mut pixmap: Pixmap) -> Pixbuf {
        // We need to convert data from ARGB32 to RGBA32, since that's the only one that gdk-pixbuf
        // understands.
        for chunk in pixmap.data.chunks_exact_mut(4) {
            let (a, r, g, b) = (chunk[0], chunk[1], chunk[2], chunk[3]);
            chunk[0] = r;
            chunk[1] = g;
            chunk[2] = b;
            chunk[3] = a;
        }

        Pixbuf::from_bytes(
            &Bytes::from_owned(pixmap.data),
            Colorspace::Rgb,
            true,
            8,
            pixmap.width,
            pixmap.height,
            pixmap.width * 4,
        )
    }
}

impl Clone for Item {
    fn clone(&self) -> Self {
        Self {
            proxy: self.proxy.clone(),
            // only the main instance have the tasks
            tasks: Vec::new(),
        }
    }
}

impl Drop for Item {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}
