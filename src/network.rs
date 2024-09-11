use crate::{backend::rfkill, HasTooltip};
use futures::TryStreamExt;
use gtk::{
    glib::{self, clone},
    prelude::*,
    EventControllerMotion, Label, Orientation, Revealer, RevealerTransitionType,
};
use gtk4 as gtk;
use libc::RT_TABLE_MAIN;
use log::error;
use netlink_packet_route::{
    address::AddressAttribute,
    link::{LinkAttribute, State},
    route::RouteAttribute,
};
use rtnetlink::IpVersion;
use std::{
    cell::Cell,
    net::IpAddr,
    rc::Rc,
    time::{Duration, SystemTime},
};
use wl_nl80211::{Nl80211Attr, Nl80211BssInfo, Nl80211InformationElements};

#[derive(Debug)]
struct Route {
    pub index: u32,
    pub iname: String,
    pub address: Option<IpAddr>, // mainly used to know if we're only linked and not connected
    pub state: State,

    // Speed
    pub rx_bytes: u64,
    pub tx_bytes: u64,

    // Wifi only
    pub ssid: Option<String>,
    pub signal_strength: Option<u32>,
}

impl Route {
    pub const fn new(index: u32) -> Self {
        Self {
            index,
            iname: String::new(),
            address: None,
            state: State::Unknown,
            rx_bytes: 0,
            tx_bytes: 0,
            ssid: None,
            signal_strength: None,
        }
    }
}

pub fn new() -> gtk::Box {
    let container = gtk::Box::new(Orientation::Horizontal, 0);
    let container2 = container.clone();

    let icon = Label::new(None);
    // todo: add ping ?
    let stats = Label::new(None);
    stats.add_css_class("left");
    let revealer = Revealer::builder()
        .transition_type(RevealerTransitionType::SlideRight)
        .transition_duration(500)
        .child(&stats)
        .build();
    container.append(&revealer);
    container.append(&icon);

    let connected = Rc::new(Cell::new(false));
    let connected2 = Rc::clone(&connected);

    let event_controller = EventControllerMotion::new();
    event_controller.connect_enter(clone!(
        #[strong]
        revealer,
        move |_, _, _| {
            if connected.get() {
                revealer.set_reveal_child(true);
            }
        }
    ));
    event_controller.connect_leave(clone!(
        #[strong]
        revealer,
        move |_| {
            revealer.set_reveal_child(false);
        }
    ));
    container.add_controller(event_controller);

    let (connection, route_handle, _) = rtnetlink::new_connection().unwrap();
    tokio::spawn(connection);
    let (connection, wifi_handle, _) = wl_nl80211::new_connection().unwrap();
    tokio::spawn(connection);

    glib::spawn_future_local(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        let mut route = None;

        loop {
            let now = SystemTime::now();
            interval.tick().await;
            update(
                &mut route,
                &container2,
                &icon,
                &stats,
                &route_handle,
                &wifi_handle,
                &connected2,
                now.elapsed().unwrap(),
            )
            .await;
        }
    });

    container
}

