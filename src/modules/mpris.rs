use crate::{
    backend::mpris::{Mpris, PlaybackStatus, Player},
    Hoverable,
};
use anyhow::Context;
use async_recursion::async_recursion;
use gtk::{
    glib::{self, clone},
    pango::EllipsizeMode,
    prelude::*,
    Button, Label, Revealer, RevealerTransitionType,
};
use gtk4 as gtk;
use log::error;
use std::{collections::HashMap, rc::Rc};
use tokio::sync::Mutex;

pub fn new() -> gtk::Box {
    let label = Label::builder()
        .single_line_mode(true)
        .ellipsize(EllipsizeMode::End)
        .build();
    let play_pause = Button::builder().child(&label).build();

    let previous = Button::builder().css_classes(["left"]).label("󰒫").build();
    let next = Button::builder().css_classes(["right"]).label("󰒬").build();
    let left_revealer = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideLeft)
        .transition_duration(500)
        .child(&previous)
        .build();
    let right_revealer = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideRight)
        .transition_duration(500)
        .child(&next)
        .build();

    let root = gtk::Box::builder()
        .name("mpris")
        .css_classes(["module"])
        .visible(false)
        .build();
    root.append(&left_revealer);
    root.append(&play_pause);
    root.append(&right_revealer);

    root.connect_hover_notify(move |_, hovered| {
        left_revealer.set_reveal_child(hovered);
        right_revealer.set_reveal_child(hovered);
    });

    let current_player: Rc<Mutex<Option<Player>>> = Rc::new(Mutex::new(None));

    // Note: because of the strong clones here, current_player won't be dropped unless
    // the signal are disconnected (or the widget is dropped ?)
    play_pause.connect_clicked(clone!(@strong current_player => move |_| {
        glib::spawn_future_local(clone!(@strong current_player => async move {
            if let Some(player) = &*current_player.lock().await {
                if let Err(e) = player.toggle().await {
                    error!("Cannot play/pause Mpris player: {e:?}");
                }
            }
        }));
    }));
    previous.connect_clicked(clone!(@strong current_player => move |_| {
        glib::spawn_future_local(clone!(@strong current_player => async move {
            if let Some(player) = &*current_player.lock().await {
                if let Err(e) = player.previous().await {
                    error!("Cannot skip Mpris player backward: {e:?}");
                }
            }
        }));
    }));
    next.connect_clicked(clone!(@strong current_player => move |_| {
        glib::spawn_future_local(clone!(@strong current_player => async move {
            if let Some(player) = &*current_player.lock().await {
                if let Err(e) = player.next().await {
                    error!("Cannot skip Mpris player forward: {e:?}");
                }
            }
        }));
    }));

    glib::spawn_future_local(clone!(@weak root => async move {
        if let Err(e) = listen(root, label, current_player).await {
            error!("Cannot listen for Mpris players: {e:?}");
        }
    }));

    root
}

async fn listen(
    root: gtk::Box,
    label: Label,
    current_player: Rc<Mutex<Option<Player>>>,
) -> anyhow::Result<()> {
    let mpris = Mpris::new()
        .await
        .context("While creating Mpris connection")?;

    let players = Rc::new(Mutex::new(HashMap::new()));

    for player in mpris.players().await.context("While getting players")? {
        manage_player(
            &player,
            &players,
            current_player.clone(),
            root.clone(),
            label.clone(),
        );
        players
            .lock()
            .await
            .insert(player.app_name().to_owned(), player);
    }

    // TODO: a player might disappear and cause an error here
    if let Some(player) = find_suitable_player(&mut *players.lock().await)
        .await
        .context("While finding suitable player")?
    {
        root.set_visible(true);
        current_player.lock().await.replace(player.clone());
        update_label(&player, &label)
            .await
            .context("While updating label")?;
    }

    mpris.connect_players_changed(
        clone!(@weak root, @weak label, @strong players, @strong current_player => move |player| {
            glib::spawn_future_local(
                clone!(@weak root, @weak label, @strong players, @strong current_player => async move {
                    handle_new_player(player, players, current_player, root, label).await;
                }),
            );
        }),
        move |player| {
            glib::spawn_future_local(clone!(@weak root, @weak label, @strong players, @strong current_player => async move {
                if let Err(e) = handle_removed_player(player, &mut *players.lock().await, current_player, &root, &label).await {
                    error!("Cannot handle removed Mpris player: {e:?}");
                }
            }));
        },
    );
    // leak the message bus, so it's not dropped and the signals can be received
    // TODO: do we still need this ?
    std::mem::forget(mpris);

    Ok(())
}

