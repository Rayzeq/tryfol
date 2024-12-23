use ipc_macros::{Read, Write};

/// Packet going from the client to the server
#[derive(Debug, Clone, Read, Write)]
pub struct Clientbound<T> {
    /// A unique id identifying this method call
    pub call_id: u64,
    pub payload: T,
}

/// Packet going from the server to the client
#[derive(Debug, Clone, Read, Write)]
pub struct Serverbound<T> {
    /// Id of the method call that this packet is a response to
    pub call_id: u64,
    pub payload: T,
}
