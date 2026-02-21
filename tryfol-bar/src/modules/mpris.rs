use crate::{
    Clickable, Hoverable,
    backend::{
        hyprland,
        mpris::{LoopStatus, Mpris, PlaybackStatus, Player},
    },
};
use anyhow::{Context as _, bail};
use async_recursion::async_recursion;
use core::f64;
use futures::FutureExt;
use gtk::{
    Button, Label, Revealer, RevealerTransitionType,
    glib::{self, clone},
    pango::EllipsizeMode,
    prelude::*,
};
use gtk4::{
    self as gtk, Align, Image, Orientation, Overflow, Popover, Scale,
    gdk::Texture,
    gdk_pixbuf::Pixbuf,
    gio::{Cancellable, MemoryInputStream},
    glib::JoinHandle,
};
use log::error;
use reqwest::Url;
use std::{
    collections::HashMap,
    rc::Rc,
    time::{Duration, Instant},
};
use tokio::{sync::Mutex, time::sleep};

pub fn new() -> gtk::Box {
    Module::new()
}

struct Module {
    /// Hashmap containing all players.
    ///
    /// Players are identified by their Dbus reverse DNS name (which also contains an
    /// instance-specific id).
    players: Mutex<HashMap<String, (Player, MprisController)>>,
    current_player: Mutex<Option<Player>>,
    root: gtk::Box,
    label: Label,
    all_players: gtk::Box,
}

struct MprisController {
    player: Player,
    pub root: gtk::Box,
    cover: Image,
    title: Label,
    album: Label,
    artists: Label,
    infos_column: gtk::Box,
    previous: Button,
    play_pause: Button,
    next: Button,
    secondary_controls: gtk::Box,
    loop_status: Button,
    shuffle_status: Button,
    timeslider_handle: JoinHandle<()>,
}

#[derive(Debug, Clone)]
struct TimeSlider {
    player: Player,
    rate: Option<f64>,
    status: PlaybackStatus,

    root: gtk::Box,
    scale: Scale,
    current_time: Label,
    total_duration: Label,
}

impl Module {
    #[allow(clippy::new_ret_no_self)]
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
            // explicitly set hexpand to false to prevent it from being inherited
            // by the popover's content
            .hexpand_set(true)
            .build();
        root.append(&left_revealer);
        root.append(&play_pause);
        root.append(&right_revealer);

        let all_players = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(5)
            .build();

        let popover = Popover::builder()
            .name("players")
            .has_arrow(false)
            .child(&all_players)
            .build();
        popover.set_parent(&root);

        root.connect_hover_notify(move |_, hovered| {
            left_revealer.set_reveal_child(hovered);
            right_revealer.set_reveal_child(hovered);
        });

        let this = Rc::new(Self {
            players: Mutex::new(HashMap::new()),
            current_player: Mutex::new(None),
            root,
            label,
            all_players,
        });

        // Note: because of the strong clones here, current_player won't be dropped unless
        // the signal are disconnected (or the widget is dropped ?)
        play_pause.connect_left_clicked(clone!(
            #[strong]
            this,
            move |_, _, _, _| {
                glib::spawn_future_local(clone!(
                    #[strong]
                    this,
                    async move {
                        if let Some(player) = &*this.current_player.lock().await
                            && let Err(e) = player.toggle().await
                        {
                            error!("Cannot play/pause Mpris player: {e:?}");
                        }
                    }
                ));
            }
        ));
        previous.connect_left_clicked(clone!(
            #[strong]
            this,
            move |_, _, _, _| {
                glib::spawn_future_local(clone!(
                    #[strong]
                    this,
                    async move {
                        if let Some(player) = &*this.current_player.lock().await
                            && let Err(e) = player.previous().await
                        {
                            error!("Cannot skip Mpris player backward: {e:?}");
                        }
                    }
                ));
            }
        ));
        next.connect_left_clicked(clone!(
            #[strong]
            this,
            move |_, _, _, _| {
                glib::spawn_future_local(clone!(
                    #[strong]
                    this,
                    async move {
                        if let Some(player) = &*this.current_player.lock().await
                            && let Err(e) = player.next().await
                        {
                            error!("Cannot skip Mpris player forward: {e:?}");
                        }
                    }
                ));
            }
        ));
        this.root.connect_right_clicked(move |_, _, _, _| {
            popover.popup();
        });

