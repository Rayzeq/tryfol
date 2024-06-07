// This file was generated by gir (https://github.com/gtk-rs/gir)
// from /nix/store/4dxx74s4g3rrn6haryx8i6yzy91f5q7m-source
// from /nix/store/687zj3l24wawn3a93nkqqcv6g0hjm9n5-dbusmenu-gtk3-gir
// DO NOT EDIT

#![allow(non_camel_case_types, non_upper_case_globals, non_snake_case)]
#![allow(clippy::approx_constant, clippy::type_complexity, clippy::unreadable_literal, clippy::upper_case_acronyms)]
#![cfg_attr(feature = "dox", feature(doc_cfg))]


#[allow(unused_imports)]
use libc::{c_int, c_char, c_uchar, c_float, c_uint, c_double,
    c_short, c_ushort, c_long, c_ulong,
    c_void, size_t, ssize_t, intptr_t, uintptr_t, FILE};

#[allow(unused_imports)]
use glib::{gboolean, gconstpointer, gpointer, GType};

// Enums
pub type DbusmenuStatus = c_int;
pub const DBUSMENU_STATUS_NORMAL: DbusmenuStatus = 0;
pub const DBUSMENU_STATUS_NOTICE: DbusmenuStatus = 1;

pub type DbusmenuTextDirection = c_int;
pub const DBUSMENU_TEXT_DIRECTION_NONE: DbusmenuTextDirection = 0;
pub const DBUSMENU_TEXT_DIRECTION_LTR: DbusmenuTextDirection = 1;
pub const DBUSMENU_TEXT_DIRECTION_RTL: DbusmenuTextDirection = 2;

