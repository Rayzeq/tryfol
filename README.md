# Tryfol

A custom Wayland bar written in Rust + GTK4.

**Note**: This project is a work in progress (WIP).

## Modules to refactor

- Bluetooth
- Dbusmenu ([Potentially useful resource](https://codeberg.org/janetski/statusnotifier-systray-gtk4/src/branch/main))
  - Use custom widgets to reduce usage of `Rc<Mutex<...>>`
- Audio (PipeWire)
- Network (Netlink)
- "`things.rs`"

## TODO List (excluding refactors)

- Full Bluetooth control (to replace `blueman-manager`)
  - Investigate where connection notifications originate (likely KDE, but may be Blueman or something else)
  - Don't forget to display battery levels somewhere
- Full network control (to replace `nm-applet`)
- Bluetooth: deterministically sort items
- Workspaces: show icons of contained windows ?
- Restructure `tryfol` as a workspace (use Varlink for IPC ?)
  - `tryfol-...`: crates for modules in `backend/*` that need to be shared between other crates
  - `tryfol-bar`: the bar itself
  - `tryfol-daemon`
    - SNI watcher
    - Bluetooth notifications
    - Auto-inhibit idle when media is playing (see `wayland-pipewire-idle-inhibit`, which is what I currently use)
    - Sound + notifications for plugging/unplugging the power cable
    - [Battery notifications](https://kota.nz/battery_notifications_with_udev.html)
    - Playerctl replacement (so the "current player" is consistent across keybinds and the bar)
  - `tryfol-idle-inhibitor`: small executable to manually inhibit idling, used to prevent swayidle from triggering after resuming from hibernation (run before sleep and terminate on resume, though it might not solve the issue)
- Show USB devices battery if available (logitech mouse)
- Use a consistent method of displaying errors (either all with `Display` or all with `Debug`)

## Other known issues

- The Bluetooth module frequently triggers panics, though these are caught by GTK. This should be resolved after the module is refactored.
- Errors from the mpris module due to players disappearing. Those are not severe in any way, but I might refactor the mpris module to fix them at some point.
