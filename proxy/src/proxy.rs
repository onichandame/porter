use std::error::Error;
use tokio::{
    io::{self, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
    task::JoinHandle,
};

pub type ProxyResult = Result<(), Box<dyn Error + Send + Sync>>;

pub struct Proxy {
    task: JoinHandle<ProxyResult>,
}

impl Proxy {
    pub fn new(
        host: &str,
        port: i32,
        remote_addr: &str,
        close_channel: oneshot::Sender<i32>,
    ) -> Self {
        let remote_addr = remote_addr.to_owned();
        let local_addr = format!("{}:{}", host, port).to_owned();
        Self {
            task: tokio::spawn(async move {
                let listener = TcpListener::bind(local_addr).await?;
                while let Ok((mut inbound, _)) = listener.accept().await {
                    if let Ok(mut outbound) = TcpStream::connect(remote_addr.clone()).await {
                        let (mut inread, mut inwrite) = inbound.split();
                        let (mut outread, mut outwrite) = outbound.split();
                        let local_to_remote = async {
                            io::copy(&mut inread, &mut outwrite).await?;
                            outwrite.shutdown().await
                        };
                        let remote_to_local = async {
                            io::copy(&mut outread, &mut inwrite).await?;
                            inwrite.shutdown().await
                        };
                        if let Err(_e) = tokio::try_join!(local_to_remote, remote_to_local) {
                            println!("proxy failed for port {}", port);
                        }
                    }
                }
                close_channel.send(port).map_err(|_| {
                    format!(
                        "failed to signal proxy manager about the end of proxy on port {}",
                        port
                    )
                })?;
                Ok(())
            }),
        }
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        self.task.abort();
    }
}
