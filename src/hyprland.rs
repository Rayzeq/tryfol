use futures::{pin_mut, StreamExt};
use gtk::{
    gio::{SocketClient, UnixSocketAddress},
    glib::{self, clone, Bytes, Priority},
    pango::EllipsizeMode,
    prelude::*,
    Button, Label,
};
use gtk4 as gtk;
use lazy_static::lazy_static;
use serde::{de::DeserializeOwned, Deserialize};
use std::path::Path;

use crate::backend::hyprland::{self, Workspace as WorkspaceV2};

#[derive(Debug)]
pub struct Modules {
    pub workspaces: gtk::Box,
    pub window: Label,
}

pub fn new() -> Modules {
    let workspaces = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    // TODO: https://discourse.gnome.org/t/how-to-make-a-marquee-gtklabel/11088
    let window = Label::new(None);

    workspaces.set_widget_name("workspaces");
    window.set_widget_name("window");
    workspaces.add_css_class("module");
    window.add_css_class("module");

    window.set_single_line_mode(true);
    window.set_ellipsize(EllipsizeMode::End);

    glib::spawn_future_local(clone!(@strong workspaces, @strong window => async move {
        let mut workspace_map = Vec::new();
        let events = hyprland::EventSocket::new().await.unwrap().events();
        pin_mut!(events);

        for workspace in hyprctl::<Vec<Workspace>>("workspaces").await {
            add_workspace(workspace.id, &mut workspace_map, &workspaces);
        }

        let active_id: Option<u32> = hyprctl::<Workspace>("activeworkspace").await.id.try_into().ok();
        if let Some((_, old)) =
                active_id.and_then(|active_id| workspace_map.iter().find(|(id, _)| *id == active_id))
            {
                old.add_css_class("active");
            }
        let mut active_workspace = active_id;

        let active_window = hyprctl::<Window>("activewindow").await;
        if !active_window.title.is_empty() {
            window.set_visible(true);
            window.set_markup(&format_window(&active_window.class, &active_window.title));
        }

        while let Some(event) = events.next().await {
            handle_message(event.unwrap(), &mut active_workspace, &mut workspace_map, &workspaces, &window).await;
        }
    }));

    Modules { workspaces, window }
}

const AUTOKILL: &[&str] = &["Update - Sublime Text", "Update - Sublime Merge"];

async fn handle_message(
    event: hyprland::Event,
    active_workspace: &mut Option<u32>,
    workspace_map: &mut Vec<(u32, Button)>,
    workspaces: &gtk::Box,
    window: &Label,
) {
    match event {
        hyprland::Event::WorkspaceV2(WorkspaceV2::Regular { id, .. }) => {
            if let Some((_, old)) =
                active_workspace.and_then(|id| workspace_map.iter().find(|(i, _)| *i == id))
            {
                old.remove_css_class("active");
            }
            if let Some((_, new)) = workspace_map.iter().find(|(i, _)| *i == id) {
                new.add_css_class("active");
            }
            *active_workspace = Some(id);
        }
        hyprland::Event::ActiveWindow { class, title } => {
            if title.is_empty() {
                window.set_visible(false);
            } else {
                window.set_visible(true);
                window.set_markup(&format_window(&class, &title));
            }
        }
        hyprland::Event::CreateWorkspaceV2(WorkspaceV2::Regular { id, .. }) => {
            add_workspace(id, workspace_map, workspaces);
        }
        hyprland::Event::DestroyWorkspaceV2(WorkspaceV2::Regular { id, .. }) => {
            if let Some((_, button)) = workspace_map
                .iter()
                .position(|(i, _)| *i == id)
                .map(|index| workspace_map.remove(index))
            {
                workspaces.remove(&button);
            }
        }
        hyprland::Event::OpenWindow { address, title, .. } => {
            if AUTOKILL.contains(&title.as_str())
            /*|| (class == "sublime_text" && title.is_empty())*/
            {
                hyprctl_dispatch(&["closewindow", &format!("address:{address}")]).await;
            }
        }
        _ => (),
    }
}

fn add_workspace(id: u32, workspace_map: &mut Vec<(u32, Button)>, workspaces: &gtk::Box) {
    let button = Button::with_label(&id.to_japanese());
    button.connect_clicked(move |_| {
        glib::spawn_future_local(async move {
            hyprctl_dispatch(&["workspace", &id.to_string()]).await;
        });
    });

    let next_index = workspace_map
        .iter()
        .position(|(i, _)| *i > id)
        .unwrap_or(workspace_map.len());

    if next_index == 0 {
        workspaces.prepend(&button);
        workspace_map.insert(0, (id, button));
    } else {
        workspaces.insert_child_after(&button, Some(&workspace_map[next_index - 1].1));
        workspace_map.insert(next_index, (id, button));
    }
}

