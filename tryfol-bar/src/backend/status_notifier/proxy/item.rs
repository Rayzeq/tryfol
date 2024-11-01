use serde::{Deserialize, Serialize};
use zbus::{
    proxy,
    zvariant::{self, OwnedObjectPath, OwnedValue},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, zvariant::Type)]
#[serde(rename_all = "lowercase")]
#[zvariant(signature = "s")]
pub enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, strum::EnumString)]
pub enum Category {
    /// The item describes the status of a generic application, for instance the current state of a media player.
    ///
    /// In the case where the category of the item can not be known, such as when the item is being proxied
    /// from another incompatible or emulated system, this can be used as a sensible default fallback.
    ApplicationStatus,

    /// The item describes the status of communication oriented applications, like an instant messenger or an email client.
    Communications,

    /// The item describes services of the system not seen as a standalone application by the user,
    /// such as an indicator for the activity of a disk indexing service.
    SystemServices,

    /// The item describes the state and control of a particular hardware,
    /// such as an indicator of the battery charge or sound card volume control.
    Hardware,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    strum::EnumString,
    zvariant::Type,
    Deserialize,
)]
#[zvariant(signature = "s")]
pub enum Status {
    /// The item doesn't convey important information to the user,
    /// it can be considered an "idle" status and is likely that visualizations will chose to hide it.
    Passive,

    /// The item is active, it is more important that the item will be shown in some way to the user.
    Active,

    /// The item carries really important information for the user, such as battery charge running out
    /// and is wants to incentive the direct user intervention.
    /// Visualizations should emphasize in some way the items with this status.
    NeedsAttention,
}

#[derive(Debug, Clone, zvariant::Value, zvariant::OwnedValue)]
pub struct Pixmap {
    pub width: i32,
    pub height: i32,
    /// ARGB32 binary representation of the icon.
    pub data: Vec<u8>,
}

#[proxy(interface = "org.kde.StatusNotifierItem")]
pub trait Item {
    /// Asks the status notifier item for activation, this is typically a consequence of user input,
    /// such as mouse left click over the graphical representation of the item.
    /// The application will perform any task is considered appropriate as an activation request.
    /// The x and y parameters are in screen coordinates and is to be considered an hint to the item
    /// where to show eventual windows (if any).
    fn activate(&self, x: i32, y: i32) -> zbus::Result<()>;

    /// Is to be considered a secondary and less important form of activation compared to Activate.
    /// This is typically a consequence of user input, such as mouse middle click over the graphical
    /// representation of the item.
    /// The application will perform any task is considered appropriate as an activation request.
    /// The x and y parameters are in screen coordinates and is to be considered an hint to the item
    /// where to show eventual windows (if any).
    fn secondary_activate(&self, x: i32, y: i32) -> zbus::Result<()>;

    /// Asks the status notifier item to show a context menu, this is typically a consequence of user input,
    /// such as mouse right click over the graphical representation of the item.
    /// The x and y parameters are in screen coordinates and is to be considered an hint to the item
    /// about where to show the context menu.
    fn context_menu(&self, x: i32, y: i32) -> zbus::Result<()>;

    /// The user asked for a scroll action.
    ///
    /// This is caused from input such as mouse wheel over the graphical representation of the item.
    fn scroll(&self, delta: i32, orientation: Orientation) -> zbus::Result<()>;

    fn provide_xdg_activation_token(&self, token: &str) -> zbus::Result<()>;

    #[zbus(property)]
    fn category(&self) -> zbus::Result<Category>;

    /// A name that should be unique for this application and consistent between sessions,
    /// such as the application name itself.
    #[zbus(property)]
    fn id(&self) -> zbus::Result<String>;

    /// A name that describes the application, it can be more descriptive than [`Self::id`].
    #[zbus(property(emits_changed_signal = "false"))]
    fn title(&self) -> zbus::Result<String>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn status(&self) -> zbus::Result<Status>;

    /// It's the windowing-system dependent identifier for a window, the application can chose one of its windows
    /// to be available through this property or just set 0 if it's not interested.
    #[zbus(property)]
    fn window_id(&self) -> zbus::Result<i32>;

    /// An additional path to add to the theme search path to find the specified icons.
    #[zbus(property)]
    fn icon_theme_path(&self) -> zbus::Result<String>;

    /// DBus path to an object which should implement the com.canonical.dbusmenu interface
    #[zbus(property(emits_changed_signal = "false"))]
    fn menu(&self) -> zbus::Result<OwnedObjectPath>;

    /// The item only support the context menu, the visualization should prefer showing the menu
    /// or sending [`Self::context_menu`] instead of [`Self::activate`]
    #[zbus(property)]
    fn item_is_menu(&self) -> zbus::Result<bool>;

    /// The icon that should be shown.
    ///
    /// Using this property is preferred over [`Self::icon_pixmap`].
    #[zbus(property(emits_changed_signal = "false"))]
    fn icon_name(&self) -> zbus::Result<String>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn icon_pixmap(&self) -> zbus::Result<Vec<Pixmap>>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn overlay_icon_name(&self) -> zbus::Result<String>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn overlay_icon_pixmap(&self) -> zbus::Result<Vec<Pixmap>>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn attention_icon_name(&self) -> zbus::Result<String>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn attention_icon_pixmap(&self) -> zbus::Result<Vec<Pixmap>>;

    #[zbus(property(emits_changed_signal = "false"))]
    fn attention_movie_name(&self) -> zbus::Result<String>;

    /// Arguments:
    ///   - icon name
    ///   - icon pixmap
    ///   - tooltip title
    ///   - tooltip body (may contain [basic markup])
    ///   
    ///   [basic markup]: https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/Markup/
    #[zbus(property(emits_changed_signal = "false"))]
    fn tool_tip(&self) -> zbus::Result<(String, Vec<Pixmap>, String, String)>;

    #[zbus(signal)]
    fn new_title(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn new_icon(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn new_attention_icon(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn new_overlay_icon(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn new_menu(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn new_tool_tip(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn new_status(&self, status: Status) -> zbus::Result<()>;
}

impl ItemProxy<'_> {
    pub const PATH: &'static str = "/StatusNotifierItem";
}

impl TryFrom<OwnedValue> for Category {
    type Error = zvariant::Error;

    fn try_from(value: OwnedValue) -> Result<Self, Self::Error> {
        let value: String = value.try_into()?;
        value
            .parse()
            .map_err(|e: strum::ParseError| zvariant::Error::Message(e.to_string()))
    }
}

impl TryFrom<OwnedValue> for Status {
    type Error = zvariant::Error;

    fn try_from(value: OwnedValue) -> Result<Self, Self::Error> {
        let value: String = value.try_into()?;
        value
            .parse()
            .map_err(|e: strum::ParseError| zvariant::Error::Message(e.to_string()))
    }
}