        let retval = this.root.clone();
        glib::spawn_future_local(clone!(
            #[strong]
            this,
            async move {
                if let Err(e) = this.listen().await {
                    error!("Cannot listen for Mpris players: {e:?}");
                }
            }
        ));

        retval
    }

    async fn listen(self: Rc<Self>) -> anyhow::Result<()> {
        let mpris = Mpris::new()
            .await
            .context("While creating Mpris connection")?;

        for player in mpris.players().await.context("While getting players")? {
            Rc::clone(&self).handle_new_player(player).await;
        }

        // TODO: a player might disappear and cause an error here
        if let Some(player) = self
            .find_suitable_player()
            .await
            .context("While finding suitable player")?
        {
            self.root.set_visible(true);
            self.current_player.lock().await.replace(player.clone());
            self.update_label(&player)
                .await
                .context("While updating label")?;
        }

        mpris.connect_players_changed(
            clone!(
                #[strong(rename_to = this)]
                self,
                move |player| {
                    glib::spawn_future_local(clone!(
                        #[strong]
                        this,
                        async move {
                            this.handle_new_player(player).await;
                        }
                    ));
                }
            ),
            clone!(
                #[strong(rename_to = this)]
                self,
                move |player| {
                    glib::spawn_future_local(clone!(
                        #[strong]
                        this,
                        async move {
                            if let Err(e) = this.handle_removed_player(player).await {
                                error!("Cannot handle removed Mpris player: {e:?}");
                            }
                        }
                    ));
                }
            ),
        );

        // leak the message bus, so it's not dropped and the signals can be received
        // TODO: do we still need this ?
        std::mem::forget(mpris);

        Ok(())
    }

    async fn handle_new_player(self: Rc<Self>, player: Player) {
        let controller = MprisController::new(player.clone()).await;
        self.all_players.append(&controller.root);

        Rc::clone(&self).manage_player(&player);
        self.players
            .lock()
            .await
            .insert(player.app_name().to_owned(), (player, controller));
    }

    #[async_recursion(?Send)]
    async fn handle_removed_player(&self, player: Player) -> anyhow::Result<()> {
        if let Some((_, controller)) = self
            .players
            .lock()
            .await
            .remove(&player.app_name().to_owned())
        {
            self.all_players.remove(&controller.root);
        }

        if self
            .current_player
            .lock()
            .await
            .as_ref()
            .is_some_and(|current| *current == player)
        {
            if let Some(player) = self
                .find_suitable_player()
                .await
                .context("While finding suitable player")?
            {
                let result = self.update_label(&player).await;
                match result {
                    Ok(()) => (),
                    Err(zbus::Error::FDO(e))
                        if matches!(*e, zbus::fdo::Error::ServiceUnknown(_)) =>
                    {
                        return self.handle_removed_player(player).await;
                    }
                    Err(e) => {
                        error!("Cannot handle Mpris player update: {e:?}");
                    }
                }
                self.root.set_visible(true);
                self.current_player.lock().await.replace(player);
            } else {
                self.root.set_visible(false);
                self.current_player.lock().await.take();
            };
        }

        Ok(())
    }

    async fn find_suitable_player(&self) -> zbus::Result<Option<Player>> {
        let mut players = self.players.lock().await;
        let mut zombie_players = Vec::new();

        for (name, (player, _)) in players.iter_mut() {
            match player.playback_status().await {
                Ok(PlaybackStatus::Playing) => return Ok(Some(player.clone())),
                Ok(_) => (),
                Err(zbus::Error::FDO(e)) if matches!(*e, zbus::fdo::Error::ServiceUnknown(_)) => {
                    zombie_players.push(name.clone());
                }
                Err(e) => return Err(e),
            }
        }

        for name in zombie_players {
            players.remove(&name);
        }

        Ok(players.values().next().map(|x| x.0.clone()))
    }

    fn manage_player(self: Rc<Self>, player: &Player) {
        player.connect_on_properties_changed(clone!(
            #[strong]
            player,
            move |_, _, _| {
                clone!(
                    #[strong(rename_to = this)]
                    self,
                    #[strong]
                    player,
                    async move {
                        let result = this.handle_player_update(player.clone()).await;
                        match result {
                            Ok(()) => (),
                            Err(zbus::Error::FDO(e))
                                if matches!(*e, zbus::fdo::Error::ServiceUnknown(_)) =>
                            {
                                if let Err(e) = this.handle_removed_player(player).await {
                                    error!("Cannot find new Mpris player: {e:?}");
                                }
                            }
                            Err(e) => {
                                error!("Cannot handle Mpris player update: {e:?}");
                            }
                        }
                    }
                )
            }
        ));
    }

    #[allow(clippy::significant_drop_tightening)]
    async fn handle_player_update(&self, player: Player) -> zbus::Result<()> {
        if let Some((_, controller)) = self.players.lock().await.get(&player.app_name().to_owned())
        {
            controller.update().await?;
        } else {
            // the player already have been removed
            return Ok(());
        };

        let mut current_player = self.current_player.lock().await;
        if current_player
            .as_ref()
            .is_some_and(|current| *current == player)
        {
            self.update_label(&player).await?;
            return Ok(());
        }

        let status = player.playback_status().await?;
        if status == PlaybackStatus::Playing {
            self.update_label(&player).await?;
            current_player.replace(player);
            self.root.set_visible(true);
        }

        Ok(())
    }

    async fn update_label(&self, player: &Player) -> zbus::Result<()> {
        let status = player.playback_status().await?;
        let mut text = if status == PlaybackStatus::Playing {
            "󰏤  "
        } else {
            "󰐊  "
        }
        .to_owned();

        text += player.title().await?.as_deref().unwrap_or("Unknown");
        let artist = player.artists().await?;
        if !(artist.is_empty() || artist.iter().all(String::is_empty)) {
            text += " - ";
            text += &artist.join(", ");
        }
        self.label.set_text(&text);

        Ok(())
    }
}

