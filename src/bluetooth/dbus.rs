use zbus::{proxy, zvariant::ObjectPath};

// emits_changed_signal=false because the signals are emitted in a way that they
// aren't detected by default (why tho?)
// even PropertiesChanged doesn't work depending on how it is used (don't worry past me,
// I don't understand either)

#[proxy(
    interface = "org.bluez.Device1",
    default_service = "org.bluez",
    default_path = "/org/bluez/hciX/dev_XX_XX_XX_XX_XX_XX"
)]
trait Device {
    async fn Pair(&self) -> Result<(), zbus::Error>;
    async fn CancelPairing(&self) -> Result<(), zbus::Error>;
    async fn Connect(&self) -> Result<(), zbus::Error>;
    async fn ConnectProfile(&self, uuid: &str) -> Result<(), zbus::Error>;
    async fn Disconnect(&self) -> Result<(), zbus::Error>;
    async fn DisconnectProfile(&self, uuid: &str) -> Result<(), zbus::Error>;

    #[zbus(property)]
    #[allow(non_snake_case)]
    fn UUIDS(&self) -> Result<Vec<String>, zbus::Error>;

    #[zbus(property(emits_changed_signal = "false"))]
    #[allow(non_snake_case)]
    fn Blocked(&self) -> Result<bool, zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn set_Blocked(&self, value: bool) -> Result<(), zbus::Error>;

    #[zbus(property(emits_changed_signal = "false"))]
    #[allow(non_snake_case)]
    fn Bonded(&self) -> Result<bool, zbus::Error>;

    #[zbus(property(emits_changed_signal = "false"))]
    #[allow(non_snake_case)]
    fn Connected(&self) -> Result<bool, zbus::Error>;

    #[zbus(property)]
    #[allow(non_snake_case)]
    fn LegacyPairing(&self) -> Result<bool, zbus::Error>;

    #[zbus(property(emits_changed_signal = "false"))]
    #[allow(non_snake_case)]
    fn Paired(&self) -> Result<bool, zbus::Error>;

    #[zbus(property)]
    #[allow(non_snake_case)]
    fn ServicesResolved(&self) -> Result<bool, zbus::Error>;

    #[zbus(property(emits_changed_signal = "false"))]
    #[allow(non_snake_case)]
    fn Trusted(&self) -> Result<bool, zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn set_Trusted(&self, value: bool) -> Result<(), zbus::Error>;

    #[zbus(property)]
    #[allow(non_snake_case)]
    fn WakeAllowed(&self) -> Result<bool, zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn set_WakeAllowed(&self, value: bool) -> Result<(), zbus::Error>;

    // #[zbus(property)]
    // fn Sets(&self) -> Result<Vec<String>, zbus::Error>;

    // #[zbus(property)]
    // fn ServiceData(&self) -> Result<Vec<String>, zbus::Error>;

    // #[zbus(property)]
    // fn ManufacturerData(&self) -> Result<Vec<String>, zbus::Error>;

    #[zbus(property)]
    #[allow(non_snake_case)]
    fn RSSI(&self) -> Result<i16, zbus::Error>;

    #[zbus(property)]
    #[allow(non_snake_case)]
    fn TxPower(&self) -> Result<i16, zbus::Error>;

    #[zbus(property)]
    #[allow(non_snake_case)]
    fn Adapter(&self) -> Result<ObjectPath, zbus::Error>;

    #[zbus(property)]
    #[allow(non_snake_case)]
    fn Address(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    #[allow(non_snake_case)]
    fn AddressType(&self) -> Result<String, zbus::Error>;

    #[zbus(property(emits_changed_signal = "false"))]
    #[allow(non_snake_case)]
    fn Alias(&self) -> Result<String, zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn set_Alias(&self, value: &str) -> Result<(), zbus::Error>;

    #[zbus(property)]
    #[allow(non_snake_case)]
    fn Icon(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    #[allow(non_snake_case)]
    fn Modalias(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    #[allow(non_snake_case)]
    fn Name(&self) -> Result<String, zbus::Error>;

    #[zbus(property)]
    #[allow(non_snake_case)]
    fn Appearance(&self) -> Result<u16, zbus::Error>;

    #[zbus(property)]
    #[allow(non_snake_case)]
    fn Class(&self) -> Result<u32, zbus::Error>;
}

#[proxy(
    interface = "org.bluez.Adapter1",
    default_service = "org.bluez",
    default_path = "/org/bluez/hciX"
)]
trait Adapter {
    async fn StartDiscovery(&self) -> Result<(), zbus::Error>;
    async fn StopDiscovery(&self) -> Result<(), zbus::Error>;
    async fn RemoveDevice(&self, device: ObjectPath<'_>) -> Result<(), zbus::Error>;
    async fn GetDiscoveryFilters(&self) -> Result<Vec<String>, zbus::Error>;
    //async fn SetDiscoveryFilters(&self, properties: HashMap<String, ...>) -> Result<(), zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn ExperimentalFeatures(&self) -> Result<Vec<String>, zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn Roles(&self) -> Result<Vec<String>, zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn UUIDS(&self) -> Result<Vec<String>, zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn Discoverable(&self) -> Result<bool, zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn set_Discoverable(&self, value: bool) -> Result<(), zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn Discovering(&self) -> Result<bool, zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn Pairable(&self) -> Result<bool, zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn set_Pairable(&self, value: bool) -> Result<(), zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn Powered(&self) -> Result<bool, zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn set_Powered(&self, value: bool) -> Result<(), zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn Address(&self) -> Result<String, zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn AddressType(&self) -> Result<String, zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn Alias(&self) -> Result<String, zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn set_Alias(&self, value: &str) -> Result<(), zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn Modalias(&self) -> Result<String, zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn Name(&self) -> Result<String, zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn Class(&self) -> Result<u32, zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn DiscoverableTimeout(&self) -> Result<u32, zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn set_DiscoverableTimeout(&self, value: u32) -> Result<(), zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn PairableTimeout(&self) -> Result<u32, zbus::Error>;
    #[zbus(property)]
    #[allow(non_snake_case)]
    fn set_PairableTimeout(&self, value: u32) -> Result<(), zbus::Error>;
}
