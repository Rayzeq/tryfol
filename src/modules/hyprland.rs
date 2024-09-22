use crate::{
    backend::hyprland::{self, Workspace, WorkspaceId, WorkspaceInfos},
    HasTooltip, Japanese,
};
use futures::{pin_mut, StreamExt};
use gtk::{
    glib::{self, clone},
    pango::EllipsizeMode,
    prelude::*,
    Button, Label,
};
use gtk4 as gtk;
use lazy_static::lazy_static;
use log::error;
use regex::Regex;
use std::collections::HashMap;

const AUTOKILL: &[&str] = &["Update - Sublime Text", "Update - Sublime Merge"];
lazy_static! {
    // we unwrap here because Regex::new will only panic if the pattern is invalid
    static ref REWRITES: [(Regex, &'static str); 6] = [
        // this will apply to youtube inside firefox
        (Regex::new(r"(.*) - YouTube").unwrap(), "<span foreground=\"#ff0000\">󰗃</span>  $1"),
        (Regex::new(r"(.*) — Mozilla Firefox Private Browsing").unwrap(), "<span foreground=\"#b13dff\">󰈹</span>  $1"),
        (Regex::new(r"(.*) — Mozilla Firefox").unwrap(), "󰈹  $1"),
        // remove space between icons
        (Regex::new(r"(>|󰈹)  (<)").unwrap(), "$1 $2"),
        (Regex::new(r"(.*) - Sublime Text \(.*\)").unwrap(), "  $1"),
        (Regex::new(r"(.*) - Discord").unwrap(), "󰙯  $1"),
    ];
}

#[derive(Debug)]
pub struct Modules {
    pub workspaces: gtk::Box,
    pub window: Label,
}

#[derive(Debug)]
struct ModulesInner {
    pub workspaces: gtk::Box,
    pub window: Label,
    workspace_map: HashMap<WorkspaceId, Button>,
    active_workspace: Option<WorkspaceId>,
}

impl Modules {
    pub fn new() -> Self {
        let inner = ModulesInner::new();
        let (workspaces, window) = (inner.workspaces.clone(), inner.window.clone());

        glib::spawn_future_local(async move {
            if let Err(e) = inner.run().await {
                error!("Error in Hyprland module: {e:?}");
            }
        });

        Self { workspaces, window }
    }
}

impl ModulesInner {
    pub fn new() -> Self {
        let workspaces = gtk::Box::builder()
            .name("workspaces")
            .css_classes(["module"])
            .build();
        let window = Label::builder()
            .name("window")
            .css_classes(["module"])
            .single_line_mode(true)
            .ellipsize(EllipsizeMode::End)
            .visible(false)
            .build();

        Self {
            workspaces,
            window,
            workspace_map: HashMap::new(),
            active_workspace: None,
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        for workspace in hyprland::workspaces()
            .await?
            .into_iter()
            .filter(WorkspaceInfos::is_regular)
        {
            self.add_workspace(workspace.id);
        }

        let active_workspace = hyprland::active_workspace().await?;
        if active_workspace.is_regular() {
            self.workspace_map[&active_workspace.id].add_css_class("active");
            self.active_workspace = Some(active_workspace.id);
        }

        if let Some(active_window) = hyprland::active_window().await? {
            self.window.set_visible(true);
            let title = format_window(&active_window.class, &active_window.title);
            self.window.set_markup(&title);
            // if i want to see the not trunctated title
            self.window.set_better_tooltip_markup(Some(title));
        }

        let events = hyprland::events().await?;
        pin_mut!(events);
        while let Some(event) = events.next().await {
            match event {
                Ok(event) => self.handle_message(event).await,
                Err(e) => error!("Error while reading Hyprland event: {e:?}"),
            }
        }

        Ok(())
    }

    async fn handle_message(&mut self, event: hyprland::Event) {
        match event {
            hyprland::Event::CreateWorkspaceV2(Workspace::Regular { id, .. }) => {
                self.add_workspace(id);
            }
            hyprland::Event::DestroyWorkspaceV2(Workspace::Regular { id, .. }) => {
                if let Some(button) = self.workspace_map.remove(&id) {
                    self.workspaces.remove(&button);
                }
            }
            hyprland::Event::WorkspaceV2(Workspace::Regular { id, .. }) => {
                if let Some(old) = self
                    .active_workspace
                    .and_then(|id| self.workspace_map.get(&id))
                {
                    old.remove_css_class("active");
                }
                if let Some(new) = self.workspace_map.get(&id) {
                    new.add_css_class("active");
                }
                self.active_workspace = Some(id);
            }
            hyprland::Event::OpenWindow { address, title, .. } => {
                if AUTOKILL.contains(&title.as_str())
                // other sublime text windows have no title unfortunatly
                /*|| (class == "sublime_text" && title.is_empty())*/
                {
                    if let Err(e) = hyprland::close_window(address).await {
                        error!("Failed to close window: {e:?}");
                    }
                }
            }
            hyprland::Event::ActiveWindow { class, title } => {
                if title.is_empty() {
                    self.window.set_visible(false);
                } else {
                    self.window.set_visible(true);
                    let title = format_window(&class, &title);
                    self.window.set_markup(&title);
                    // if i want to see the not trunctated title
                    self.window.set_better_tooltip_markup(Some(title));
                }
            }
            _ => (),
        }
    }

    fn add_workspace(&mut self, id: WorkspaceId) {
        let button = Button::with_label(&id.to_japanese());
        button.connect_clicked(move |this| {
            glib::spawn_future_local(clone!(
                #[strong]
                this,
                async move {
                    // trying to switch to the current workspace is an error (depending
                    // on your config)
                    if this.has_css_class("active") {
                        return;
                    }

                    if let Err(e) = hyprland::change_workspace(id as WorkspaceId).await {
                        error!("Failed to change workspace: {e:?}");
                    }
                },
            ));
        });

        let previous = self
            .workspace_map
            .iter()
            .filter(|(i, _)| **i < id)
            .max_by_key(|(id, _)| **id)
            .map(|(_, button)| button);

        self.workspaces.insert_child_after(&button, previous);
        self.workspace_map.insert(id, button);
    }
}

fn format_window(class: &str, title: &str) -> String {
    let mut title = html_escape::encode_text(title).into_owned();

    for (pattern, replacement) in REWRITES.iter() {
        title = pattern.replace(&title, *replacement).into_owned();
    }

    if class == "kitty" {
        title = "  ".to_owned() + &title;
    }

    title
}
