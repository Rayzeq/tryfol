use anyhow::{anyhow, Context};
use async_stream::stream;
use core::fmt::{self, Display};
use futures::Stream;
use std::path::Path;
use tokio::{io::AsyncReadExt, net::UnixStream};

#[derive(Clone, Copy, Debug)]
pub struct WorkspaceId(pub i32);

#[derive(Clone, Copy, Debug)]
pub struct WindowAddress(pub u32);

#[derive(Clone, Debug)]
pub enum Event {
    Workspace {
        name: String,
    },
    WorkspaceV2 {
        id: WorkspaceId,
        name: String,
    },
    FocusedMonitor,
    ActiveWindow {
        class: String,
        title: String,
    },
    ActiveWindowV2,
    Fullscreen,
    MonitorRemoved,
    MonitorAdded,
    MonitorAddedV2,
    CreateWorkspace,
    CreateWorkspaceV2 {
        id: WorkspaceId,
        name: String,
    },
    DestroyWorkspace,
    DestroyWorkspaceV2 {
        id: WorkspaceId,
        name: String,
    },
    MoveWorkspace,
    MoveWorkspaceV2,
    RenameWorkspace,
    ActiveSpecial,
    ActiveLayout,
    OpenWindow {
        address: WindowAddress,
        class: String,
        title: String,
        workspace_name: String,
    },
    CloseWindow,
    MoveWindow,
    MoveWindowV2,
    OpenLayer,
    CloseLayer,
    Submap,
    ChangeFloatingMode,
    Urgent,
    Minimize,
    Screencast,
    WindowTitle,
    ToggleGroup,
    MoveIntoGroup,
    MoveOutOfGroup,
    IgnoreGroupLock,
    LockGroups,
    ConfigReloaded,
    Pin,
}

pub struct EventSocket {
    socket: UnixStream,
}

impl WorkspaceId {
    pub fn from(id: &str) -> anyhow::Result<Self> {
        Ok(Self(id.parse().context("Invalid workspace id")?))
    }

    pub const fn is_special(self) -> bool {
        self.0 < 0
    }
}

impl WindowAddress {
    pub fn from(address: &str) -> anyhow::Result<Self> {
        Ok(Self(
            u32::from_str_radix(address, 16).context("Invalid workspace id")?,
        ))
    }
}

impl Display for WindowAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl Event {
    pub fn from(string: &str) -> anyhow::Result<Self> {
        let (name, arguments) = string.split_once(">>").context("Malformed event")?;

        Ok(match name {
            "workspace" => Self::Workspace {
                name: arguments.to_owned(),
            },
            "workspacev2" => {
                let (id, name) =
                    Self::parse_full_workspace(arguments).context("While parsing `workspacev2`")?;
                Self::WorkspaceV2 { id, name }
            }
            "focusedmon" => Self::FocusedMonitor,
            "activewindow" => {
                let (class, title) = arguments
                    .split_once(',')
                    .context("Malformed arguments")
                    .context("While parsing `activewindow`")?;
                Self::ActiveWindow {
                    class: class.to_owned(),
                    title: title.to_owned(),
                }
            }
            "activewindowv2" => Self::ActiveWindowV2,
            "fullscreen" => Self::Fullscreen,
            "monitorremoved" => Self::MonitorRemoved,
            "monitoradded" => Self::MonitorAdded,
            "monitoraddedv2" => Self::MonitorAddedV2,
            "createworkspace" => Self::CreateWorkspace,
            "createworkspacev2" => {
                let (id, name) = Self::parse_full_workspace(arguments)
                    .context("While parsing `createworkspacev2`")?;
                Self::CreateWorkspaceV2 { id, name }
            }
            "destroyworkspace" => Self::DestroyWorkspace,
            "destroyworkspacev2" => {
                let (id, name) = Self::parse_full_workspace(arguments)
                    .context("While parsing `destroyworkspacev2`")?;
                Self::DestroyWorkspaceV2 { id, name }
            }
            "moveworkspace" => Self::MoveWorkspace,
            "moveworkspacev2" => Self::MoveWorkspaceV2,
            "renameworkspace" => Self::RenameWorkspace,
            "activespecial" => Self::ActiveSpecial,
            "activelayout" => Self::ActiveLayout,
            "openwindow" => {
                let values: [&str; 4] = arguments
                    .splitn(4, ',')
                    .collect::<Vec<_>>()
                    .try_into()
                    .map_err(|_| {
                        anyhow!("Malformed arguments").context("While parsing `openwindow`")
                    })?;
                let (address, workspace_name, class, title) = values.into();
                Self::OpenWindow {
                    address: WindowAddress::from(address)?,
                    class: class.to_owned(),
                    title: title.to_owned(),
                    workspace_name: workspace_name.to_owned(),
                }
            }
            "closewindow" => Self::CloseWindow,
            "movewindow" => Self::MoveWindow,
            "movewindowv2" => Self::MoveWindowV2,
            "openlayer" => Self::OpenLayer,
            "closelayer" => Self::CloseLayer,
            "submap" => Self::Submap,
            "changefloatingmode" => Self::ChangeFloatingMode,
            "urgent" => Self::Urgent,
            "minimize" => Self::Minimize,
            "screencast" => Self::Screencast,
            "windowtitle" => Self::WindowTitle,
            "togglegroup" => Self::ToggleGroup,
            "moveintogroup" => Self::MoveIntoGroup,
            "moveoutofgroup" => Self::MoveOutOfGroup,
            "ignoregrouplock" => Self::IgnoreGroupLock,
            "lockgroups" => Self::LockGroups,
            "configreloaded" => Self::ConfigReloaded,
            "pin" => Self::Pin,
            _ => anyhow::bail!("Unknown event"),
        })
    }

    fn parse_full_workspace(arguments: &str) -> anyhow::Result<(WorkspaceId, String)> {
        let (id, name) = arguments.split_once(',').context("Malformed arguments")?;

        Ok((WorkspaceId::from(id)?, name.to_owned()))
    }
}

impl EventSocket {
    pub async fn new() -> Result<Self, anyhow::Error> {
        let path = Path::new(
            &std::env::var_os("XDG_RUNTIME_DIR")
                .context("Runtime directory is not set (missing $XDG_RUNTIME_DIR)")?,
        )
        .join("hypr")
        .join(
            std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
                .context("Can't find Hyprland directory (missing $HYPRLAND_INSTANCE_SIGNATURE)")?,
        )
        .join(".socket2.sock");
        let socket = UnixStream::connect(path)
            .await
            .context("Cannot connect to Hyprland socket")?;

        Ok(Self { socket })
    }

    pub fn events(mut self) -> impl Stream<Item = Result<Event, anyhow::Error>> {
        stream! {
            let mut data = String::new();

            loop {
                self.socket
                    .read_to_string(&mut data)
                    .await
                    .context("Error while reading from Hyprland event socket")?;
                for message in data.split('\n') {
                    yield Event::from(message);
                }

                data.clear();
            }
        }
    }
}
