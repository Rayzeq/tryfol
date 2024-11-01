use std::collections::HashMap;
use zbus::{proxy, zvariant::OwnedValue};

/// A DBus interface to expose menus on DBus.
///
/// Menu items are represented with a unique numeric id and a dictionary of properties.
///
/// To reduce the amount of DBus traffic, a property should only be returned
/// if its value is not the default value.
///
/// Available properties are:
///
/// |       Name       |   Type    | Default Value | Description                                               |
/// |------------------|-----------|---------------|-----------------------------------------------------------|
/// | type             | string    | "standard"    | Can be one of:                                            |
/// |                  |           |               |   - "standard": an item which can be clicked to           |
/// |                  |           |               |     trigger an action or show another menu                |
/// |                  |           |               |   - "separator": a separator                              |
/// |                  |           |               |                                                           |
/// |                  |           |               | Vendor specific types can be added by prefixing them with |
/// |                  |           |               | "x-<vendor>-".                                            |
/// |------------------|-----------|---------------|-----------------------------------------------------------|
/// | label            | string    | ""            | Text of the item, except that:                            |
/// |                  |           |               |   - two consecutive underscore characters "__" are        |
/// |                  |           |               |     displayed as a single underscore,                     |
/// |                  |           |               |   - any remaining underscore characters are not           |
/// |                  |           |               |     displayed at all,                                     |
/// |                  |           |               |   - the first of those remaining underscore characters    |
/// |                  |           |               |     (unless it is the last character in the string)       |
/// |                  |           |               |     indicates that the following character is the         |
/// |                  |           |               |     access key.                                           |
/// |------------------|-----------|---------------|-----------------------------------------------------------|
/// | enabled          | boolean   | true          | Whether the item can be activated or not.                 |
/// |------------------|-----------|---------------|-----------------------------------------------------------|
/// | visible          | boolean   | true          | True if the item is visible in the menu.                  |
/// |------------------|-----------|---------------|-----------------------------------------------------------|
/// | icon-name        | string    | ""            | Icon name of the item, following the freedesktop.org      |
/// |                  |           |               | icon spec.                                                |
/// |------------------|-----------|---------------|-----------------------------------------------------------|
/// | icon-data        | binary    | Empty         | PNG data of the icon.                                     |
/// |------------------|-----------|---------------|-----------------------------------------------------------|
/// | shortcut         | array of  | Empty         | The shortcut of the item. Each array represents the       |
/// |                  | arrays of |               | key press in the list of keypresses. Each list of strings |
/// |                  | strings   |               | contains a list of modifiers and then the key that is     |
/// |                  |           |               | used. The modifier strings allowed are: "Control", "Alt", |
/// |                  |           |               | "Shift" and "Super".                                      |
/// |                  |           |               | - A simple shortcut like Ctrl+S is represented as:        |
/// |                  |           |               |   [["Control", "S"]]                                      |
/// |                  |           |               | - A complex shortcut like Ctrl+Q, Alt+X is represented    |
/// |                  |           |               |   as: [["Control", "Q"], ["Alt", "X"]]                    |
/// |------------------|-----------|---------------|-----------------------------------------------------------|
/// | toggle-type      | string    | ""            | If the item can be toggled, this property should be       |
/// |                  |           |               | set to:                                                   |
/// |                  |           |               |   - "checkmark": Item is an independent togglable item    |
/// |                  |           |               |   - "radio": Item is part of a group where only one       |
/// |                  |           |               |      item can be toggled at a time                        |
/// |                  |           |               |   - "": Item cannot be toggled                            |
/// |------------------|-----------|---------------|-----------------------------------------------------------|
/// | toggle-state     | int       | -1            | Describe the current state of a "togglable" item.         |
/// |                  |           |               | Can be one of:                                            |
/// |                  |           |               |   - 0 = off                                               |
/// |                  |           |               |   - 1 = on                                                |
/// |                  |           |               |   - anything else = indeterminate                         |
/// |                  |           |               |                                                           |
/// |                  |           |               |   Note: The implementation does not itself handle         |
/// |                  |           |               |   ensuring that only one item in a radio group is set     |
/// |                  |           |               |   to "on", or that a group does not have "on" and         |
/// |                  |           |               |   "indeterminate" items simultaneously; maintaining       |
/// |                  |           |               |   this policy is up to the toolkit wrappers.              |
/// |------------------|-----------|---------------|-----------------------------------------------------------|
/// | children-display | string    | ""            | If the menu item has children this property should be     |
/// |                  |           |               | set to "submenu".                                         |
/// |------------------|-----------|---------------|-----------------------------------------------------------|
/// | disposition      | string    | "normal"      | How the menuitem feels the information it's displaying    |
/// |                  |           |               | to the user should be presented.                          |
/// |                  |           |               | - "normal" a standard menu item                           |
/// |                  |           |               | - "informative" providing additional information to the   |
/// |                  |           |               |    user                                                   |
/// |                  |           |               | - "warning" looking at potentially harmful results        |
/// |                  |           |               | - "alert" something bad could potentially happen          |
/// |------------------|-----------|---------------|-----------------------------------------------------------|
///
/// Vendor specific properties can be added by prefixing them with "x-<vendor>-".
#[proxy(interface = "com.canonical.dbusmenu")]
pub trait DbusMenu {
    /// Provides the layout and propertiers that are attached to the entries
    /// that are in the layout.  It only gives the items that are children
    /// of the item that is specified in @a parentId.  It will return all of the
    /// properties or specific ones depending of the value in @a propertyNames.
    ///
    /// The format is recursive, where the second 'v' is in the same format
    /// as the original 'a(ia{sv}av)'.  Its content depends on the value
    /// of @a recursionDepth.
    fn get_layout(
        &self,
        // The ID of the parent node for the layout.
        // For grabbing the layout from the root node use zero.
        parent_id: i32,
        // The amount of levels of recursion to use.
        // This affects the content of the second variant array.
        //  - -1: deliver all the items under the `parent_id`.
        //  - 0: no recursion, the array will be empty.
        //  - n: array will contains items up to 'n' level depth.
        recursion_depth: i32,
        // The list of item properties we are interested in.
        // If there are no entries in the list all of the properties will be sent.
        property_names: &[&str],
    ) -> zbus::Result<(
        // The revision number of the layout. For matching with `layout_updated` signals.
        u32,
        // The layout, as a recursive structure.
        (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>),
    )>;

