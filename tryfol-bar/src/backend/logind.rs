#![allow(dead_code)]

use std::os::fd::RawFd;
use zbus::{proxy, zvariant::ObjectPath};

#[proxy(
    interface = "org.freedesktop.login1.Session",
    default_path = "/org/freedesktop/login1/session/auto",
    default_service = "org.freedesktop.login1"
)]
pub trait Session {
    fn terminate(&self) -> zbus::Result<()>;
    fn activate(&self) -> zbus::Result<()>;
    fn lock(&self) -> zbus::Result<()>;
    fn unlock(&self) -> zbus::Result<()>;
    fn set_idle_hint(&self, idle: bool) -> zbus::Result<()>;
    fn set_locked_hint(&self, locked: bool) -> zbus::Result<()>;
    fn kill(&self, who: &str, signal_number: i32) -> zbus::Result<()>;
    fn take_control(&self, force: bool) -> zbus::Result<()>;
    fn release_control(&self) -> zbus::Result<()>;
    fn set_type(&self, r#type: &str) -> zbus::Result<()>;
    fn set_class(&self, class: &str) -> zbus::Result<()>;
    fn set_display(&self, display: &str) -> zbus::Result<()>;
    fn set_tty(&self, tty_fd: RawFd) -> zbus::Result<()>;
    /// Returns (fd, inactive)
    fn take_device(&self, major: u32, minor: u32) -> zbus::Result<(RawFd, bool)>;
    fn release_device(&self, major: u32, minor: u32) -> zbus::Result<()>;
    fn pause_device_complete(&self, major: u32, minor: u32) -> zbus::Result<()>;
    fn set_brightness(&self, subsystem: &str, name: &str, brightness: u32) -> zbus::Result<()>;

    #[zbus(signal)]
    fn pause_device(major: u32, minor: u32, r#type: &str) -> zbus::Result<()>;
    #[zbus(signal)]
    fn resume_device(major: u32, minor: u32, fd: RawFd) -> zbus::Result<()>;
    #[zbus(signal)]
    fn lock() -> zbus::Result<()>;
    #[zbus(signal)]
    fn unlock() -> zbus::Result<()>;

    #[zbus(property(emits_changed_signal = "const"))]
    fn id(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "const"))]
    fn user(&self) -> zbus::Result<(u32, ObjectPath)>;
    #[zbus(property(emits_changed_signal = "const"))]
    fn name(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "const"))]
    fn timestamp(&self) -> zbus::Result<u64>;
    #[zbus(property(emits_changed_signal = "const"))]
    fn timestamp_monotonic(&self) -> zbus::Result<u64>;
    #[zbus(property(emits_changed_signal = "const"), name = "VTNr")]
    fn vt_number(&self) -> zbus::Result<u32>;
    #[zbus(property(emits_changed_signal = "const"))]
    fn seat(&self) -> zbus::Result<(String, ObjectPath)>;
    #[zbus(property, name = "TTY")]
    fn tty(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn display(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "const"))]
    fn remote(&self) -> zbus::Result<bool>;
    #[zbus(property(emits_changed_signal = "const"))]
    fn remote_host(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "const"))]
    fn remote_user(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "const"))]
    fn service(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "const"))]
    fn desktop(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "const"))]
    fn scope(&self) -> zbus::Result<String>;
    #[zbus(property(emits_changed_signal = "const"))]
    fn leader(&self) -> zbus::Result<u32>;
    #[zbus(property(emits_changed_signal = "const"))]
    fn audit(&self) -> zbus::Result<u32>;
    #[zbus(property, name = "Type")]
    fn type_(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn class(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn active(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn state(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn idle_hint(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn idle_since_hint(&self) -> zbus::Result<u64>;
    #[zbus(property)]
    fn idle_since_hint_monotonic(&self) -> zbus::Result<u64>;
    #[zbus(property)]
    fn locked_hint(&self) -> zbus::Result<bool>;
}
