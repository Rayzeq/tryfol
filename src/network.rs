use crate::{rfkill, HasTooltip};
use anyhow::Context;
use futures::{future::Either, FutureExt, Stream, StreamExt, TryStream, TryStreamExt};
use genetlink::GenetlinkHandle;
use gtk::{
    glib::{self, clone},
    prelude::*,
    EventControllerMotion, Label, Orientation, Revealer, RevealerTransitionType,
};
use gtk4 as gtk;
use libc::RT_TABLE_MAIN;
use netlink_packet_core::{ErrorMessage, NetlinkMessage, NLM_F_DUMP, NLM_F_REQUEST};
use netlink_packet_generic::{GenlFamily, GenlHeader, GenlMessage};
use netlink_packet_route::{
    address::AddressAttribute,
    link::{LinkAttribute, State},
    route::RouteAttribute,
};
use netlink_packet_utils::{
    byteorder::{ByteOrder, NativeEndian},
    nla::{DefaultNla, Nla, NlaBuffer, NlasIterator},
    parsers::{parse_u16, parse_u32, parse_u64, parse_u8},
    DecodeError, Emitable, Parseable, ParseableParametrized,
};
use rtnetlink::IpVersion;
use std::{
    cell::Cell,
    net::IpAddr,
    rc::Rc,
    time::{Duration, SystemTime},
};
use wl_nl80211::Nl80211Attr;

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
    event_controller.connect_enter(clone!(@strong revealer => move |_, _, _| {
        if connected.get() {
            revealer.set_reveal_child(true);
        }
    }));
    event_controller.connect_leave(clone!(@strong revealer => move |_| {
        revealer.set_reveal_child(false);
    }));
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
            let wkill = rfkill::list()
                .into_iter()
                .find(|event| event.r#type == rfkill::Type::Wlan)
                .unwrap();
            if wkill.soft || wkill.hard {
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

    let Some(bss) = nl80211_get_scan(wifi_handle, index)
        .await
        .try_collect::<Vec<_>>()
        .await
        .unwrap()
        .into_iter()
        .next()
        .and_then(|message| {
            message.payload.nlas.into_iter().find_map(|attr| {
                if let CustomAttr::Bss(infos) = attr {
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
            BssInfo::SignalMbm(strength) => {
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
            BssInfo::InformationElements(elements) => {
                for element in elements {
                    if let InformationElement::Ssid(ssid) = element {
                        route.ssid = Some(ssid);
                    }
                }
            }
            _ => (),
        }
    }

    Some(route)
}

pub struct CustomHandle {
    pub handle: GenetlinkHandle,
}

impl CustomHandle {
    pub async fn request(
        &mut self,
        message: NetlinkMessage<GenlMessage<CustomMessage>>,
    ) -> Result<
        impl Stream<Item = Result<NetlinkMessage<GenlMessage<CustomMessage>>, DecodeError>>,
        CustomError,
    > {
        self.handle
            .request(message)
            .await
            .map_err(|e| CustomError::RequestFailed(format!("BUG: Request failed with {}", e)))
    }
}

#[derive(Clone, Eq, PartialEq, Debug, thiserror::Error)]
pub enum CustomError {
    #[error("Received an unexpected message {0:?}")]
    UnexpectedMessage(NetlinkMessage<GenlMessage<CustomMessage>>),

    #[error("Received a netlink error message {0}")]
    NetlinkError(ErrorMessage),

    #[error("A netlink request failed")]
    RequestFailed(String),

    #[error("A bug in this crate")]
    Bug(String),
}

macro_rules! try_nl80211 {
    ($msg: expr) => {{
        use netlink_packet_core::{NetlinkMessage, NetlinkPayload};

        match $msg {
            Ok(msg) => {
                let (header, payload) = msg.into_parts();
                match payload {
                    NetlinkPayload::InnerMessage(msg) => msg,
                    NetlinkPayload::Error(err) => return Err(CustomError::NetlinkError(err)),
                    _ => {
                        return Err(CustomError::UnexpectedMessage(NetlinkMessage::new(
                            header, payload,
                        )))
                    }
                }
            }
            Err(e) => return Err(CustomError::Bug(format!("BUG: decode error {:?}", e))),
        }
    }};
}

async fn nl80211_get_scan(
    wifi_handle: &wl_nl80211::Nl80211Handle,
    iface: u32,
) -> impl TryStream<Ok = GenlMessage<CustomMessage>, Error = CustomError> {
    let mut handle = CustomHandle {
        handle: wifi_handle.handle.clone(),
    };
    let nl80211_msg = CustomMessage::new_scan_get(iface);
    let mut nl_msg = NetlinkMessage::from(GenlMessage::from_payload(nl80211_msg));

    nl_msg.header.flags = NLM_F_REQUEST | NLM_F_DUMP;

    match handle.request(nl_msg).await {
        Ok(response) => Either::Left(response.map(move |msg| Ok(try_nl80211!(msg)))),
        Err(e) => Either::Right(
            futures::future::err::<GenlMessage<CustomMessage>, CustomError>(e).into_stream(),
        ),
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct CustomMessage {
    pub cmd: u8,
    pub nlas: Vec<CustomAttr>,
}

impl GenlFamily for CustomMessage {
    fn family_name() -> &'static str {
        "nl80211"
    }

    fn version(&self) -> u8 {
        1
    }

    fn command(&self) -> u8 {
        self.cmd
    }
}

impl CustomMessage {
    pub fn new_scan_get(iface: u32) -> Self {
        let nlas = vec![CustomAttr::IfIndex(iface)];

        Self { cmd: 32, nlas }
    }
}

impl Emitable for CustomMessage {
    fn buffer_len(&self) -> usize {
        self.nlas.as_slice().buffer_len()
    }

    fn emit(&self, buffer: &mut [u8]) {
        self.nlas.as_slice().emit(buffer)
    }
}

fn parse_nlas(buffer: &[u8]) -> Result<Vec<CustomAttr>, DecodeError> {
    let mut nlas = Vec::new();
    for nla in NlasIterator::new(buffer) {
        let error_msg = format!("Failed to parse nl80211 message attribute {:?}", nla);
        let nla = &nla.context(error_msg.clone())?;
        nlas.push(CustomAttr::parse(nla).context(error_msg)?);
    }
    Ok(nlas)
}

impl ParseableParametrized<[u8], GenlHeader> for CustomMessage {
    fn parse_with_param(buffer: &[u8], header: GenlHeader) -> Result<Self, DecodeError> {
        Ok(match header.cmd {
            34 => Self {
                cmd: 32,
                nlas: parse_nlas(buffer)?,
            },
            cmd => {
                return Err(DecodeError::from(format!(
                    "Unsupported nl80211 reply command: {}",
                    cmd
                )));
            }
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum CustomAttr {
    IfIndex(u32),
    Generation(u32),
    Bss(Vec<BssInfo>),
    Wdev(u64),
}

impl Nla for CustomAttr {
    fn value_len(&self) -> usize {
        match self {
            Self::IfIndex(_) => 4,
            _ => todo!(),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::IfIndex(_) => 3,
            _ => todo!(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::IfIndex(d) => NativeEndian::write_u32(buffer, *d),
            _ => todo!(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for CustomAttr {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            3 => {
                let err_msg = format!("Invalid NL80211_ATTR_IFINDEX value {:?}", payload);
                Self::IfIndex(parse_u32(payload).context(err_msg)?)
            }
            46 => {
                let err_msg = format!("Invalid NL80211_ATTR_GENERATION value {:?}", payload);
                Self::Generation(parse_u32(payload).context(err_msg)?)
            }
            47 => {
                let err_msg = format!("Invalid NL80211_ATTR_STA_INFO value {:?}", payload);
                let mut nlas = Vec::new();
                for nla in NlasIterator::new(payload) {
                    let nla = &nla.context(err_msg.clone())?;
                    nlas.push(BssInfo::parse(nla).context(err_msg.clone())?);
                }
                Self::Bss(nlas)
            }
            153 => {
                let err_msg = format!("Invalid NL80211_ATTR_WDEV value {:?}", payload);
                Self::Wdev(parse_u64(payload).context(err_msg)?)
            }
            n => todo!("{:?}", n),
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum BssInfo {
    // Bssid(hex),
    Frequency(u32),
    // Tsf(TSF),
    BeaconInterval(u16),
    // Capability(capability),
    InformationElements(Vec<InformationElement>),
    SignalMbm(i32),
    SignalUnspec(u8),
    Status(u32),
    SeenMsAgo(u32),
    // BeaconIes(elementsBinary),
    ChanWidth(u32),
    BeaconTsf(u64),
    // PrespData(hex),
    // Max(hex),
    Other(DefaultNla),
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for BssInfo {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, netlink_packet_utils::DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            1 => Self::Other(DefaultNla::parse(buf).context("invalid NLA (unknown kind)")?),
            2 => {
                let err_msg = format!("Invalid NL80211_ATTR_GENERATION value {:?}", payload);
                Self::Frequency(parse_u32(payload).context(err_msg)?)
            }
            3 => Self::Other(DefaultNla::parse(buf).context("invalid NLA (unknown kind)")?),
            4 => {
                let err_msg = format!("Invalid NL80211_ATTR_GENERATION value {:?}", payload);
                Self::BeaconInterval(parse_u16(payload).context(err_msg)?)
            }
            5 => Self::Other(DefaultNla::parse(buf).context("invalid NLA (unknown kind)")?),
            6 => Self::InformationElements(InformationElement::parse_vec(buf).unwrap()),
            7 => {
                let err_msg = format!("Invalid NL80211_ATTR_GENERATION value {:?}", payload);
                Self::SignalMbm(parse_u32(payload).context(err_msg)? as i32)
            }
            8 => {
                let err_msg = format!("Invalid NL80211_ATTR_GENERATION value {:?}", payload);
                Self::SignalUnspec(parse_u8(payload).context(err_msg)?)
            }
            9 => {
                let err_msg = format!("Invalid NL80211_ATTR_GENERATION value {:?}", payload);
                Self::Status(parse_u32(payload).context(err_msg)?)
            }
            10 => {
                let err_msg = format!("Invalid NL80211_ATTR_GENERATION value {:?}", payload);
                Self::SeenMsAgo(parse_u32(payload).context(err_msg)?)
            }
            11 => Self::Other(DefaultNla::parse(buf).context("invalid NLA (unknown kind)")?),
            12 => {
                let err_msg = format!("Invalid NL80211_ATTR_GENERATION value {:?}", payload);
                Self::ChanWidth(parse_u32(payload).context(err_msg)?)
            }
            13 => {
                let err_msg = format!("Invalid NL80211_ATTR_GENERATION value {:?}", payload);
                Self::BeaconTsf(parse_u64(payload).context(err_msg)?)
            }
            _ => Self::Other(DefaultNla::parse(buf).context("invalid NLA (unknown kind)")?),
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum InformationElement {
    Ssid(String),
    Other(u8, Vec<u8>),
}

impl InformationElement {
    fn parse_vec<T: AsRef<[u8]> + ?Sized>(
        buf: &NlaBuffer<&T>,
    ) -> Result<Vec<Self>, netlink_packet_utils::DecodeError> {
        let mut result = Vec::new();
        let payload = buf.value();

        let mut offset = 0;

        while offset < payload.len() {
            let msg_type = parse_u8(&payload[offset..][..1]).unwrap();
            let length = parse_u8(&payload[offset + 1..][..1]).unwrap() as usize;

            match msg_type {
                0 => result.push(Self::Ssid(
                    String::from_utf8(payload[offset + 2..][..length].to_vec()).unwrap(),
                )),
                msg_type => result.push(Self::Other(
                    msg_type,
                    payload[offset + 2..][..length].to_owned(),
                )),
            }

            offset += length + 2;
        }

        Ok(result)
    }
}
