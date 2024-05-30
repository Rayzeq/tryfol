//! # bindings for glib part of libdbusmenu
//!
//! Rust bindings for the glib part of [libdbusmenu] that work with the [gtk-rs ecosystem].
//!
//! By using [`Server`], you can use this crate in desktop applications to expose a menu over DBus.
//! For more information, including code examples, see [libdbusmenu].
//!
//! This crate also provides a UI-framework-independent interface to read them by using [`Client`].
//! However, if you are using GTK, it is recommended that you use `dbusmenu-gtk3`, which handles most of the GTK glue required to show it.
//!
//! [libdbusmenu]: https://github.com/AyatanaIndicators/libdbusmenu
//! [gtk-rs ecosystem]: https://gtk-rs.org
#[allow(unused_macros)]
#[doc(hidden)]
macro_rules! skip_assert_initialized {
    () => {};
}

#[allow(unused_macros)]
#[doc(hidden)]
macro_rules! assert_initialized_main_thread {
    () => {
      
    };
}

mod auto;
pub use auto::*;
pub use ffi;
