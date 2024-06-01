use anyhow::{anyhow, Context};
use async_stream::stream;
use futures::Stream;
use std::{io, path::Path};
use tokio::net::UnixStream;

use super::{WindowAddress, Workspace};

#[derive(Clone, Debug)]
pub enum Event {
    Workspace {
        name: String,
    },
    WorkspaceV2(Workspace),
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
    CreateWorkspaceV2(Workspace),
    DestroyWorkspace,
    DestroyWorkspaceV2(Workspace),
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

impl Event {
    pub fn from(string: &str) -> anyhow::Result<Self> {
        let (name, arguments) = string.split_once(">>").context("Malformed event")?;

        Ok(match name {
            "workspace" => Self::Workspace {
                name: arguments.to_owned(),
            },
            "workspacev2" => Self::WorkspaceV2(
                Self::parse_full_workspace(arguments).context("While parsing `workspacev2`")?,
            ),
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
            "createworkspacev2" => Self::CreateWorkspaceV2(
                Self::parse_full_workspace(arguments)
                    .context("While parsing `createworkspacev2`")?,
            ),
            "destroyworkspace" => Self::DestroyWorkspace,
            "destroyworkspacev2" => Self::DestroyWorkspaceV2(
                Self::parse_full_workspace(arguments)
                    .context("While parsing `destroyworkspacev2`")?,
            ),
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

    fn parse_full_workspace(arguments: &str) -> anyhow::Result<Workspace> {
        let (id, name) = arguments.split_once(',').context("Malformed arguments")?;
        Workspace::from_raw(id, name.to_owned())
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

    pub fn events(self) -> impl Stream<Item = Result<Event, anyhow::Error>> {
        let mut data = Vec::new();

        stream! {
            loop {
                self.socket
                    .readable()
                    .await
                    .context("Error while waiting for readiness of Hyprland event socket")?;

                match self.socket.try_read_buf(&mut data) {
                    Ok(_) => {
                        let data_str = match std::str::from_utf8(&data) {
                            Ok(x) => x,
                            Err(e) => {
                                data.clear();
                                Err(e).context("Invalid utf8 received from Hyprland event socket")?
                            }
                        };
                        let (messages, end) = data_str.rsplit_once('\n').unwrap_or(("", data_str));
                        let end = end.to_owned();

                        if !messages.is_empty() {
                            for message in messages.split('\n') {
                                yield Event::from(message);
                            }
                        }

                        data.clear();
                        data.extend_from_slice(end.as_bytes());
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        continue;
                    }
                    Err(e) => {
                        data.clear();
                        Err(e).context("Error while reading from Hyprland event socket")?;
                    }
                }
            }
        }
    }
}
