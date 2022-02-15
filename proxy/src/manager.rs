use std::{collections::HashMap, time};
use tokio::{
    runtime,
    sync::{mpsc, oneshot, watch},
    task::JoinHandle,
};

use super::proxy::Proxy;
use crate::types::{Error, Request, Response};

pub struct ProxyManager {
    sender: mpsc::Sender<Request>,
    daemon: JoinHandle<()>,
    ready_watcher: watch::Receiver<bool>,
}

impl ProxyManager {
    const REQUEST_ERROR: &'static str =
        "proxy manager request cannot be sent, program is likely broken";
    const PROXY_NOT_READY: &'static str = "proxy not ready";
    const PROXY_ALREADY_STARTED: &'static str = "proxy already started";
    const REQUEST_TIMEOUT: time::Duration = time::Duration::from_secs(5);

    pub fn new() -> Self {
        let (sender, mut receiver) = mpsc::channel::<Request>(8);
        let req_sender = sender.clone();
        let (ready_sender, ready_receiver) = watch::channel(false);
        Self {
            sender,
            ready_watcher: ready_receiver,
            daemon: tokio::spawn(async move {
                let mut proxy_map = HashMap::<i32, Proxy>::new();
                ready_sender.send(true).unwrap();
                while let Some(ev) = receiver.recv().await {
                    match ev {
                        Request::ProxyStatus(port, res) => {
                            if proxy_map.contains_key(&port) {
                                res.send(Ok(())).ok();
                            } else {
                                res.send(Err(ProxyManager::PROXY_NOT_READY.into())).ok();
                            }
                        }
                        Request::CreateProxy {
                            host,
                            port,
                            remote_addr,
                            response_channel,
                        } => {
                            if proxy_map.contains_key(&port) {
                                response_channel
                                    .send(Err(ProxyManager::PROXY_ALREADY_STARTED.into()))
                                    .ok();
                            } else {
                                let (close_sender, mut close_receiver) = oneshot::channel::<i32>();
                                let request_sender = req_sender.clone();
                                proxy_map.insert(
                                    port,
                                    Proxy::new(&host, port, &remote_addr, close_sender),
                                );
                                tokio::spawn(async move {
                                    while let Ok(port) = close_receiver.try_recv() {
                                        request_sender
                                            .send(Request::DeleteProxy(port, None))
                                            .await
                                            .map_err(|_| ProxyManager::REQUEST_ERROR)
                                            .unwrap();
                                    }
                                });
                            }
                        }
                        Request::DeleteProxy(port, res_maybe) => {
                            proxy_map.remove(&port);
                            if let Some(res) = res_maybe {
                                res.send(Ok(())).ok();
                            }
                        }
                        Request::Terminate => {
                            receiver.close();
                        }
                    };
                }
                ready_sender.send(false).unwrap();
            }),
        }
    }

    pub async fn ready(&self) -> bool {
        let receiver = self.ready_watcher.clone();
        let res = *receiver.borrow();
        res
    }

    pub async fn wait_for_ready(&self) {
        let mut receiver = self.ready_watcher.clone();
        let ready = *receiver.borrow_and_update();
        if !ready {
            while let Ok(_) = receiver.changed().await {
                let ready = *receiver.borrow();
                if ready {
                    break;
                }
            }
        }
    }

    pub async fn proxy_ready(&self, port: i32) -> bool {
        let (res_sender, res_receiver) = oneshot::channel();
        self.send_request(Request::ProxyStatus(port, res_sender))
            .await;
        if let Ok(Ok(_)) = self.wait_for_response(res_receiver).await {
            true
        } else {
            false
        }
    }

    pub async fn create_proxy(
        &self,
        host: String,
        port: i32,
        remote_addr: String,
    ) -> Result<(), Error> {
        let (res_sender, res_receiver) = oneshot::channel();
        self.send_request(Request::CreateProxy {
            host,
            port,
            remote_addr,
            response_channel: res_sender,
        })
        .await;
        Ok(self.wait_for_response(res_receiver).await??)
    }

    pub async fn delete_proxy(&self, port: i32) -> Result<(), Error> {
        let (res_sender, res_receiver) = oneshot::channel();
        self.send_request(Request::DeleteProxy(port, Some(res_sender)))
            .await;
        Ok(self.wait_for_response(res_receiver).await??)
    }

    async fn send_request(&self, req: Request) {
        let req_sender = self.sender.clone();
        req_sender
            .send(req)
            .await
            .map_err(|_| ProxyManager::REQUEST_ERROR)
            .unwrap();
    }

    async fn wait_for_response(
        &self,
        mut receiver: oneshot::Receiver<Response>,
    ) -> Result<Response, Error> {
        Ok(tokio::time::timeout(
            ProxyManager::REQUEST_TIMEOUT,
            async move { receiver.try_recv() },
        )
        .await??)
    }
}

impl Drop for ProxyManager {
    fn drop(&mut self) {
        runtime::Handle::current()
            .block_on(tokio::time::timeout(time::Duration::from_secs(3), async {
                self.send_request(Request::Terminate).await;
                let mut receiver = self.ready_watcher.clone();
                let ready = *receiver.borrow_and_update();
                if ready {
                    while let Ok(_) = receiver.changed().await {
                        let ready = *receiver.borrow();
                        if !ready {
                            break;
                        }
                    }
                }
            }))
            .ok();
        self.daemon.abort();
    }
}
