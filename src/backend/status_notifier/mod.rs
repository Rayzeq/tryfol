// See: https://github.com/KDE/plasma-workspace/tree/1ae799372c3d5353151f3a0b76bb4f1410392865/applets/systemtray
//      https://github.com/elkowar/eww/tree/master/crates/notifier_host

mod host;
mod item;
mod proxy;
mod watcher;

pub use host::{run_host, Host};
pub use item::{Event, Item};
pub use proxy::{Category, Orientation, Status};
pub use watcher::Watcher;