/// This is basically the same design as the media controller in gnome shell.
// root
//   - main_row
//     - cover
//     - infos_column
//     - controls_column
//   - timeslider
impl MprisController {
    #[allow(clippy::too_many_lines)]
    async fn new(player: Player) -> Self {
        let cover = Image::builder()
            .css_classes(["cover"])
            .pixel_size(75)
            .overflow(Overflow::Hidden)
            .build();

        let title = Label::builder()
            .css_classes(["title"])
            .halign(Align::Start)
            .build();
        let album = Label::builder()
            .css_classes(["album"])
            .halign(Align::Start)
            .build();
        let artists = Label::builder()
            .css_classes(["artists"])
            .halign(Align::Start)
            .build();

        let infos_column = gtk::Box::builder()
            .css_classes(["infos"])
            .orientation(Orientation::Vertical)
            .hexpand(true)
            .build();
        infos_column.append(&title);
        infos_column.append(&album);
        infos_column.append(&artists);

        let previous = Button::builder().label("󰒫").build();
        let play_pause = Button::builder().build();
        let next = Button::builder().label("󰒬").build();

        let primary_controls_row = gtk::Box::builder()
            .css_classes(["primary"])
            .spacing(20)
            // allows the mains controls to be vertically centered when there
            // is no secondary controls
            .vexpand(true)
            .build();
        primary_controls_row.append(&previous);
        primary_controls_row.append(&play_pause);
        primary_controls_row.append(&next);

        let loop_status = Button::builder().build();
        let shuffle_status = Button::builder().build();

        let secondary_controls_row = gtk::Box::builder()
            .css_classes(["secondary"])
            .spacing(15)
            .halign(Align::Center)
            .build();
        secondary_controls_row.append(&loop_status);
        secondary_controls_row.append(&shuffle_status);

        let controls_column = gtk::Box::builder()
            .css_classes(["controls"])
            .spacing(10)
            .orientation(Orientation::Vertical)
            .build();
        controls_column.append(&primary_controls_row);
        controls_column.append(&secondary_controls_row);

        let main_row = gtk::Box::builder().spacing(10).build();
        main_row.append(&cover);
        main_row.append(&infos_column);
        main_row.append(&controls_column);

        let root = gtk::Box::builder()
            .css_classes(["mpris-controller"])
            .orientation(Orientation::Vertical)
            .build();
        root.append(&main_row);

        let (timeslider_handle, timeslider) = TimeSlider::new(player.clone());
        root.append(&timeslider);

        previous.connect_left_clicked(clone!(
            #[strong]
            player,
            move |_, _, _, _| {
                glib::spawn_future_local(clone!(
                    #[strong]
                    player,
                    async move {
                        if let Err(e) = player.previous().await {
                            error!("Cannot skip Mpris player backward: {e:?}");
                        }
                    }
                ));
            }
        ));
        play_pause.connect_left_clicked(clone!(
            #[strong]
            player,
            move |_, _, _, _| {
                glib::spawn_future_local(clone!(
                    #[strong]
                    player,
                    async move {
                        if let Err(e) = player.toggle().await {
                            error!("Cannot play/pause Mpris player: {e:?}");
                        }
                    }
                ));
            }
        ));
        next.connect_left_clicked(clone!(
            #[strong]
            player,
            move |_, _, _, _| {
                glib::spawn_future_local(clone!(
                    #[strong]
                    player,
                    async move {
                        if let Err(e) = player.next().await {
                            error!("Cannot skip Mpris player forward: {e:?}");
                        }
                    }
                ));
            }
        ));
        loop_status.connect_left_clicked(clone!(
            #[strong]
            player,
            move |_, _, _, _| {
                async fn cycle_loop_status(player: Player) -> zbus::Result<()> {
                    player
                        .set_loop_status(match player.loop_status().await? {
                            Some(LoopStatus::None) => LoopStatus::Playlist,
                            Some(LoopStatus::Playlist) => LoopStatus::Track,
                            Some(LoopStatus::Track) => LoopStatus::None,
                            None => return Ok(()),
                        })
                        .await
                }

                glib::spawn_future_local(clone!(
                    #[strong]
                    player,
                    async move {
                        if let Err(e) = cycle_loop_status(player).await {
                            error!("Cannot cycle loop status: {e:?}");
                        }
                    }
                ));
            }
        ));
        shuffle_status.connect_left_clicked(clone!(
            #[strong]
            player,
            move |_, _, _, _| {
                async fn toggle_shuffle(player: Player) -> zbus::Result<()> {
                    if let Some(is_shuffled) = player.is_shuffled().await? {
                        player.set_shuffled(!is_shuffled).await
                    } else {
                        Ok(())
                    }
                }

                glib::spawn_future_local(clone!(
                    #[strong]
                    player,
                    async move {
                        if let Err(e) = toggle_shuffle(player).await {
                            error!("Cannot toggle shuffle status: {e:?}");
                        }
                    }
                ));
            }
        ));
        root.connect_left_clicked(clone!(
            #[strong]
            player,
            move |_, _, _, _| {
                glib::spawn_future_local(clone!(
                    #[strong]
                    player,
                    async move {
                        let pid = match player.pid().await {
                            Ok(x) => x,
                            Err(e) => {
                                error!("Cannot get mpris player pid: {e}");
                                return;
                            }
                        };
                        let windows = match hyprland::windows().await {
                            Ok(x) => x,
                            Err(e) => {
                                error!("Cannot get window list: {e}");
                                return;
                            }
                        };
                        let Some(window) = windows.into_iter().find(|x| x.pid as u32 == pid) else {
                            error!("Player window was not found");
                            return;
                        };
                        if let Err(e) = hyprland::change_workspace(window.workspace.id()).await {
                            error!("Cannot switch workspace: {e}");
                        }
                    }
                ));
            }
        ));

        let this = Self {
            player,
            root,
            cover,
            title,
            album,
            artists,
            infos_column,
            previous,
            play_pause,
            next,
            secondary_controls: secondary_controls_row,
            loop_status,
            shuffle_status,
            timeslider_handle,
        };
        if let Err(e) = this.update().await {
            error!("Cannot update mpris controller: {e}");
        }

        this
    }

