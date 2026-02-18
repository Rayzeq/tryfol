# Tryfol

A custom Wayland bar written in Rust + GTK4.

**Note**: This project is a work in progress (WIP).

## Modules to refactor

- Bluetooth
- Dbusmenu ([Potentially useful resource](https://codeberg.org/janetski/statusnotifier-systray-gtk4/src/branch/main))
  - Use custom widgets to reduce usage of `Rc<Mutex<...>>`
  - Test with: Discord, blueman, heroic, nm-applet
- Audio (PipeWire)
- Network (Netlink)
- "`things.rs`"

## TODO List (excluding refactors)

- Full Bluetooth control (to replace `blueman-manager`)
  - Investigate where connection notifications originate (likely KDE, but may be Blueman or something else), and implement them.
  - Don't forget to display battery levels somewhere (change icon color depending on battery level, and show precise level in tooltip)
  - If possible, enable bluetooth when trying to connect device (is it possible to list unconnected devices when bluetooth is disabled ?) and disable bluetooth when last device is disconnected
- Full network control (to replace `nm-applet`)
- Bluetooth: deterministically sort items
- Workspaces: show icons of contained windows ? (i.e one small dot per window in the workspace)
- Restructure `tryfol` as a workspace => custom ipc
  - `tryfol-...`: crates for modules in `backend/*` that need to be shared between other crates
  - `tryfol-bar`: the bar itself
  - `tryfol-daemon`
    - SNI watcher
    - Bluetooth notifications
    - Auto-inhibit idle when media is playing (see `wayland-pipewire-idle-inhibit`, which is what I currently use)
    - Sound + notifications for plugging/unplugging the power cable
    - [Battery notifications](https://kota.nz/battery_notifications_with_udev.html)
    - Playerctl replacement (so the "current player" is consistent across keybinds and the bar)
- Show USB devices battery if available (logitech mouse)
- Use a consistent method of displaying errors (either all with `Display` or all with `Debug`)
- Network: add ping with router and with internet
- Network: if ping with the routeur exceed a thresold, disconnect and reconnect (in Daemon)
- Cpu: make orange when 1 core is at 100%

## Other known issues

- The Bluetooth module frequently triggers panics, though these are caught by GTK. This should be resolved after the module is refactored.
- Errors from the mpris module due to players disappearing. Those are not severe in any way, but I might refactor the mpris module to fix them at some point.