// Constants
pub const DBUSMENU_CLIENT_PROP_DBUS_NAME: &[u8] = b"dbus-name\0";
pub const DBUSMENU_CLIENT_PROP_DBUS_OBJECT: &[u8] = b"dbus-object\0";
pub const DBUSMENU_CLIENT_PROP_GROUP_EVENTS: &[u8] = b"group-events\0";
pub const DBUSMENU_CLIENT_PROP_STATUS: &[u8] = b"status\0";
pub const DBUSMENU_CLIENT_PROP_TEXT_DIRECTION: &[u8] = b"text-direction\0";
pub const DBUSMENU_CLIENT_SIGNAL_EVENT_RESULT: &[u8] = b"event-result\0";
pub const DBUSMENU_CLIENT_SIGNAL_ICON_THEME_DIRS_CHANGED: &[u8] = b"icon-theme-dirs-changed\0";
pub const DBUSMENU_CLIENT_SIGNAL_ITEM_ACTIVATE: &[u8] = b"item-activate\0";
pub const DBUSMENU_CLIENT_SIGNAL_LAYOUT_UPDATED: &[u8] = b"layout-updated\0";
pub const DBUSMENU_CLIENT_SIGNAL_NEW_MENUITEM: &[u8] = b"new-menuitem\0";
pub const DBUSMENU_CLIENT_SIGNAL_ROOT_CHANGED: &[u8] = b"root-changed\0";
pub const DBUSMENU_CLIENT_TYPES_DEFAULT: &[u8] = b"standard\0";
pub const DBUSMENU_CLIENT_TYPES_IMAGE: &[u8] = b"standard\0";
pub const DBUSMENU_CLIENT_TYPES_SEPARATOR: &[u8] = b"separator\0";
pub const DBUSMENU_MENUITEM_CHILD_DISPLAY_SUBMENU: &[u8] = b"submenu\0";
pub const DBUSMENU_MENUITEM_DISPOSITION_ALERT: &[u8] = b"alert\0";
pub const DBUSMENU_MENUITEM_DISPOSITION_INFORMATIVE: &[u8] = b"informative\0";
pub const DBUSMENU_MENUITEM_DISPOSITION_NORMAL: &[u8] = b"normal\0";
pub const DBUSMENU_MENUITEM_DISPOSITION_WARNING: &[u8] = b"warning\0";
pub const DBUSMENU_MENUITEM_EVENT_ACTIVATED: &[u8] = b"clicked\0";
pub const DBUSMENU_MENUITEM_EVENT_CLOSED: &[u8] = b"closed\0";
pub const DBUSMENU_MENUITEM_EVENT_OPENED: &[u8] = b"opened\0";
pub const DBUSMENU_MENUITEM_ICON_NAME_BLANK: &[u8] = b"blank-icon\0";
pub const DBUSMENU_MENUITEM_PROP_ACCESSIBLE_DESC: &[u8] = b"accessible-desc\0";
pub const DBUSMENU_MENUITEM_PROP_CHILD_DISPLAY: &[u8] = b"children-display\0";
pub const DBUSMENU_MENUITEM_PROP_DISPOSITION: &[u8] = b"disposition\0";
pub const DBUSMENU_MENUITEM_PROP_ENABLED: &[u8] = b"enabled\0";
pub const DBUSMENU_MENUITEM_PROP_ICON_DATA: &[u8] = b"icon-data\0";
pub const DBUSMENU_MENUITEM_PROP_ICON_NAME: &[u8] = b"icon-name\0";
pub const DBUSMENU_MENUITEM_PROP_LABEL: &[u8] = b"label\0";
pub const DBUSMENU_MENUITEM_PROP_SHORTCUT: &[u8] = b"shortcut\0";
pub const DBUSMENU_MENUITEM_PROP_TOGGLE_STATE: &[u8] = b"toggle-state\0";
pub const DBUSMENU_MENUITEM_PROP_TOGGLE_TYPE: &[u8] = b"toggle-type\0";
pub const DBUSMENU_MENUITEM_PROP_TYPE: &[u8] = b"type\0";
pub const DBUSMENU_MENUITEM_PROP_VISIBLE: &[u8] = b"visible\0";
pub const DBUSMENU_MENUITEM_SHORTCUT_ALT: &[u8] = b"Alt\0";
pub const DBUSMENU_MENUITEM_SHORTCUT_CONTROL: &[u8] = b"Control\0";
pub const DBUSMENU_MENUITEM_SHORTCUT_SHIFT: &[u8] = b"Shift\0";
pub const DBUSMENU_MENUITEM_SHORTCUT_SUPER: &[u8] = b"Super\0";
pub const DBUSMENU_MENUITEM_SIGNAL_ABOUT_TO_SHOW: &[u8] = b"about-to-show\0";
pub const DBUSMENU_MENUITEM_SIGNAL_CHILD_ADDED: &[u8] = b"child-added\0";
pub const DBUSMENU_MENUITEM_SIGNAL_CHILD_MOVED: &[u8] = b"child-moved\0";
pub const DBUSMENU_MENUITEM_SIGNAL_CHILD_REMOVED: &[u8] = b"child-removed\0";
pub const DBUSMENU_MENUITEM_SIGNAL_EVENT: &[u8] = b"event\0";
pub const DBUSMENU_MENUITEM_SIGNAL_ITEM_ACTIVATED: &[u8] = b"item-activated\0";
pub const DBUSMENU_MENUITEM_SIGNAL_PROPERTY_CHANGED: &[u8] = b"property-changed\0";
pub const DBUSMENU_MENUITEM_SIGNAL_REALIZED: &[u8] = b"realized\0";
pub const DBUSMENU_MENUITEM_SIGNAL_SHOW_TO_USER: &[u8] = b"show-to-user\0";
pub const DBUSMENU_MENUITEM_TOGGLE_CHECK: &[u8] = b"checkmark\0";
pub const DBUSMENU_MENUITEM_TOGGLE_RADIO: &[u8] = b"radio\0";
pub const DBUSMENU_MENUITEM_TOGGLE_STATE_CHECKED: c_int = 1;
pub const DBUSMENU_MENUITEM_TOGGLE_STATE_UNCHECKED: c_int = 0;
pub const DBUSMENU_MENUITEM_TOGGLE_STATE_UNKNOWN: c_int = -1;
pub const DBUSMENU_SERVER_PROP_DBUS_OBJECT: &[u8] = b"dbus-object\0";
pub const DBUSMENU_SERVER_PROP_ROOT_NODE: &[u8] = b"root-node\0";
pub const DBUSMENU_SERVER_PROP_STATUS: &[u8] = b"status\0";
pub const DBUSMENU_SERVER_PROP_TEXT_DIRECTION: &[u8] = b"text-direction\0";
pub const DBUSMENU_SERVER_PROP_VERSION: &[u8] = b"version\0";
pub const DBUSMENU_SERVER_SIGNAL_ID_PROP_UPDATE: &[u8] = b"item-property-updated\0";
pub const DBUSMENU_SERVER_SIGNAL_ID_UPDATE: &[u8] = b"item-updated\0";
pub const DBUSMENU_SERVER_SIGNAL_ITEM_ACTIVATION: &[u8] = b"item-activation-requested\0";
pub const DBUSMENU_SERVER_SIGNAL_LAYOUT_UPDATED: &[u8] = b"layout-updated\0";

