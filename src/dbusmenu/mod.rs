use dbus::traits::{ClientExt, MenuitemExt};
use dbusmenu_glib as dbus;
use gtk::{
    gdk,
    gio::{Menu, MenuItem, SimpleAction, SimpleActionGroup, ThemedIcon},
    glib::clone,
    prelude::*,
    PopoverMenu, PopoverMenuFlags, Widget,
};
use gtk4 as gtk;

#[derive(Debug, Clone)]
pub struct DBusMenu {
    client: dbus::Client,
    popover: PopoverMenu,
    action_group: SimpleActionGroup,
}

enum Menuitem {
    Item(MenuItem),
    Separator,
}

impl DBusMenu {
    pub fn new(name: &str, path: &str) -> Self {
        Self::from_client(&dbus::Client::new(name, path))
    }

    pub fn from_client(client: &dbus::Client) -> Self {
        let action_group = SimpleActionGroup::new();

        let popover = PopoverMenu::builder()
            .has_arrow(false)
            .flags(PopoverMenuFlags::NESTED)
            .build();
        popover.insert_action_group("menu", Some(&action_group));

        let this = Self {
            client: client.clone(),
            popover,
            action_group,
        };

        client.connect_layout_updated(clone!(
            #[strong]
            this,
            move |client| {
                if let Some(root) = client.root() {
                    this.rebuild(&root);
                }
            }
        ));

        this
    }

    pub fn set_parent(&self, parent: Option<&impl IsA<Widget>>) {
        if let Some(parent) = parent {
            self.popover.set_parent(parent);
        } else {
            self.popover.unparent();
        }
    }

    pub fn popup(&self) {
        self.popover.popup();
    }

    fn rebuild(&self, root: &dbus::Menuitem) {
        self.popover.set_menu_model(Some(&self.build_menu(root)));
    }

    fn build_menu(&self, dbus_menu: &dbus::Menuitem) -> Menu {
        let menu = Menu::new();

        dbus_menu
            .properties_list()
            .iter()
            .filter(|prop| {
                !["children-display", "label", "icon-name", "enabled"].contains(&prop.as_str())
            })
            .for_each(|prop| {
                println!("Unknown {:?}, {:?}", prop, dbus_menu.property_get(prop));
            });

        let mut current_section = Menu::new();
        for child in dbus_menu.children() {
            match self.build_menuitem(&child) {
                Some(Menuitem::Item(item)) => current_section.append_item(&item),
                Some(Menuitem::Separator) => {
                    menu.append_section(None, &current_section);
                    current_section = Menu::new();
                }
                None => (),
            }
        }
        menu.append_section(None, &current_section);
        menu
    }

    fn build_menuitem(&self, dbus_item: &dbus::Menuitem) -> Option<Menuitem> {
        dbus_item
            .properties_list()
            .iter()
            .filter(|prop| {
                ![
                    "label",
                    "type",
                    "visible",
                    "icon-name",
                    "enabled",
                    "children-display",
                    // not implementd - I just don't care
                    "accessible-desc",
                ]
                .contains(&prop.as_str())
            })
            .for_each(|prop| {
                println!("Unknown {:?}, {:?}", prop, dbus_item.property_get(prop));
            });

        match dbus_item.property_get("type").as_deref() {
            Some("separator") => Some(Menuitem::Separator),
            Some(r#type) => {
                panic!("Unknown menu item type: {type}");
            }
            None => {
                let item = MenuItem::new(None, None);
                dbus_item
                    .property_get("label")
                    .map_or_else(|| todo!("uh?"), |label| item.set_label(Some(&label)));
                if let Some(icon_name) = dbus_item.property_get("icon-name") {
                    // TODO: cannot show icon AND text at the same time
                    // fuck gnome
                    // see: https://gitlab.gnome.org/GNOME/gtk/-/blob/main/gtk/gtkmodelbutton.c#L642
                    // https://gitlab.gnome.org/GNOME/gtk/-/blob/main/gtk/gtkmenusectionbox.c#L412
                    // if i do reimplement this shit to fix it I probably could also make
                    // so that submenus are 5px higher so I can make them great again
                    item.set_icon(&ThemedIcon::new(&icon_name));
                }

                let action = SimpleAction::new(&dbus_item.id().to_string(), None);
                action.connect_activate(clone!(
                    #[strong]
                    dbus_item,
                    move |_, _| {
                        dbus_item.handle_event("clicked", &0.into(), gdk::CURRENT_TIME);
                    }
                ));
                self.action_group.add_action(&action);
                item.set_action_and_target_value(Some(&format!("menu.{}", dbus_item.id())), None);

                if dbus_item.property_exist("visible") && !dbus_item.property_get_bool("visible") {
                    return None;
                }
                if dbus_item.property_exist("enabled") && !dbus_item.property_get_bool("enabled") {
                    action.set_enabled(false);
                }

                match dbus_item.property_get("children-display").as_deref() {
                    Some("submenu") => {
                        let submenu = self.build_menu(dbus_item);
                        item.set_submenu(Some(&submenu));
                    }
                    Some(r#type) => {
                        panic!("Unknown menu item chilren display: {type}");
                    }
                    None => (),
                }

                Some(Menuitem::Item(item))
            }
        }
    }
}