async fn update(
    route: &mut Option<Route>,
    container: &gtk::Box,
    icon: &Label,
    stats: &Label,
    rtnetlink: &rtnetlink::Handle,
    wifi: &wl_nl80211::Nl80211Handle,
    connected_: &Rc<Cell<bool>>,
    delta: Duration,
) {
    let new_route = default_route(rtnetlink, wifi).await;

    let mut connected = false;

    let (icon_, tooltip) = match &new_route {
        Some(Route {
            state: State::Down, ..
        }) => ("󰤯", "Wifi: Disconnected\nEthernet: Not detected".to_owned()),
        Some(Route { address: None, .. }) => {
            connected = true;
            ("󱚵", "Wifi: Linked".to_owned())
        }
        Some(Route {
            ssid: Some(ssid),
            signal_strength,
            iname,
            address,
            ..
        }) => {
            connected = true;
            let icon = signal_strength.as_ref().map_or("󱛇", |signal_strength| {
                ["󰤟", "󰤢", "󰤥", "󰤨"][(*signal_strength as f64 / 25.1) as usize]
            });
            (icon, format!("Wifi: {ssid}\n{iname}: {}", address.unwrap()))
        }
        Some(Route { iname, address, .. }) => {
            connected = true;
            ("󰈁", format!("{iname}: {}", address.unwrap()))
        }
        None => {
            let rfkill_state = rfkill::MANAGER
                .with(|m| {
                    glib::spawn_future_local(clone!(
                        #[strong]
                        m,
                        async move { m.state().await }
                    ))
                })
                .await;
            let wkill = match rfkill_state {
                Ok(x) => x
                    .iter()
                    .find(|(_, (r#type, _))| *r#type == rfkill::Type::Wlan)
                    .map_or(true, |(_, (_, state))| *state),
                Err(e) => {
                    error!("Cannot get rfkill state: {e}");
                    false
                }
            };

            if wkill {
                ("󰖪", "Wifi: Disabled\nEthernet: Not detected".to_owned())
            } else {
                ("󰤯", "Wifi: Disconnected\nEthernet: Not detected".to_owned())
            }
        }
    };

    connected_.set(connected);
    let stats_ = match (connected, &new_route, &route) {
        (
            true,
            Some(Route {
                index: new_index,
                rx_bytes: new_rx_bytes,
                tx_bytes: new_tx_bytes,
                ..
            }),
            Some(Route {
                index: old_index,
                rx_bytes: old_rx_bytes,
                tx_bytes: old_tx_bytes,
                ..
            }),
        ) if new_index == old_index && delta.as_secs_f64() > 0. => {
            format!(
                "  {}/s    {}/s",
                format_bytes((new_rx_bytes - old_rx_bytes) as f64 / delta.as_secs_f64()),
                format_bytes((new_tx_bytes - old_tx_bytes) as f64 / delta.as_secs_f64())
            )
        }
        _ => "  N/A    N/A".to_owned(),
    };

    icon.set_text(icon_);
    stats.set_text(&stats_);
    container.set_better_tooltip(Some(tooltip));

    *route = new_route;
}

fn fixed_width(number: f64, width: usize) -> String {
    let int_part = number as u64;
    let int_len = int_part.to_string().len();
    if int_len >= width {
        format!("{:.0}", number.round())
    } else if int_len + 1 == width {
        // WARNING: this is NOT a space, it's a nobreak space, because a normal space is not large enough
        format!(" {:.0}", number.round())
    } else {
        format!("{number:.0$}", width - (int_len + 1))
    }
}

const UNIT_MULTIPLIER: f64 = 1024.;
const UNIT_PREFIXES: &[&str] = &["", "k", "M", "G", "T"];

fn format_bytes(mut quantity: f64) -> String {
    let mut i = 0;
    while quantity > 1000. {
        quantity /= UNIT_MULTIPLIER;
        i += 1;
    }

    let prefix = UNIT_PREFIXES[i];
    format!("{}{prefix}B", fixed_width(quantity, 4 - prefix.len()))
}

async fn default_route(
    route_handle: &rtnetlink::Handle,
    wifi_handle: &wl_nl80211::Nl80211Handle,
) -> Option<Route> {
    let mut candidates = Vec::new();

    let mut all_routes = route_handle.route().get(IpVersion::V4).execute();
    while let Ok(Some(route)) = all_routes.try_next().await {
        if route.header.table != RT_TABLE_MAIN {
            continue;
        }

        let mut gateway_found = false;
        let mut destination_found = false;
        let mut index = None;
        let mut priority = None;
        for attr in route.attributes {
            match attr {
                RouteAttribute::Destination(_) => destination_found = true,
                RouteAttribute::Gateway(_) => gateway_found = true,
                RouteAttribute::Oif(index_) => index = Some(index_),
                RouteAttribute::Priority(priority_) => priority = Some(priority_),
                _ => (),
            }
        }
        if !gateway_found || destination_found {
            continue;
        }

        if let (Some(index), Some(priority)) = (index, priority) {
            candidates.push((index, priority));
        }
    }

    candidates.sort_by_key(|(_, priority)| *priority);
    let (index, _) = candidates.into_iter().next()?;
    let mut route = Route::new(index);

    let Some(addr) = route_handle
        .address()
        .get()
        .set_link_index_filter(index)
        .execute()
        .try_collect::<Vec<_>>()
        .await
        .unwrap()
        .into_iter()
        .next()
    else {
        return Some(route);
    };

    for attr in addr.attributes {
        match attr {
            AddressAttribute::Address(address) => route.address = Some(address),
            AddressAttribute::Label(label) => route.iname = label,
            _ => (),
        }
    }

    let Some(link) = route_handle
        .link()
        .get()
        .match_index(index)
        .execute()
        .try_collect::<Vec<_>>()
        .await
        .unwrap()
        .into_iter()
        .next()
    else {
        return Some(route);
    };

    for attr in link.attributes {
        match attr {
            LinkAttribute::OperState(state) => route.state = state,
            LinkAttribute::Stats64(stats) => {
                route.rx_bytes = stats.rx_bytes;
                route.tx_bytes = stats.tx_bytes;
            }
            _ => (),
        }
    }

    let Some(interface) = wifi_handle
        .interface()
        .get()
        .execute()
        .await
        .try_collect::<Vec<_>>()
        .await
        .unwrap()
        .into_iter()
        .find(|message| message.payload.nlas.contains(&Nl80211Attr::IfIndex(index)))
    else {
        return Some(route);
    };

    for attr in interface.payload.nlas {
        if let Nl80211Attr::Ssid(ssid) = attr {
            route.ssid = Some(ssid);
        }
    }

    let Some(bss) = wifi_handle
        .scan()
        .dump(index)
        .execute()
        .await
        .try_collect::<Vec<_>>()
        .await
        .unwrap()
        .into_iter()
        .next()
        .and_then(|message| {
            message.payload.nlas.into_iter().find_map(|attr| {
                if let Nl80211Attr::Bss(infos) = attr {
                    Some(infos)
                } else {
                    None
                }
            })
        })
    else {
        return Some(route);
    };

    for info in bss {
        match info {
            Nl80211BssInfo::SignalMbm(strength) => {
                // convert mBm to dBm
                let strength = strength as f64 / 100.;

                // I stole the homeworks of Waybar

                // WiFi-hardware usually operates in the range -90 to -30dBm.
                // If a signal is too strong, it can overwhelm receiving circuity that is designed
                // to pick up and process a certain signal level. The following percentage is scaled to
                // punish signals that are too strong (>= -45dBm) or too weak (<= -45 dBm).
                let hardware_optimum = -45.;
                let hardware_min = -90.;
                let strength = ((strength - hardware_optimum).abs()
                    / (hardware_optimum - hardware_min))
                    .mul_add(-100., 100.);
                let strength = strength.clamp(0., 100.);

                route.signal_strength = Some(strength.round() as u32);
            }
            Nl80211BssInfo::InformationElements(elements) => {
                for element in elements {
                    if let Nl80211InformationElements::Ssid(ssid) = element {
                        route.ssid = Some(ssid);
                    }
                }
            }
            _ => (),
        }
    }

    Some(route)
}