    async fn update(&self) -> zbus::Result<()> {
        if let Some(art_url) = self.player.art_url().await? {
            // we assume the art url always starts with `file://`
            // TODO: remove this limitation
            let path = art_url
                .strip_prefix("file://")
                .unwrap_or(&art_url)
                .to_owned();
            glib::spawn_future_local(Self::get_cover(art_url).then(clone!(
                #[strong(rename_to = cover)]
                self.cover,
                move |texture| async move {
                    match texture {
                        Ok(texture) => cover.set_paintable(Some(&texture)),
                        Err(e) => error!("Cannot get media art: {e}"),
                    }
                }
            )));
            self.cover.set_from_file(Some(path));
            self.cover.set_visible(true);
        } else {
            self.cover.set_visible(false);
        };

        self.title
            .set_text(self.player.title().await?.as_deref().unwrap_or("Unknown"));
        if let Some(album) = self.player.album().await? {
            self.album.set_text(&album);
            self.album.set_visible(true);
        } else {
            self.album.set_visible(false);
        }

        let artists = self.player.artists().await?;
        self.artists.set_text(&artists.join(", "));
        self.artists.set_visible(!artists.is_empty());

        if self.album.get_visible() || self.artists.get_visible() {
            self.infos_column.set_valign(Align::Start);
        } else {
            self.infos_column.set_valign(Align::Center);
        }

        self.previous
            .set_sensitive(self.player.can_go_previous().await?);
        if self.player.playback_status().await? == PlaybackStatus::Playing {
            self.play_pause.set_label("󰏤");
            self.play_pause
                .set_sensitive(self.player.can_pause().await?);
        } else {
            self.play_pause.set_label("󰐊");
            self.play_pause.set_sensitive(self.player.can_play().await?);
        }
        self.next.set_sensitive(self.player.can_go_next().await?);

        self.play_pause
            .set_sensitive(self.player.can_toggle().await?);

        if let Some(status) = self.player.loop_status().await? {
            self.loop_status.set_visible(true);
            self.loop_status.set_label(match status {
                LoopStatus::None => "󰑗",
                LoopStatus::Track => "󰑘",
                LoopStatus::Playlist => "󰑖",
            });
        } else {
            self.loop_status.set_visible(false);
        }
        self.loop_status
            .set_sensitive(self.player.can_control().await?);

        if let Some(is_shuffled) = self.player.is_shuffled().await? {
            self.shuffle_status.set_visible(true);
            self.shuffle_status
                .set_label(if is_shuffled { "󰒟" } else { "󰒞" });
        } else {
            self.shuffle_status.set_visible(false);
        }
        self.shuffle_status
            .set_sensitive(self.player.can_control().await?);

        if self.loop_status.get_visible() || self.shuffle_status.get_visible() {
            self.secondary_controls.set_visible(true);
        } else {
            self.secondary_controls.set_visible(false);
        }

        Ok(())
    }

