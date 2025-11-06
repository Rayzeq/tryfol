use super::{WindowAddress, Workspace};
use crate::Split;
use anyhow::{Context, anyhow};
use async_stream::stream;
use futures::Stream;
use std::io;
use tokio::net::UnixStream;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum Event {
    Workspace {
        name: String,
    },
    WorkspaceV2(Workspace),
    FocusedMonitor,
    FocusedMonitorV2,
    ActiveWindow {
        class: String,
        title: String,
    },
    ActiveWindowV2,
    Fullscreen,
    MonitorRemoved,
    MonitorRemovedV2,
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
    ActiveSpecialV2,
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
    WindowTitleV2,
    ToggleGroup,
    MoveIntoGroup,
    MoveOutOfGroup,
    IgnoreGroupLock,
    LockGroups,
    ConfigReloaded,
    Pin,
    Minimized,
    Bell,
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
            "focusedmonv2" => Self::FocusedMonitorV2,
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
            "monitorremovedv2" => Self::MonitorRemovedV2,
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
            "activespecialv2" => Self::ActiveSpecialV2,
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
            "windowtitlev2" => Self::WindowTitleV2,
            "togglegroup" => Self::ToggleGroup,
            "moveintogroup" => Self::MoveIntoGroup,
            "moveoutofgroup" => Self::MoveOutOfGroup,
            "ignoregrouplock" => Self::IgnoreGroupLock,
            "lockgroups" => Self::LockGroups,
            "configreloaded" => Self::ConfigReloaded,
            "pin" => Self::Pin,
            "minimized" => Self::Minimized,
            "bell" => Self::Bell,
            name => anyhow::bail!("Unknown event: {name}"),
        })
    }

    fn parse_full_workspace(arguments: &str) -> anyhow::Result<Workspace> {
        let (id, name) = arguments.split_once(',').context("Malformed arguments")?;
        Workspace::from_raw(id, name.to_owned())
    }
}

pub async fn events() -> anyhow::Result<impl Stream<Item = anyhow::Result<Event>>> {
    let socket = UnixStream::connect(super::get_hyprland_path()?.join(".socket2.sock"))
        .await
        .context("Cannot connect to Hyprland event socket")?;

    let mut data = Vec::new();

    Ok(stream! {
        loop {
            socket
                .readable()
                .await
                .context("Error while waiting for readiness of Hyprland event socket")?;

            match socket.try_read_buf(&mut data) {
                Ok(_) => {
                    let (messages, end) = data.rsplit_once(b'\n').unwrap_or((b"", &data));
                    let end = end.to_owned();

                    if !messages.is_empty() {
                        for message in messages.split(|x| *x == b'\n') {
                            yield Event::from(String::from_utf8_lossy(message).as_ref());
                        }
                    }

                    data = end;
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
    })
}
