use self::hyprland::EventSocket;
use futures::{pin_mut, StreamExt};

pub mod hyprland;

pub async fn test() {
    let socket = EventSocket::new().await.unwrap();
    let events = socket.events();
    pin_mut!(events);
    while let Some(v) = events.next().await {
        println!("GOT = {:?}", v);
    }
}