    async fn get_cover(url: String) -> anyhow::Result<Texture> {
        let url = Url::parse(&url)?;
        let pixbuf = if url.scheme() == "file" {
            let Ok(path) = url.to_file_path() else {
                bail!("Invalid file:// url: {url}");
            };
            Pixbuf::from_file(path)?
        } else {
            let image = reqwest::get(url).await?.bytes().await?;
            let stream = MemoryInputStream::from_bytes(&glib::Bytes::from(&image));
            Pixbuf::from_stream(&stream, None::<&Cancellable>)?
        };

        Ok(Texture::for_pixbuf(&pixbuf))
    }
}

impl Drop for MprisController {
    fn drop(&mut self) {
        self.timeslider_handle.abort();
    }
}

impl TimeSlider {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(player: Player) -> (JoinHandle<()>, gtk::Box) {
        let scale = Scale::builder()
            .orientation(Orientation::Horizontal)
            .hexpand(true)
            .build();
        let current_time = Label::new(None);
        let total_duration = Label::new(None);

        let root = gtk::Box::new(Orientation::Horizontal, 5);
        root.append(&current_time);
        root.append(&scale);
        root.append(&total_duration);

        let this = Self {
            player,
            rate: None,
            status: PlaybackStatus::Stopped,

            root: root.clone(),
            scale,
            current_time,
            total_duration,
        };
        let update_handle = glib::spawn_future_local(async move {
            if let Err(e) = this.setup().await {
                error!("Cannot setup time slider: {e}");
            }
        });

