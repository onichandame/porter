use std::{collections::HashMap, time};
use tokio::{
    runtime,
    sync::{mpsc, oneshot, watch},
    task::JoinHandle,
};

use super::proxy::Proxy;
use crate::types::{Error, Request, Response};

/// A TCP proxy connecting local ports to remote addresses.
/// Currently only 1-to-1 relation is supported, i.e. no loadbalancing.
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
    const MANAGER_TERMINATING: &'static str = "proxy manager is terminating";
    const REQUEST_TIMEOUT: time::Duration = time::Duration::from_secs(5);

    /// Create a new Proxy Manager.
    ///
    /// A background async task is created and kept running until the instance is dropped.
    ///
    /// This method MUST be called within a tokio runtime!!!
    pub fn new() -> Self {
        let (sender, mut receiver) = mpsc::channel::<Request>(8);
        let req_sender = sender.clone();
        let (ready_sender, ready_receiver) = watch::channel(false);
        Self {
            sender,
            ready_watcher: ready_receiver,
            daemon: tokio::spawn(async move {
                let mut proxy_map = HashMap::<i32, Proxy>::new();
                let mut watcher_map = HashMap::<i32, JoinHandle<()>>::new();
                let mut terminating = false;
                ready_sender.send(true).unwrap();
                'ev_loop: while let Some(ev) = receiver.recv().await {
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
                            if terminating {
                                response_channel
                                    .send(Err(ProxyManager::MANAGER_TERMINATING.into()))
                                    .unwrap();
                                continue 'ev_loop;
                            }
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
                                watcher_map.insert(
                                    port,
                                    tokio::spawn(async move {
                                        if let Ok(port) = close_receiver.try_recv() {
                                            request_sender
                                                .send(Request::DeleteProxy(port, None))
                                                .await
                                                .map_err(|_| ProxyManager::REQUEST_ERROR)
                                                .unwrap();
                                        }
                                    }),
                                );
                            }
                        }
                        Request::DeleteProxy(port, response_maybe) => {
                            proxy_map.remove(&port);
                            if let Some(handle) = watcher_map.remove(&port) {
                                handle.abort();
                            }
                            if let Some(response) = response_maybe {
                                response.send(Ok(())).ok();
                            }
                        }
                        Request::Terminate => {
                            terminating = true;
                            let request_sender = req_sender.clone();
                            for (port, _) in &proxy_map {
                                request_sender
                                    .send(Request::DeleteProxy(*port, None))
                                    .await
                                    .map_err(|_| ProxyManager::REQUEST_ERROR)
                                    .unwrap();
                            }
                            receiver.close();
                        }
                    };
                }
                ready_sender.send(false).unwrap();
            }),
        }
    }

    /// Check if the proxy manager is currently functioning.
    ///
    /// Returns true if ok, false if the manager is shut down.
    pub async fn ready(&self) -> bool {
        let receiver = self.ready_watcher.clone();
        let res = *receiver.borrow();
        res
    }

    /// Wait for the proxy manager to be ready.
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

    /// Check if the specified proxy is ready.
    ///
    /// - port: the port on which the proxy is meant to listen.
    ///
    /// Returns true if the proxy is active, false otherwise.
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

    /// Create a proxy listening on a port and connecting to a remote address.
    ///
    /// - host: the host from which the proxy should accept the requests. e.g. "127.0.0.1" for only
    /// accepting requests from the localhost.
    /// - port: the local port on which the proxy should listen.
    /// - remote_addr: the complete address of the remote service. e.g. google.com:80
    ///
    /// Returns Ok if the proxy is successfully created.
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
        match self.wait_for_response(res_receiver).await? {
            Ok(_) => Ok(()),
            Err(e) => {
                self.delete_proxy(port).await.ok();
                Err(e)
            }
        }
    }

    /// Delete a proxy.
    ///
    /// Returns Ok if the proxy is not found or successfully deleted.
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
