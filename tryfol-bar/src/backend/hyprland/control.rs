use super::{WindowAddress, WindowInfos, WorkspaceId, WorkspaceInfos};
use anyhow::{Context, anyhow};
use std::{
    io::{self, ErrorKind, Write},
    os::unix::net::UnixStream as StdUnixStream,
    time::Duration,
};
use tokio::{io::AsyncReadExt, net::UnixStream, time::sleep};

pub async fn active_workspace() -> anyhow::Result<WorkspaceInfos> {
    let response = raw_request(&['j'], "activeworkspace").await?;
    serde_json::from_slice(&response).context("Invalid data while parsing active workspace infos")
}

pub async fn active_window() -> anyhow::Result<Option<WindowInfos>> {
    let response = raw_request(&['j'], "activewindow").await?;

    Ok(if response == b"{}" {
        None
    } else {
        Some(
            serde_json::from_slice(&response)
                .context("Invalid data while parsing active window infos")?,
        )
    })
}

pub async fn workspaces() -> anyhow::Result<Vec<WorkspaceInfos>> {
    let response = raw_request(&['j'], "workspaces").await?;
    serde_json::from_slice(&response).context("Invalid data while parsing workspaces infos")
}

#[doc(alias = "clients")]
pub async fn windows() -> anyhow::Result<Vec<WindowInfos>> {
    let response = raw_request(&['j'], "clients").await?;
    serde_json::from_slice(&response).context("Invalid data while parsing windows infos")
}

// TODO: there are a lot of other ways to change workspaces
pub async fn change_workspace(id: WorkspaceId) -> anyhow::Result<()> {
    dispatch(&("workspace ".to_owned() + &id.to_string())).await
}

// TODO: there are a lot of different selectors
pub async fn close_window(address: WindowAddress) -> anyhow::Result<()> {
    dispatch(&("closewindow address:".to_owned() + &address.to_string())).await
}

async fn dispatch(dispatcher: &str) -> anyhow::Result<()> {
    let command = "dispatch ".to_owned() + dispatcher;
    let response = raw_request(&[], &command).await?;

    if response == b"ok" {
        Ok(())
    } else {
        Err(anyhow!(
            "Dispatcher `{}` failed: {}",
            dispatcher,
            String::from_utf8_lossy(&response)
        ))
    }
}

async fn raw_request(flags: &[char], command: &str) -> anyhow::Result<Vec<u8>> {
    loop {
        match raw_request_noretry(flags, command).await {
            Ok(x) => return Ok(x),
            Err(e) => match e.downcast::<io::Error>() {
                // This indicate that the request was too slow and Hyprland closed the socket,
                // so we retry the request.
                // We wait 0 seconds to give back control to the async runtime between retries,
                // otherwise we might block it (if we continually fail).
                Ok(e) if e.kind() == ErrorKind::BrokenPipe => sleep(Duration::from_secs(0)).await,
                Ok(e) => return Err(e.into()),
                Err(e) => return Err(e),
            },
        }
    }
}

async fn raw_request_noretry(flags: &[char], command: &str) -> anyhow::Result<Vec<u8>> {
    let command = if flags.is_empty() {
        command.to_owned()
    } else {
        let flags: String = flags.iter().collect();
        format!("{flags}/{command}")
    };

    // The "open then write" operation must take less than 5ms to execute, otherwise Hyprland will close the socket.
    // This is why we use the sync api there, if there is await points, this will slow down things.
    let mut socket = StdUnixStream::connect(super::get_hyprland_path()?.join(".socket.sock"))
        .context("Cannot connect to Hyprland control socket")?;
    socket
        .write_all(command.as_bytes())
        .context("Cannot write to Hyprland control socket")?;
    socket.set_nonblocking(true);

    let mut socket =
        UnixStream::from_std(socket).context("Cannot convert std socket to tokio socket")?;
    let mut response = Vec::new();
    socket.read_to_end(&mut response).await?;

    Ok(response)
}
