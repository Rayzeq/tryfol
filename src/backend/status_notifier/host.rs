use super::{
    proxy::{StatusNotifierItemRegistered, StatusNotifierItemUnregistered},
    Item, Watcher,
};
use log::error;
use ordered_stream::OrderedStreamExt;
use std::{collections::HashSet, convert::Infallible, process};
use zbus::{
    export::ordered_stream,
    fdo::{RequestNameFlags, RequestNameReply},
    names::WellKnownName,
    Connection,
};

pub trait Host {
    async fn item_registered(&mut self, id: &str, item: Item);
    async fn item_unregistered(&mut self, id: &str);
}

enum ItemEvent {
    RegisteredItem(StatusNotifierItemRegistered),
    UnregisteredItem(StatusNotifierItemUnregistered),
}

pub async fn run_host(connection: &Connection, host: &mut impl Host) -> zbus::Result<Infallible> {
    let watcher = Watcher::get_or_start(connection).await?;
    let connection = watcher.inner().connection();

    let pid = process::id();
    let mut i = 0;
    let wellknown = loop {
        let name: WellKnownName = format!("org.freedesktop.StatusNotifierHost-{pid}-{i}")
            .try_into()
            .unwrap();

        match connection
            .request_name_with_flags(&name, RequestNameFlags::DoNotQueue.into())
            .await?
        {
            RequestNameReply::PrimaryOwner => break name,
            RequestNameReply::Exists | RequestNameReply::AlreadyOwner => {}
            RequestNameReply::InQueue => unreachable!(),
        };
        i += 1;
    };

    watcher
        .register_status_notifier_host(wellknown.into())
        .await?;

    let registered_events = watcher.receive_status_notifier_item_registered().await?;
    let unregistered_events = watcher.receive_status_notifier_item_unregistered().await?;

    let mut items = HashSet::new();

    // Process initial items here. Since we already subscribed to events, we might have duplicates, but we can't
    // miss items (that's what I think at least)
    for item_id in watcher.registered_status_notifier_items().await? {
        match Item::from_id(connection, &item_id).await {
            Ok(item) => {
                host.item_registered(&item_id, item).await;
                items.insert(item_id);
            }
            Err(e) => error!("{item_id} is not a valid item id: {e}"),
        }
    }

    let mut events = ordered_stream::join(
        OrderedStreamExt::map(registered_events, ItemEvent::RegisteredItem),
        OrderedStreamExt::map(unregistered_events, ItemEvent::UnregisteredItem),
    );
    while let Some(event) = events.next().await {
        match event {
            ItemEvent::RegisteredItem(event) => {
                let item_id = event.args()?.service;
                if !items.contains(item_id) {
                    match Item::from_id(connection, item_id).await {
                        Ok(item) => {
                            items.insert(item_id.to_owned());
                            host.item_registered(item_id, item).await;
                        }
                        Err(e) => error!("{item_id} is not a valid item id: {e}"),
                    }
                }
            }
            ItemEvent::UnregisteredItem(event) => {
                let item_id = event.args()?.service;
                if items.remove(item_id) {
                    host.item_unregistered(item_id).await;
                }
            }
        }
    }

    // TODO: this can probably be reached if the watcher disapears
    unreachable!()
}