// Callbacks
pub type DbusmenuClientTypeHandler = Option<unsafe extern "C" fn(*mut DbusmenuMenuitem, *mut DbusmenuMenuitem, *mut DbusmenuClient, gpointer) -> gboolean>;
pub type dbusmenu_menuitem_about_to_show_cb = Option<unsafe extern "C" fn(*mut DbusmenuMenuitem, gpointer)>;
pub type dbusmenu_menuitem_buildvariant_slot_t = Option<unsafe extern "C" fn(*mut DbusmenuMenuitem, *mut *mut c_char) -> *mut glib::GVariant>;

// Records
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DbusmenuClientClass {
    pub parent_class: gobject::GObjectClass,
    pub layout_updated: Option<unsafe extern "C" fn()>,
    pub root_changed: Option<unsafe extern "C" fn(*mut DbusmenuMenuitem)>,
    pub new_menuitem: Option<unsafe extern "C" fn(*mut DbusmenuMenuitem)>,
    pub item_activate: Option<unsafe extern "C" fn(*mut DbusmenuMenuitem, c_uint)>,
    pub event_result: Option<unsafe extern "C" fn(*mut DbusmenuMenuitem, *mut c_char, *mut glib::GVariant, c_uint, *mut glib::GError)>,
    pub icon_theme_dirs: Option<unsafe extern "C" fn(*mut DbusmenuMenuitem, gpointer, *mut glib::GError)>,
    pub reserved1: Option<unsafe extern "C" fn()>,
    pub reserved2: Option<unsafe extern "C" fn()>,
    pub reserved3: Option<unsafe extern "C" fn()>,
    pub reserved4: Option<unsafe extern "C" fn()>,
    pub reserved5: Option<unsafe extern "C" fn()>,
}

impl ::std::fmt::Debug for DbusmenuClientClass {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("DbusmenuClientClass @ {self:p}"))
         .field("parent_class", &self.parent_class)
         .field("layout_updated", &self.layout_updated)
         .field("root_changed", &self.root_changed)
         .field("new_menuitem", &self.new_menuitem)
         .field("item_activate", &self.item_activate)
         .field("event_result", &self.event_result)
         .field("icon_theme_dirs", &self.icon_theme_dirs)
         .field("reserved1", &self.reserved1)
         .field("reserved2", &self.reserved2)
         .field("reserved3", &self.reserved3)
         .field("reserved4", &self.reserved4)
         .field("reserved5", &self.reserved5)
         .finish()
    }
}

#[repr(C)]
pub struct _DbusmenuClientPrivate {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

pub type DbusmenuClientPrivate = *mut _DbusmenuClientPrivate;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct DbusmenuMenuitemClass {
    pub parent_class: gobject::GObjectClass,
    pub property_changed: Option<unsafe extern "C" fn(*mut c_char, *mut glib::GVariant)>,
    pub item_activated: Option<unsafe extern "C" fn(c_uint)>,
    pub child_added: Option<unsafe extern "C" fn(*mut DbusmenuMenuitem, c_uint)>,
    pub child_removed: Option<unsafe extern "C" fn(*mut DbusmenuMenuitem)>,
    pub child_moved: Option<unsafe extern "C" fn(*mut DbusmenuMenuitem, c_uint, c_uint)>,
    pub realized: Option<unsafe extern "C" fn()>,
    pub buildvariant: dbusmenu_menuitem_buildvariant_slot_t,
    pub handle_event: Option<unsafe extern "C" fn(*mut DbusmenuMenuitem, *const c_char, *mut glib::GVariant, c_uint)>,
    pub send_about_to_show: Option<unsafe extern "C" fn(*mut DbusmenuMenuitem, dbusmenu_menuitem_about_to_show_cb, gpointer)>,
    pub show_to_user: Option<unsafe extern "C" fn(*mut DbusmenuMenuitem, c_uint, gpointer)>,
    pub about_to_show: Option<unsafe extern "C" fn() -> gboolean>,
    pub event: Option<unsafe extern "C" fn(*const c_char, *mut glib::GVariant, c_uint)>,
    pub reserved1: Option<unsafe extern "C" fn()>,
    pub reserved2: Option<unsafe extern "C" fn()>,
    pub reserved3: Option<unsafe extern "C" fn()>,
    pub reserved4: Option<unsafe extern "C" fn()>,
    pub reserved5: Option<unsafe extern "C" fn()>,
}

impl ::std::fmt::Debug for DbusmenuMenuitemClass {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("DbusmenuMenuitemClass @ {self:p}"))
         .field("parent_class", &self.parent_class)
         .field("property_changed", &self.property_changed)
         .field("item_activated", &self.item_activated)
         .field("child_added", &self.child_added)
         .field("child_removed", &self.child_removed)
         .field("child_moved", &self.child_moved)
         .field("realized", &self.realized)
         .field("buildvariant", &self.buildvariant)
         .field("handle_event", &self.handle_event)
         .field("send_about_to_show", &self.send_about_to_show)
         .field("show_to_user", &self.show_to_user)
         .field("about_to_show", &self.about_to_show)
         .field("event", &self.event)
         .field("reserved1", &self.reserved1)
         .field("reserved2", &self.reserved2)
         .field("reserved3", &self.reserved3)
         .field("reserved4", &self.reserved4)
         .field("reserved5", &self.reserved5)
         .finish()
    }
}