    /// Returns the list of items which are children of `parent_id`.
    fn get_group_properties(
        &self,
        // A list of ids that we should be finding the properties on.
        // If the list is empty, all menu items should be sent.
        ids: &[i32],
        // The list of item properties we are interested in.
        // If there are no entries in the list all of the properties will be sent.
        property_names: &[&str],
    ) -> zbus::Result<
        Vec<(
            // the item id
            i32,
            // the requested item properties
            HashMap<String, OwnedValue>,
        )>,
    >;

    /// Get a signal property on a single item. This is not useful if you're
    /// going to implement this interface, it should only be used if you're
    /// debugging via a commandline tool.
    fn get_property(
        &self,
        // The id of the item which received the event
        id: i32,
        // The name of the property to get
        name: &str,
    ) -> zbus::Result<OwnedValue>;

    /// This is called by the applet to notify the application an event happened on a menu item.
    /// `event_id` can be one of the following:
    ///   - "clicked"
    ///   - "hovered"
    ///   - "opened"
    ///   - "closed"
    /// Vendor specific events can be added by prefixing them with "x-<vendor>-"
    fn event(
        &self,
        // The id of the item which received the event
        id: i32,
        // The type of event
        r#type: &str,
        // Event-specific data
        data: OwnedValue,
        // The time that the event occured if available or the time the message was sent if not
        timestamp: u32,
    ) -> zbus::Result<()>;

