use crate::{
    backend::mpris::{Mpris, PlaybackStatus, Player},
    Hoverable,
};
use anyhow::Context;
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

    let mut players = HashMap::new();
    for player in mpris.players().await.context("While getting players")? {
        manage_player(&player, current_player.clone(), root.clone(), label.clone());
        players.insert(player.app_name().to_owned(), player);
    }

    if let Some(player) = find_suitable_player(&players)
        .await
        .context("While finding suitable player")?
    {
        root.set_visible(true);
        current_player.lock().await.replace(player.clone());
        update_label(&player, &label)
            .await
            .context("While updating label")?;
    }
    let players = Rc::new(Mutex::new(players));

    mpris.connect_players_changed(
        clone!(@weak root, @weak label, @strong players, @strong current_player => move |player| {
            glib::spawn_future_local(
                clone!(@weak root, @weak label, @strong players, @strong current_player => async move {
                    handle_new_player(player, &mut *players.lock().await, current_player, root, label);
                }),
            );
        }),
        move |player| {
            glib::spawn_future_local(clone!(@weak root, @weak label, @strong players, @strong current_player => async move {
                if let Err(e) = handle_removed_player(player, &mut *players.lock().await, current_player, root, label).await {
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

fn handle_new_player(
    player: Player,
    players: &mut HashMap<String, Player>,
    current_player: Rc<Mutex<Option<Player>>>,
    root: gtk::Box,
    label: Label,
) {
    manage_player(&player, current_player, root, label);
    players.insert(player.app_name().to_owned(), player);
}

async fn handle_removed_player(
    player: Player,
    players: &mut HashMap<String, Player>,
    current_player: Rc<Mutex<Option<Player>>>,
    root: gtk::Box,
    label: Label,
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
            root.set_visible(true);
            current_player.lock().await.replace(player.clone());
            update_label(&player, &label)
                .await
                .context("While updating label")?;
        } else {
            root.set_visible(false);
            current_player.lock().await.take();
        };
    }

    Ok(())
}

async fn find_suitable_player(players: &HashMap<String, Player>) -> anyhow::Result<Option<Player>> {
    for player in players.values() {
        if player.playback_status().await? == PlaybackStatus::Playing {
            return Ok(Some(player.clone()));
        }
    }

    Ok(players.values().next().cloned())
}

fn manage_player(
    player: &Player,
    current_player: Rc<Mutex<Option<Player>>>,
    root: gtk::Box,
    label: Label,
) {
    player
        .connect_on_properties_changed(clone!(@strong player => move |_, _, _| {
            glib::spawn_future_local(clone!(@strong player, @strong current_player, @weak root, @weak label  => async move {
                if  current_player.lock().await.as_ref().is_some_and(|current| *current == player) {
                    if let Err(e) = update_label(&player, &label).await {
                        error!("Cannot update Mpris label: {e:?}");
                    }
                }

                let status = match player.playback_status().await {
                    Ok(x) => x,
                    Err(e) => {
                        error!("Cannot get playback status: {e:?}");
                        return;
                    },
                };
                if status == PlaybackStatus::Playing {
                    if let Err(e) = update_label(&player, &label).await {
                        error!("Cannot update Mpris label: {e:?}");
                    }
                    current_player.lock().await.replace(player);
                    root.set_visible(true);
                }
            }));
        }));
}

async fn update_label(player: &Player, label: &Label) -> anyhow::Result<()> {
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