        (update_handle, root)
    }

    async fn setup(self) -> zbus::Result<()> {
        let this = Rc::new(Mutex::new(self.clone()));

        self.player.connect_on_properties_changed(clone!(
            #[strong]
            this,
            move |_, _, _| clone!(
                #[strong]
                this,
                async move {
                    match this.lock().await.update().await {
                        Ok(()) => (),
                        // the player is gone, it should be removed soon
                        Err(zbus::Error::FDO(e))
                            if matches!(*e, zbus::fdo::Error::ServiceUnknown(_)) => {}
                        Err(e) => {
                            error!("Error while updating player time slider: {e}");
                        }
                    }
                }
            )
        ));
        self.player.connect_seeked(clone!(
            #[strong]
            this,
            move |position| clone!(
                #[strong]
                this,
                async move {
                    this.lock().await.scale.set_value(position as f64);
                }
            )
        ));
        this.lock().await.update().await?;

        Self::run(this).await;

        Ok(())
    }

    async fn update(&mut self) -> zbus::Result<()> {
        self.root.set_visible(true);
        if let Some(length) = self.player.length().await? {
            self.scale.set_range(0., length as f64);
            self.total_duration.set_text(&Self::format_time(length));
        } else {
            self.root.set_visible(false);
        }

        if let Some(position) = self.player.position().await? {
            self.scale.set_value(position as f64);
            self.current_time.set_text(&Self::format_time(position));
        } else {
            self.root.set_visible(false);
        }

        self.rate = self.player.rate().await?;
        self.status = self.player.playback_status().await?;

        Ok(())
    }

    async fn run(this: Rc<Mutex<Self>>) {
        let mut last_time = Instant::now();

        loop {
            sleep(Duration::from_millis(100)).await;
            let now = Instant::now();
            let delta = now - last_time;
            last_time = now;

            let this = this.lock().await;
            if let (PlaybackStatus::Playing, Some(rate)) = (this.status, this.rate) {
                let delta = delta.as_micros() as f64 * rate;
                let new_time = this.scale.value() + delta;

                this.scale.set_value(new_time);
                this.current_time
                    .set_text(&Self::format_time(new_time as i64));
            }
        }
    }

    fn format_time(time: i64) -> String {
        let seconds = time / 1000 / 1000;
        let minutes = seconds / 60;
        let seconds = seconds % 60;
        if minutes > 60 {
            let hours = minutes / 60;
            let minutes = minutes % 60;
            format!("{hours:02}:{minutes:02}:{seconds:02}")
        } else {
            format!("{minutes:02}:{seconds:02}")
        }
    }
}