async fn handle_new_player(
    player: Player,
    players: Rc<Mutex<HashMap<String, Player>>>,
    current_player: Rc<Mutex<Option<Player>>>,
    root: gtk::Box,
    label: Label,
) {
    manage_player(&player, &players, current_player, root, label);
    players
        .lock()
        .await
        .insert(player.app_name().to_owned(), player);
}

#[async_recursion(?Send)]
async fn handle_removed_player(
    player: Player,
    players: &mut HashMap<String, Player>,
    current_player: Rc<Mutex<Option<Player>>>,
    root: &gtk::Box,
    label: &Label,
) -> anyhow::Result<()> {
    players.remove(&player.app_name().to_owned());

    if current_player
        .lock()
        .await
        .as_ref()
        .is_some_and(|current| *current == player)
    {
        if let Some(player) = find_suitable_player(players)
            .await
            .context("While finding suitable player")?
        {
            let result = update_label(&player, label).await;
            match result {
                Ok(()) => (),
                Err(zbus::fdo::Error::ServiceUnknown(_)) => {
                    return handle_removed_player(player, players, current_player, root, label)
                        .await;
                }
                Err(e) => {
                    error!("Cannot handle Mpris player update: {e:?}");
                }
            }
            root.set_visible(true);
            current_player.lock().await.replace(player);
        } else {
            root.set_visible(false);
            current_player.lock().await.take();
        };
    }

    Ok(())
}

async fn find_suitable_player(
    players: &mut HashMap<String, Player>,
) -> zbus::fdo::Result<Option<Player>> {
    let mut zombie_players = Vec::new();

    for (name, player) in players.iter_mut() {
        match player.playback_status().await {
            Ok(PlaybackStatus::Playing) => return Ok(Some(player.clone())),
            Ok(_) => (),
            Err(zbus::fdo::Error::ServiceUnknown(_)) => zombie_players.push(name.clone()),
            Err(e) => return Err(e),
        }
    }

    for name in zombie_players {
        players.remove(&name);
    }

    Ok(players.values().next().cloned())
}

fn manage_player(
    player: &Player,
    players: &Rc<Mutex<HashMap<String, Player>>>,
    current_player: Rc<Mutex<Option<Player>>>,
    root: gtk::Box,
    label: Label,
) {
    player
        .connect_on_properties_changed(clone!(@strong player, @strong players => move |_, _, _| {
            glib::spawn_future_local(clone!(@strong player, @strong players, @strong current_player, @weak root, @weak label => async move {
                let result = handle_player_update(player.clone(), &mut *current_player.lock().await, &root, &label).await;
                match result {
                    Ok(()) => (),
                    Err(zbus::fdo::Error::ServiceUnknown(_)) => {
                        if let Err(e) = handle_removed_player(player, &mut *players.lock().await, current_player, &root, &label).await {
                            error!("Cannot find new Mpris player: {e:?}");
                        }
                    }
                    Err(e) => {
                        error!("Cannot handle Mpris player update: {e:?}");
                    }
                }
            }));
        }));
}

async fn handle_player_update(
    player: Player,
    current_player: &mut Option<Player>,
    root: &gtk::Box,
    label: &Label,
) -> zbus::fdo::Result<()> {
    if current_player
        .as_ref()
        .is_some_and(|current| *current == player)
    {
        update_label(&player, label).await?;
        return Ok(());
    }

    let status = player.playback_status().await?;
    if status == PlaybackStatus::Playing {
        update_label(&player, label).await?;
        current_player.replace(player);
        root.set_visible(true);
    }

    Ok(())
}

async fn update_label(player: &Player, label: &Label) -> zbus::fdo::Result<()> {
    let status = player.playback_status().await?;
    let mut text = if status == PlaybackStatus::Playing {
        "󰏤  "
    } else {
        "󰐊  "
    }
    .to_owned();

    text += &player.title().await?.as_deref().unwrap_or("Unknown");
    let artist = player.artists().await?;
    if !(artist.is_empty() || artist.iter().all(String::is_empty)) {
        text += " - ";
        text += &artist.join(", ");
    }
    label.set_text(&text);

    Ok(())
}
