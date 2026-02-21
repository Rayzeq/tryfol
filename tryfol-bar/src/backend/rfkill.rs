use futures::lock::Mutex;
use gtk4::glib::{self, clone};
use log::{error, warn};
use std::{collections::HashMap, rc::Rc};
use tokio::{fs::File, io::AsyncReadExt};

const RFKILL_PATH: &str = "/dev/rfkill";

thread_local! {
    pub static MANAGER: Manager = {
        let manager = Manager::new();
        glib::spawn_future_local(clone!(
            #[strong] manager,
            async move {
                if let Err(e) = manager.listen().await {
                    error!("Error while listening to rkill events: {e}");
                }
            }
        ));
        manager
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    All,
    Wlan,
    Bluetooth,
    Uwb,
    Wimax,
    Wwan,
    Gps,
    Fm,
    Nfc,
    Unknown,
}

/// I didn't find documentation online, so this documentation is not
/// necessarily correct
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operation {
    /// A device has been added
    Add,
    /// A device has been removed (driver was unloaded ?)
    Del,
    /// The rfkill state of a device changed
    Change,
    ChangeAll,
    Unknown,
}

#[repr(C, packed)]
#[derive(Default)]
struct RawEvent {
    index: u32,
    r#type: u8,
    operation: u8,
    soft: bool,
    hard: bool,
}

#[derive(Debug)]
struct Event {
    pub index: u32,
    pub r#type: Type,
    pub operation: Operation,
    pub soft: bool,
    pub hard: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Manager {
    inner: Rc<Mutex<ManagerInner>>,
}

#[derive(Default)]
pub struct ManagerInner {
    // {(device_index, device_type): is_enabled}
    state: HashMap<u32, (Type, bool)>,
    callbacks: Vec<(Option<u32>, Option<Type>, Box<dyn Fn(u32, Type, bool)>)>,
}

impl Manager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn listen(&self) -> anyhow::Result<()> {
        let mut file = File::open(RFKILL_PATH).await?;

        loop {
            let mut event = RawEvent::default();
            file.read_exact(unsafe { any_as_u8_slice(&mut event) })
                .await?;
            let event: Event = event.into();

            match event.operation {
                Operation::Add => {
                    let mut this = self.inner.lock().await;

                    this.state
                        .insert(event.index, (event.r#type, event.hard || event.soft));
                    for (index, r#type, callback) in &this.callbacks {
                        if let Some(index) = index
                            && *index != event.index
                        {
                            continue;
                        }
                        if let Some(r#type) = r#type
                            && *r#type != event.r#type
                        {
                            continue;
                        }
                        callback(event.index, event.r#type, event.hard || event.soft);
                    }
                }
                Operation::Del => {
                    self.inner.lock().await.state.remove(&event.index);
                }
                Operation::Change => {
                    self.inner
                        .lock()
                        .await
                        .state
                        .entry(event.index)
                        .or_insert((event.r#type, false))
                        .1 = event.hard || event.soft;
                }
                Operation::ChangeAll => todo!("don't know what this event does"),
                Operation::Unknown => (),
            }
        }
    }

    pub async fn state(&self) -> HashMap<u32, (Type, bool)> {
        self.inner.lock().await.state.clone()
    }

    pub async fn connect_changed<F>(
        &self,
        device_index: Option<u32>,
        device_type: Option<Type>,
        callback: F,
    ) where
        F: Fn(u32, Type, bool) + 'static,
    {
        self.inner
            .lock()
            .await
            .callbacks
            .push((device_index, device_type, Box::new(callback)));
    }
}

unsafe fn any_as_u8_slice<T: Sized>(p: &mut T) -> &mut [u8] {
    use core::{mem, ptr, slice};

    unsafe { slice::from_raw_parts_mut(ptr::from_mut(p).cast::<u8>(), mem::size_of::<T>()) }
}

impl From<u8> for Type {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::All,
            1 => Self::Wlan,
            2 => Self::Bluetooth,
            3 => Self::Uwb,
            4 => Self::Wimax,
            5 => Self::Wwan,
            6 => Self::Gps,
            7 => Self::Fm,
            8 => Self::Nfc,
            value => {
                warn!("Invalid rfkill type: {value}");
                Self::Unknown
            }
        }
    }
}

impl From<u8> for Operation {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Add,
            1 => Self::Del,
            2 => Self::Change,
            3 => Self::ChangeAll,
            value => {
                warn!("Invalid rfkill operation: {value}");
                Self::Unknown
            }
        }
    }
}

impl From<RawEvent> for Event {
    fn from(value: RawEvent) -> Self {
        Self {
            index: value.index,
            r#type: value.r#type.into(),
            operation: value.operation.into(),
            soft: value.soft,
            hard: value.hard,
        }
    }
}
