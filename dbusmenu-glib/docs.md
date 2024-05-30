<!-- file * -->
<!-- static CLIENT_PROP_DBUS_NAME -->
String to access property [`dbus-name`][struct@crate::Client#dbus-name]
<!-- static CLIENT_PROP_DBUS_OBJECT -->
String to access property [`dbus-object`][struct@crate::Client#dbus-object]
<!-- static CLIENT_PROP_GROUP_EVENTS -->
String to access property [`group-events`][struct@crate::Client#group-events]
<!-- static CLIENT_PROP_STATUS -->
String to access property [`status`][struct@crate::Client#status]
<!-- static CLIENT_PROP_TEXT_DIRECTION -->
String to access property [`text-direction`][struct@crate::Client#text-direction]
<!-- static CLIENT_SIGNAL_EVENT_RESULT -->
String to attach to signal [`event-result`][struct@crate::Client#event-result]
<!-- static CLIENT_SIGNAL_ICON_THEME_DIRS_CHANGED -->
String to attach to signal [`icon-theme-dirs-changed`][struct@crate::Client#icon-theme-dirs-changed]
<!-- static CLIENT_SIGNAL_ITEM_ACTIVATE -->
String to attach to signal [`item-activate`][struct@crate::Client#item-activate]
<!-- static CLIENT_SIGNAL_LAYOUT_UPDATED -->
String to attach to signal [`layout-updated`][struct@crate::Client#layout-updated]
<!-- static CLIENT_SIGNAL_NEW_MENUITEM -->
String to attach to signal [`new-menuitem`][struct@crate::Client#new-menuitem]
<!-- static CLIENT_SIGNAL_ROOT_CHANGED -->
String to attach to signal [`root-changed`][struct@crate::Client#root-changed]
<!-- static CLIENT_TYPES_DEFAULT -->
Used to set the 'type' property on a menu item to create
a standard menu item.
<!-- static CLIENT_TYPES_IMAGE -->
Used to set the 'type' property on a menu item to create
an image menu item. Deprecated as standard menu items now
support images as well.
<!-- static CLIENT_TYPES_SEPARATOR -->
Used to set the 'type' property on a menu item to create
a separator menu item.
<!-- static MENUITEM_CHILD_DISPLAY_SUBMENU -->
Used in `DBUSMENU_MENUITEM_PROP_CHILD_DISPLAY` to have the
subitems displayed as a submenu.
<!-- static MENUITEM_DISPOSITION_ALERT -->
Used in `DBUSMENU_MENUITEM_PROP_DISPOSITION` to have a menu
item displayed in a way that conveys it's giving an alert
to the user.
<!-- static MENUITEM_DISPOSITION_INFORMATIVE -->
Used in `DBUSMENU_MENUITEM_PROP_DISPOSITION` to have a menu
item displayed in a way that conveys it's giving additional
information to the user.
<!-- static MENUITEM_DISPOSITION_NORMAL -->
Used in `DBUSMENU_MENUITEM_PROP_DISPOSITION` to have a menu
item displayed in the normal manner. Default value.
<!-- static MENUITEM_DISPOSITION_WARNING -->
Used in `DBUSMENU_MENUITEM_PROP_DISPOSITION` to have a menu
item displayed in a way that conveys it's giving a warning
to the user.
<!-- static MENUITEM_EVENT_ACTIVATED -->
String for the event identifier when a menu item is clicked
on by the user.
<!-- static MENUITEM_EVENT_CLOSED -->
String for the event identifier when a menu is closed and
displayed to the user. Only valid for items that contain
submenus.
<!-- static MENUITEM_EVENT_OPENED -->
String for the event identifier when a menu is opened and
displayed to the user. Only valid for items that contain
submenus.
<!-- static MENUITEM_ICON_NAME_BLANK -->
Used to set `DBUSMENU_MENUITEM_PROP_TOGGLE_STATE` so that the menu's
toggle item is undecided.
<!-- static MENUITEM_PROP_ACCESSIBLE_DESC -->
[`Menuitem`][crate::Menuitem] property used to provide a textual description of any
information that the icon may convey. The contents of this property are
passed through to assistive technologies such as the Orca screen reader.
The contents of this property will not be visible in the menu item. If
this property is set, Orca will use this property instead of the label
property.
<!-- static MENUITEM_PROP_CHILD_DISPLAY -->
[`Menuitem`][crate::Menuitem] property that tells how the children of this menuitem
should be displayed. Most likely this will be unset or of the value
`DBUSMENU_MENUITEM_CHILD_DISPLAY_SUBMENU`. Type: `G_VARIANT_TYPE_STRING`
<!-- static MENUITEM_PROP_DISPOSITION -->
[`Menuitem`][crate::Menuitem] property to tell what type of information that the
menu item is displaying to the user. Type: `G_VARIANT_TYPE_STRING`
<!-- static MENUITEM_PROP_ENABLED -->
[`Menuitem`][crate::Menuitem] property used to represent whether the menuitem
is clickable or not. Type: `G_VARIANT_TYPE_BOOLEAN`.
<!-- static MENUITEM_PROP_ICON_DATA -->
[`Menuitem`][crate::Menuitem] property that is the raw data of a custom icon
used in the application. Type: `G_VARIANT_TYPE_VARIANT`

It is recommended that this is not set directly but instead the
libdbusmenu-gtk library is used with the function `dbusmenu_menuitem_property_set_image()`
<!-- static MENUITEM_PROP_ICON_NAME -->
[`Menuitem`][crate::Menuitem] property that is the name of the icon under the
Freedesktop.org icon naming spec. Type: `G_VARIANT_TYPE_STRING`
<!-- static MENUITEM_PROP_LABEL -->
[`Menuitem`][crate::Menuitem] property used for the text on the menu item.
<!-- static MENUITEM_PROP_SHORTCUT -->
[`Menuitem`][crate::Menuitem] property that is the entries that represent a shortcut
to activate the menuitem. It is an array of arrays of strings.

It is recommended that this is not set directly but instead the
libdbusmenu-gtk library is used with the function `dbusmenu_menuitem_property_set_shortcut()`
<!-- static MENUITEM_PROP_TOGGLE_STATE -->
[`Menuitem`][crate::Menuitem] property that says what state a toggle entry should
be shown as the menu. Should be either `DBUSMENU_MENUITEM_TOGGLE_STATE_UNCHECKED`
`DBUSMENU_MENUITEM_TOGGLE_STATE_CHECKED` or `DBUSMENU_MENUITEM_TOGGLE_STATUE_UNKNOWN`.
<!-- static MENUITEM_PROP_TOGGLE_TYPE -->
[`Menuitem`][crate::Menuitem] property that says what type of toggle entry should
be shown in the menu. Should be either `DBUSMENU_MENUITEM_TOGGLE_CHECK`
or `DBUSMENU_MENUITEM_TOGGLE_RADIO`. Type: `G_VARIANT_TYPE_STRING`
<!-- static MENUITEM_PROP_TYPE -->
[`Menuitem`][crate::Menuitem] property used to represent what type of menuitem
this object represents. Type: `G_VARIANT_TYPE_STRING`.
<!-- static MENUITEM_PROP_VISIBLE -->
[`Menuitem`][crate::Menuitem] property used to represent whether the menuitem
should be shown or not. Type: `G_VARIANT_TYPE_BOOLEAN`.
<!-- static MENUITEM_SHORTCUT_ALT -->
Used in `DBUSMENU_MENUITEM_PROP_SHORTCUT` to represent the
alternate key.
<!-- static MENUITEM_SHORTCUT_CONTROL -->
Used in `DBUSMENU_MENUITEM_PROP_SHORTCUT` to represent the
control key.
<!-- static MENUITEM_SHORTCUT_SHIFT -->
Used in `DBUSMENU_MENUITEM_PROP_SHORTCUT` to represent the
shift key.
<!-- static MENUITEM_SHORTCUT_SUPER -->
Used in `DBUSMENU_MENUITEM_PROP_SHORTCUT` to represent the
super key.
<!-- static MENUITEM_SIGNAL_ABOUT_TO_SHOW -->
String to attach to signal [`about-to-show`][struct@crate::Server#about-to-show]
<!-- static MENUITEM_SIGNAL_CHILD_ADDED -->
String to attach to signal [`child-added`][struct@crate::Server#child-added]
<!-- static MENUITEM_SIGNAL_CHILD_MOVED -->
String to attach to signal [`child-moved`][struct@crate::Server#child-moved]
<!-- static MENUITEM_SIGNAL_CHILD_REMOVED -->
String to attach to signal [`child-removed`][struct@crate::Server#child-removed]
<!-- static MENUITEM_SIGNAL_EVENT -->
String to attach to signal [`event`][struct@crate::Server#event]
<!-- static MENUITEM_SIGNAL_ITEM_ACTIVATED -->
String to attach to signal [`item-activated`][struct@crate::Server#item-activated]
<!-- static MENUITEM_SIGNAL_PROPERTY_CHANGED -->
String to attach to signal [`property-changed`][struct@crate::Server#property-changed]
<!-- static MENUITEM_SIGNAL_REALIZED -->
String to attach to signal [`realized`][struct@crate::Server#realized]
<!-- static MENUITEM_SIGNAL_SHOW_TO_USER -->
String to attach to signal [`show-to-user`][struct@crate::Server#show-to-user]
<!-- static MENUITEM_TOGGLE_CHECK -->
Used to set `DBUSMENU_MENUITEM_PROP_TOGGLE_TYPE` to be a standard
check mark item.
<!-- static MENUITEM_TOGGLE_RADIO -->
Used to set `DBUSMENU_MENUITEM_PROP_TOGGLE_TYPE` to be a standard
radio item.
<!-- const MENUITEM_TOGGLE_STATE_CHECKED -->
Used to set `DBUSMENU_MENUITEM_PROP_TOGGLE_STATE` so that the menu's
toggle item is filled.
<!-- const MENUITEM_TOGGLE_STATE_UNCHECKED -->
Used to set `DBUSMENU_MENUITEM_PROP_TOGGLE_STATE` so that the menu's
toggle item is empty.
<!-- const MENUITEM_TOGGLE_STATE_UNKNOWN -->
Used to set `DBUSMENU_MENUITEM_PROP_TOGGLE_STATE` so that the menu's
toggle item is undecided.
<!-- static SERVER_PROP_DBUS_OBJECT -->
String to access property [`dbus-object`][struct@crate::Server#dbus-object]
<!-- static SERVER_PROP_ROOT_NODE -->
String to access property [`root-node`][struct@crate::Server#root-node]
<!-- static SERVER_PROP_STATUS -->
String to access property [`status`][struct@crate::Server#status]
<!-- static SERVER_PROP_TEXT_DIRECTION -->
String to access property [`text-direction`][struct@crate::Server#text-direction]
<!-- static SERVER_PROP_VERSION -->
String to access property [`version`][struct@crate::Server#version]
<!-- static SERVER_SIGNAL_ID_PROP_UPDATE -->
String to attach to signal [`item-property-updated`][struct@crate::Server#item-property-updated]
<!-- static SERVER_SIGNAL_ID_UPDATE -->
String to attach to signal [`item-updated`][struct@crate::Server#item-updated]
<!-- static SERVER_SIGNAL_ITEM_ACTIVATION -->
String to attach to signal [`item-activation-requested`][struct@crate::Server#item-activation-requested]
<!-- static SERVER_SIGNAL_LAYOUT_UPDATED -->
String to attach to signal [`layout-updated`][struct@crate::Server#layout-updated]
<!-- struct Client -->
The client for a [`Server`][crate::Server] creating a shared
    object set of [`Menuitem`][crate::Menuitem] objects.

## Properties


#### `dbus-name`
 Readable | Writeable | Construct Only


#### `dbus-object`
 Readable | Writeable | Construct Only


#### `group-events`
 Readable | Writeable

## Signals


#### `event-result`
 


#### `icon-theme-dirs-changed`
 Signaled when the theme directories are changed by the server.




#### `item-activate`
 Signaled when the server wants to activate an item in
        order to display the menu.




#### `layout-updated`
 


#### `new-menuitem`
 Signaled when the client creates a new menuitem. This
        doesn't mean that it's placed anywhere. The parent that
        it's applied to will signal [`child-added`][struct@crate::Menuitem#child-added]
        when it gets parented.




#### `root-changed`
 The layout has changed in a way that can not be
        represented by the individual items changing as the
        root of this client has changed.



# Implements

[`ClientExt`][trait@crate::prelude::ClientExt]
<!-- trait ClientExt::fn connect_icon_theme_dirs_changed -->
Signaled when the theme directories are changed by the server.
## `arg1`
A `GStrv` of theme directories
<!-- trait ClientExt::fn connect_item_activate -->
Signaled when the server wants to activate an item in
        order to display the menu.
## `arg1`
The [`Menuitem`][crate::Menuitem] activated
## `arg2`
A timestamp that the event happened at
<!-- trait ClientExt::fn connect_new_menuitem -->
Signaled when the client creates a new menuitem. This
        doesn't mean that it's placed anywhere. The parent that
        it's applied to will signal [`child-added`][struct@crate::Menuitem#child-added]
        when it gets parented.
## `arg1`
The new [`Menuitem`][crate::Menuitem] created
<!-- trait ClientExt::fn connect_root_changed -->
The layout has changed in a way that can not be
        represented by the individual items changing as the
        root of this client has changed.
## `arg1`
The new root [`Menuitem`][crate::Menuitem]
<!-- struct Menuitem -->
This is the `GObject` based object that represents a menu
item. It gets created the same on both the client and
the server side and libdbusmenu-glib does the work of making
this object model appear on both sides of DBus. Simple
really, though through updates and people coming on and off
the bus it can lead to lots of fun complex scenarios.

## Properties


#### `id`
 Readable | Writeable | Construct Only

## Signals


#### `about-to-show`
 Emitted when the submenu for this item
        is about to be shown




#### `child-added`
 Signaled when the child menuitem has been added to
        the parent.




#### `child-moved`
 Signaled when the child menuitem has had its location
        in the list change.




#### `child-removed`
 Signaled when the child menuitem has been requested to
        be removed from the parent. This signal is called when
        it has been removed from the list but not yet had
        `g_object_unref` called on it.




#### `event`
 Emitted when an event is passed through. The event is signalled
        after handle_event is called.

Detailed


#### `item-activated`
 Emitted on the objects on the server side when
        they are signaled on the client side.




#### `property-changed`
 Emitted everytime a property on a menuitem is either
        updated or added.




#### `realized`
 Emitted when the initial request for properties
        is complete on the item. If there is a type
        handler configured for the "type" parameter
        that will be executed before this is signaled.




#### `show-to-user`
 Signaled when the application would like the visualization
        of this menu item shown to the user. This usually requires
        going over the bus to get it done.



# Implements

[`MenuitemExt`][trait@crate::prelude::MenuitemExt]
<!-- trait MenuitemExt::fn foreach -->
This calls the function `func` on this menu item and all
of the children of this item. And their children. And
their children. And... you get the point. It will get
called on the whole tree.
## `func`
Function to call on every node in the tree
<!-- trait MenuitemExt::fn send_about_to_show -->
This function is used to send the even that the submenu
of this item is about to be shown. Callers to this event
should delay showing the menu until their callback is
called if possible.
## `cb`
Callback to call when the call has returned.
## `cb_data`
Data to pass to the callback.
<!-- trait MenuitemExt::fn connect_child_added -->
Signaled when the child menuitem has been added to
        the parent.
## `arg1`
The [`Menuitem`][crate::Menuitem] which is the child.
## `arg2`
The position that the child is being added in.
<!-- trait MenuitemExt::fn connect_child_moved -->
Signaled when the child menuitem has had its location
        in the list change.
## `arg1`
The [`Menuitem`][crate::Menuitem] which is the child.
## `arg2`
The position that the child is being moved to.
## `arg3`
The position that the child is was in.
<!-- trait MenuitemExt::fn connect_child_removed -->
Signaled when the child menuitem has been requested to
        be removed from the parent. This signal is called when
        it has been removed from the list but not yet had
        `g_object_unref` called on it.
## `arg1`
The [`Menuitem`][crate::Menuitem] which was the child.
<!-- struct MenuitemProxy -->
Public instance data for a [`MenuitemProxy`][crate::MenuitemProxy].

## Properties


#### `menu-item`
 Readable | Writeable | Construct Only
<details><summary><h4>Menuitem</h4></summary>


#### `id`
 Readable | Writeable | Construct Only
</details>

# Implements

[`MenuitemProxyExt`][trait@crate::prelude::MenuitemProxyExt], [`MenuitemExt`][trait@crate::prelude::MenuitemExt]
<!-- struct Server -->
A server which represents a sharing of a set of
    `DbusmenuMenuitems` across DBus to a [`Client`][crate::Client].

## Properties


#### `dbus-object`
 Readable | Writeable | Construct Only


#### `root-node`
 Readable | Writeable


#### `version`
 Readable

## Signals


#### `item-activation-requested`
 This is signaled when a menuitem under this server
        sends its activate signal.




#### `item-property-updated`
 


#### `item-updated`
 


#### `layout-updated`
 This signal is emitted any time the layout of the
        menuitems under this server is changed.



# Implements

[`ServerExt`][trait@crate::prelude::ServerExt]