#[repr(C)]
pub struct _DbusmenuMenuitemPrivate {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

pub type DbusmenuMenuitemPrivate = *mut _DbusmenuMenuitemPrivate;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct DbusmenuMenuitemProxyClass {
    pub parent_class: DbusmenuMenuitemClass,
    pub reserved1: Option<unsafe extern "C" fn()>,
    pub reserved2: Option<unsafe extern "C" fn()>,
    pub reserved3: Option<unsafe extern "C" fn()>,
    pub reserved4: Option<unsafe extern "C" fn()>,
}

impl ::std::fmt::Debug for DbusmenuMenuitemProxyClass {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("DbusmenuMenuitemProxyClass @ {self:p}"))
         .field("parent_class", &self.parent_class)
         .field("reserved1", &self.reserved1)
         .field("reserved2", &self.reserved2)
         .field("reserved3", &self.reserved3)
         .field("reserved4", &self.reserved4)
         .finish()
    }
}

#[repr(C)]
pub struct _DbusmenuMenuitemProxyPrivate {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

pub type DbusmenuMenuitemProxyPrivate = *mut _DbusmenuMenuitemProxyPrivate;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct DbusmenuServerClass {
    pub parent_class: gobject::GObjectClass,
    pub id_prop_update: Option<unsafe extern "C" fn(c_int, *mut c_char, *mut c_char)>,
    pub id_update: Option<unsafe extern "C" fn(c_int)>,
    pub layout_updated: Option<unsafe extern "C" fn(c_int)>,
    pub item_activation: Option<unsafe extern "C" fn(c_int, c_uint)>,
    pub reserved1: Option<unsafe extern "C" fn()>,
    pub reserved2: Option<unsafe extern "C" fn()>,
    pub reserved3: Option<unsafe extern "C" fn()>,
    pub reserved4: Option<unsafe extern "C" fn()>,
    pub reserved5: Option<unsafe extern "C" fn()>,
    pub reserved6: Option<unsafe extern "C" fn()>,
}

impl ::std::fmt::Debug for DbusmenuServerClass {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("DbusmenuServerClass @ {self:p}"))
         .field("parent_class", &self.parent_class)
         .field("id_prop_update", &self.id_prop_update)
         .field("id_update", &self.id_update)
         .field("layout_updated", &self.layout_updated)
         .field("item_activation", &self.item_activation)
         .field("reserved1", &self.reserved1)
         .field("reserved2", &self.reserved2)
         .field("reserved3", &self.reserved3)
         .field("reserved4", &self.reserved4)
         .field("reserved5", &self.reserved5)
         .field("reserved6", &self.reserved6)
         .finish()
    }
}

#[repr(C)]
pub struct _DbusmenuServerPrivate {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

pub type DbusmenuServerPrivate = *mut _DbusmenuServerPrivate;

// Classes
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DbusmenuClient {
    pub parent: gobject::GObject,
    pub priv_: *mut DbusmenuClientPrivate,
}

impl ::std::fmt::Debug for DbusmenuClient {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("DbusmenuClient @ {self:p}"))
         .finish()
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct DbusmenuMenuitem {
    pub parent: gobject::GObject,
    pub priv_: *mut DbusmenuMenuitemPrivate,
}

impl ::std::fmt::Debug for DbusmenuMenuitem {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("DbusmenuMenuitem @ {self:p}"))
         .field("parent", &self.parent)
         .field("priv_", &self.priv_)
         .finish()
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct DbusmenuMenuitemProxy {
    pub parent: DbusmenuMenuitem,
    pub priv_: *mut DbusmenuMenuitemProxyPrivate,
}

impl ::std::fmt::Debug for DbusmenuMenuitemProxy {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("DbusmenuMenuitemProxy @ {self:p}"))
         .finish()
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct DbusmenuServer {
    pub parent: gobject::GObject,
    pub priv_: *mut DbusmenuServerPrivate,
}

impl ::std::fmt::Debug for DbusmenuServer {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct(&format!("DbusmenuServer @ {self:p}"))
         .finish()
    }
}

