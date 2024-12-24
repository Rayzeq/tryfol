#[derive(Debug, thiserror::Error)]
pub enum ClientError<T, U = String> {
    #[error("Could not send method call")]
    Call(#[source] anyhow::Error),
    #[error("Wrong response type: {0:?}")]
    Type(T),
    #[error("Error from server")]
    Server(#[source] U),
    #[error("The connection to the server was broken or the server closed")]
    Connection,
}