    /// Used to pass a set of events as a single message for possibily several different
    /// menuitems. This is done to optimize DBus traffic.
    fn event_group(
        &self,
        // An array of all the events that should be passed. This tuple should match the
        // parameters of the 'Event' signal. Which is roughly: id, event_id, data and timestamp.
        events: &[(i32, &str, OwnedValue, u32)],
    ) -> zbus::Result<
        // A list of menuitem IDs that couldn't be found. If none of the ones in the list
        // can be found, a DBus error is returned.
        Vec<i32>,
    >;

    /// This is called by the applet to notify the application that it is about to show
    /// the menu under the specified item.
    fn about_to_show(
        &self,
        // Which menu item represents the parent of the item about to be shown.
        id: i32,
    ) -> zbus::Result<
        // Whether this AboutToShow event should result in the menu being updated.
        bool,
    >;

    /// A function to tell several menus being shown that they are about to be shown to the user.
    /// This is likely only useful for programitc purposes so while the return values are returned,
    /// in general, the singular function should be used in most user interacation scenarios.
    fn about_to_show_group(
        &self,
        // The IDs of the menu items who's submenus are being shown.
        ids: &[i32],
    ) -> zbus::Result<(
        // The IDs of the menus that need updates.
        // Note: if no update information is needed the DBus message should set the no reply flag.
        Vec<i32>,
        // A list of menuitem IDs that couldn't be found. If none of the ones in the list
        // can be found, a DBus error is returned.
        Vec<i32>,
    )>;

    /// Provides the version of the DBusmenu API that this API is implementing.
    #[zbus(property)]
    fn version(&self) -> zbus::Result<u32>;

    /// Represents the way the text direction of the application. This allows
    /// the server to handle mismatches intelligently. For left-to-right the
    /// string is "ltr" for right-to-left it is "rtl".
    #[zbus(property)]
    fn text_direction(&self) -> zbus::Result<String>;

    /// Tells if the menus are in a normal state or they believe that they
    /// could use some attention. Cases for showing them would be if help
    /// were referring to them or they accessors were being highlighted.
    /// This property can have two values: "normal" in almost all cases and
    /// "notice" when they should have a higher priority to be shown.
    #[zbus(property)]
    fn status(&self) -> zbus::Result<String>;

    /// A list of directories that should be used for finding icons using
    /// the icon naming spec.  Idealy there should only be one for the icon
    /// theme, but additional ones are often added by applications for
    /// app specific icons.
    #[zbus(property)]
    fn icon_theme_path(&self) -> zbus::Result<Vec<String>>;

    /// Triggered when there are lots of property updates across many items so they all
    /// get grouped into a single dbus message. The format is the ID of the item with a
    /// hashtable of names and values for those properties.
    #[zbus(signal)]
    fn items_properties_updated(
        &self,
        updated_props: Vec<(i32, HashMap<String, OwnedValue>)>,
        removed_props: Vec<(i32, Vec<String>)>,
    ) -> zbus::Result<()>;

    /// Triggered by the application to notify display of a layout update, up to revision
    #[zbus(signal)]
    fn layout_updated(
        &self,
        // The revision of the layout that we're currently on
        revision: u32,
        // If the layout update is only of a subtree, this is the parent item
        // for the entries that have changed. It is zero if the whole layout
        // should be considered invalid.
        parent: i32,
    ) -> zbus::Result<()>;

    /// The server is requesting that all clients displaying this menu open it to the user.
    /// This would be for things like hotkeys that when the user presses them the menu should
    /// open and display itself to the user.
    #[zbus(signal)]
    fn item_activation_requested(
        &self,
        // ID of the menu that should be activated
        id: i32,
        // The time that the event occured
        timestamp: u32,
    ) -> zbus::Result<()>;
}