extern "C" {

    //=========================================================================
    // DbusmenuClient
    //=========================================================================
    pub fn dbusmenu_client_get_type() -> GType;
    pub fn dbusmenu_client_new(name: *const c_char, object: *const c_char) -> *mut DbusmenuClient;
    pub fn dbusmenu_client_add_type_handler(client: *mut DbusmenuClient, type_: *const c_char, newfunc: DbusmenuClientTypeHandler) -> gboolean;
    pub fn dbusmenu_client_add_type_handler_full(client: *mut DbusmenuClient, type_: *const c_char, newfunc: DbusmenuClientTypeHandler, user_data: gpointer, destroy_func: glib::GDestroyNotify) -> gboolean;
    pub fn dbusmenu_client_get_icon_paths(client: *mut DbusmenuClient) -> glib::GStrv;
    pub fn dbusmenu_client_get_root(client: *mut DbusmenuClient) -> *mut DbusmenuMenuitem;
    pub fn dbusmenu_client_get_status(client: *mut DbusmenuClient) -> DbusmenuStatus;
    pub fn dbusmenu_client_get_text_direction(client: *mut DbusmenuClient) -> DbusmenuTextDirection;

    //=========================================================================
    // DbusmenuMenuitem
    //=========================================================================
    pub fn dbusmenu_menuitem_get_type() -> GType;
    pub fn dbusmenu_menuitem_new() -> *mut DbusmenuMenuitem;
    pub fn dbusmenu_menuitem_new_with_id(id: c_int) -> *mut DbusmenuMenuitem;
    pub fn dbusmenu_menuitem_child_add_position(mi: *mut DbusmenuMenuitem, child: *mut DbusmenuMenuitem, position: c_uint) -> gboolean;
    pub fn dbusmenu_menuitem_child_append(mi: *mut DbusmenuMenuitem, child: *mut DbusmenuMenuitem) -> gboolean;
    pub fn dbusmenu_menuitem_child_delete(mi: *mut DbusmenuMenuitem, child: *mut DbusmenuMenuitem) -> gboolean;
    pub fn dbusmenu_menuitem_child_find(mi: *mut DbusmenuMenuitem, id: c_int) -> *mut DbusmenuMenuitem;
    pub fn dbusmenu_menuitem_child_prepend(mi: *mut DbusmenuMenuitem, child: *mut DbusmenuMenuitem) -> gboolean;
    pub fn dbusmenu_menuitem_child_reorder(mi: *mut DbusmenuMenuitem, child: *mut DbusmenuMenuitem, position: c_uint) -> gboolean;
    pub fn dbusmenu_menuitem_find_id(mi: *mut DbusmenuMenuitem, id: c_int) -> *mut DbusmenuMenuitem;
    pub fn dbusmenu_menuitem_foreach(mi: *mut DbusmenuMenuitem, func: *mut gpointer, data: gpointer);
    pub fn dbusmenu_menuitem_get_children(mi: *mut DbusmenuMenuitem) -> *mut glib::GList;
    pub fn dbusmenu_menuitem_get_id(mi: *mut DbusmenuMenuitem) -> c_int;
    pub fn dbusmenu_menuitem_get_parent(mi: *mut DbusmenuMenuitem) -> *mut DbusmenuMenuitem;
    pub fn dbusmenu_menuitem_get_position(mi: *mut DbusmenuMenuitem, parent: *mut DbusmenuMenuitem) -> c_uint;
    pub fn dbusmenu_menuitem_get_position_realized(mi: *mut DbusmenuMenuitem, parent: *mut DbusmenuMenuitem) -> c_uint;
    pub fn dbusmenu_menuitem_get_root(mi: *mut DbusmenuMenuitem) -> gboolean;
    pub fn dbusmenu_menuitem_handle_event(mi: *mut DbusmenuMenuitem, name: *const c_char, variant: *mut glib::GVariant, timestamp: c_uint);
    pub fn dbusmenu_menuitem_properties_copy(mi: *mut DbusmenuMenuitem) -> *mut glib::GHashTable;
    pub fn dbusmenu_menuitem_properties_list(mi: *mut DbusmenuMenuitem) -> *mut glib::GList;
    pub fn dbusmenu_menuitem_property_exist(mi: *const DbusmenuMenuitem, property: *const c_char) -> gboolean;
    pub fn dbusmenu_menuitem_property_get(mi: *const DbusmenuMenuitem, property: *const c_char) -> *const c_char;
    pub fn dbusmenu_menuitem_property_get_bool(mi: *const DbusmenuMenuitem, property: *const c_char) -> gboolean;
    pub fn dbusmenu_menuitem_property_get_byte_array(mi: *const DbusmenuMenuitem, property: *const c_char, nelements: *mut size_t) -> *const u8;
    pub fn dbusmenu_menuitem_property_get_int(mi: *const DbusmenuMenuitem, property: *const c_char) -> c_int;
    pub fn dbusmenu_menuitem_property_get_variant(mi: *const DbusmenuMenuitem, property: *const c_char) -> *mut glib::GVariant;
    pub fn dbusmenu_menuitem_property_remove(mi: *mut DbusmenuMenuitem, property: *const c_char);
    pub fn dbusmenu_menuitem_property_set(mi: *mut DbusmenuMenuitem, property: *const c_char, value: *const c_char) -> gboolean;
    pub fn dbusmenu_menuitem_property_set_bool(mi: *mut DbusmenuMenuitem, property: *const c_char, value: gboolean) -> gboolean;
    pub fn dbusmenu_menuitem_property_set_byte_array(mi: *mut DbusmenuMenuitem, property: *const c_char, value: *const u8, nelements: size_t) -> gboolean;
    pub fn dbusmenu_menuitem_property_set_int(mi: *mut DbusmenuMenuitem, property: *const c_char, value: c_int) -> gboolean;
    pub fn dbusmenu_menuitem_property_set_variant(mi: *mut DbusmenuMenuitem, property: *const c_char, value: *mut glib::GVariant) -> gboolean;
    pub fn dbusmenu_menuitem_send_about_to_show(mi: *mut DbusmenuMenuitem, cb: *mut gpointer, cb_data: gpointer);
    pub fn dbusmenu_menuitem_set_parent(mi: *mut DbusmenuMenuitem, parent: *mut DbusmenuMenuitem) -> gboolean;
    pub fn dbusmenu_menuitem_set_root(mi: *mut DbusmenuMenuitem, root: gboolean);
    pub fn dbusmenu_menuitem_show_to_user(mi: *mut DbusmenuMenuitem, timestamp: c_uint);
    pub fn dbusmenu_menuitem_take_children(mi: *mut DbusmenuMenuitem) -> *mut glib::GList;
    pub fn dbusmenu_menuitem_unparent(mi: *mut DbusmenuMenuitem) -> gboolean;

    //=========================================================================
    // DbusmenuMenuitemProxy
    //=========================================================================
    pub fn dbusmenu_menuitem_proxy_get_type() -> GType;
    pub fn dbusmenu_menuitem_proxy_new(mi: *mut DbusmenuMenuitem) -> *mut DbusmenuMenuitemProxy;
    pub fn dbusmenu_menuitem_proxy_get_wrapped(pmi: *mut DbusmenuMenuitemProxy) -> *mut DbusmenuMenuitem;

    //=========================================================================
    // DbusmenuServer
    //=========================================================================
    pub fn dbusmenu_server_get_type() -> GType;
    pub fn dbusmenu_server_new(object: *const c_char) -> *mut DbusmenuServer;
    pub fn dbusmenu_server_get_icon_paths(server: *mut DbusmenuServer) -> glib::GStrv;
    pub fn dbusmenu_server_get_status(server: *mut DbusmenuServer) -> DbusmenuStatus;
    pub fn dbusmenu_server_get_text_direction(server: *mut DbusmenuServer) -> DbusmenuTextDirection;
    pub fn dbusmenu_server_set_icon_paths(server: *mut DbusmenuServer, icon_paths: glib::GStrv);
    pub fn dbusmenu_server_set_root(self_: *mut DbusmenuServer, root: *mut DbusmenuMenuitem);
    pub fn dbusmenu_server_set_status(server: *mut DbusmenuServer, status: DbusmenuStatus);
    pub fn dbusmenu_server_set_text_direction(server: *mut DbusmenuServer, dir: DbusmenuTextDirection);

}