lazy_static! {
    static ref REWRITES: [(regex::Regex, &'static str); 6] = [
        // this will apply to youtube inside firefox
        (regex::Regex::new(r"(.*) - YouTube").unwrap(), "󰗃  $1"),
        (regex::Regex::new(r"(.*) — Mozilla Firefox Private Browsing").unwrap(), "<span foreground=\"#b13dff\">󰈹</span>  $1"),
        (regex::Regex::new(r"(.*) — Mozilla Firefox").unwrap(), "󰈹  $1"),
        // remove space between icons
        (regex::Regex::new(r"(󰈹|>)  (󰗃)").unwrap(), "$1 $2"),
        (regex::Regex::new(r"(.*) - Sublime Text \(.*\)").unwrap(), "  $1"),
        (regex::Regex::new(r"(.*) - Discord").unwrap(), "󰙯  $1"),
    ];
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

#[allow(non_snake_case, dead_code)]
#[derive(Deserialize, Debug)]
struct Workspace {
    pub id: u32,
    pub name: String,
    pub monitor: String,
    pub monitorID: usize,
    pub windows: usize,
    pub hasfullscreen: bool,
    pub lastwindow: String,
    pub lastwindowtitle: String,
}

#[allow(non_snake_case, dead_code)]
#[derive(Deserialize, Debug)]
struct WorkspaceShort {
    pub id: u32,
    pub name: String,
}

#[allow(non_snake_case, dead_code, clippy::struct_excessive_bools)]
#[derive(Deserialize, Debug)]
struct Window {
    pub address: String,
    pub mapped: bool,
    pub hidden: bool,
    pub at: (usize, usize),
    pub size: (usize, usize),
    pub workspace: WorkspaceShort,
    pub floating: bool,
    pub monitor: usize,
    pub class: String,
    pub title: String,
    pub initialClass: String,
    pub initialTitle: String,
    pub pid: usize,
    pub xwayland: bool,
    pub pinned: bool,
    pub fullscreen: bool,
    pub fullscreenMode: usize,
    pub fakeFullscreen: bool,
    pub grouped: Vec<()>,
    pub swallowing: String,
    pub focusHistoryID: usize,
}

async fn hyprctl_dispatch(command: &[&str]) {
    let socket = SocketClient::new()
        .connect_future(&UnixSocketAddress::new(
            &Path::new(&std::env::var("XDG_RUNTIME_DIR").expect("Missing $XDG_RUNTIME_DIR"))
                .join("hypr")
                .join(
                    std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
                        .expect("Can't find hyprland socket"),
                )
                .join(".socket.sock"),
        ))
        .await
        .expect("can't connect to hyprland socket");

    let full_command = "dispatch ".to_owned() + &command.join(" ");
    socket
        .output_stream()
        .write_bytes_future(&Bytes::from_owned(full_command), Priority::DEFAULT)
        .await
        .expect("cannot write to hyprland socket");

    let data = socket
        .input_stream()
        .read_bytes_future(32768, Priority::DEFAULT)
        .await
        .expect("cannot read from hyprland socket");
    let data = std::str::from_utf8(&data).expect("invalid utf8 received from hyprland");
    if data != "ok" {
        println!("error in `dispatch {}`: {data}", command.join(" "));
    }
}

async fn hyprctl<T: DeserializeOwned>(command: &str) -> T {
    let socket = SocketClient::new()
        .connect_future(&UnixSocketAddress::new(
            &Path::new(&std::env::var("XDG_RUNTIME_DIR").expect("Missing $XDG_RUNTIME_DIR"))
                .join("hypr")
                .join(
                    std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
                        .expect("Can't find hyprland socket"),
                )
                .join(".socket.sock"),
        ))
        .await
        .expect("can't connect to hyprland socket");

    let command = "j/".to_owned() + command;
    socket
        .output_stream()
        .write_bytes_future(&Bytes::from_owned(command), Priority::DEFAULT)
        .await
        .expect("cannot write to hyprland socket");

    let data = socket
        .input_stream()
        .read_bytes_future(32768, Priority::DEFAULT)
        .await
        .expect("cannot read from hyprland socket");

    serde_json::from_slice(&data).expect("hyprland sent invalid data over the socket")
}

trait NumberExt {
    fn first_digit(self) -> (Self, u32)
    where
        Self: Sized;
    fn as_japanese_direct(&self) -> Option<&'static str>;
    fn to_japanese(&self) -> String;
}

impl NumberExt for u32 {
    fn first_digit(mut self) -> (Self, u32) {
        let mut i = 0;
        while self >= 10 {
            self /= 10;
            i += 1;
        }
        (self, i)
    }

    fn as_japanese_direct(&self) -> Option<&'static str> {
        Some(match self {
            0 => "〇",
            1 => "一",
            2 => "二",
            3 => "三",
            4 => "四",
            5 => "五",
            6 => "六",
            7 => "七",
            8 => "八",
            9 => "九",
            10 => "十",
            100 => "百",
            1_000 => "千",
            10_000 => "万",
            100_000_000 => "億",
            _ => return None,
        })
    }

    fn to_japanese(&self) -> String {
        if let Some(x) = self.as_japanese_direct() {
            return x.to_owned();
        }

        let (base, digit_count) = self.first_digit();
        let multiplier = 10u32.pow(digit_count);
        let remaining = self - base * multiplier;

        format!(
            "{}{}{}",
            if base == 1 {
                ""
            } else {
                unsafe { base.as_japanese_direct().unwrap_unchecked() }
            },
            // TODO: this will panic for some numbers (e.g 100_000)
            multiplier.as_japanese_direct().unwrap(),
            if remaining == 0 {
                String::new()
            } else {
                remaining.to_japanese()
            }
        )
    }
}
