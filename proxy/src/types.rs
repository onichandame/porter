use std::error::Error as StdError;
use tokio::sync::oneshot;

pub type Error = Box<dyn StdError + Send + Sync>;

pub enum Request {
    ProxyStatus(i32, oneshot::Sender<Response>),
    CreateProxy {
        host: String,
        port: i32,
        remote_addr: String,
        response_channel: oneshot::Sender<Response>,
    },
    DeleteProxy(i32, Option<oneshot::Sender<Response>>),
    Terminate,
}

pub type Response = Result<(), Error>;
