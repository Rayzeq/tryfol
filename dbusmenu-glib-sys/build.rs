// This file was generated by gir (https://github.com/gtk-rs/gir)
// from /nix/store/4dxx74s4g3rrn6haryx8i6yzy91f5q7m-source
// from /nix/store/687zj3l24wawn3a93nkqqcv6g0hjm9n5-dbusmenu-gtk3-gir
// DO NOT EDIT

#[cfg(not(feature = "dox"))]
use std::process;

#[cfg(feature = "dox")]
fn main() {} // prevent linking libraries to avoid documentation failure

#[cfg(not(feature = "dox"))]
fn main() {
    if let Err(s) = system_deps::Config::new().probe() {
        println!("cargo:warning={s}");
        process::exit(1);
    }
}