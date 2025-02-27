use std::collections::HashMap;
use std::hash::Hash;
use std::net::SocketAddr;

use tokio::runtime::Handle;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use warp::filters::ws::Message;
use warp::filters::ws::WebSocket;
use warp::Filter;

use futures::prelude::*;
use futures::stream::SplitSink;

use crate::routes;

pub enum CtrlEvent<A, M>
where
    A: Hash + Eq,
{
    Track {
        addr: A,
        sink: SplitSink<WebSocket, Message>,
        auth: auth::Wrapper,
    },

    Forget {
        addr: A,
    },

    SendTo {
        addr: A,
        message: M,
    },

    SendAll(M),
}

pub trait IntoCtrlEvent<A, M>: Sync + Send + Clone + 'static
where
    A: Hash + Eq,
{
    fn into_ctrl_event(self) -> CtrlEvent<A, M>;
}

impl<A> IntoCtrlEvent<A, String> for String
where
    A: Hash + Eq,
{
    fn into_ctrl_event(self) -> CtrlEvent<A, String> {
        CtrlEvent::SendAll(self)
    }
}

impl<A> CtrlEvent<A, String>
where
    A: Hash + Eq + Clone,
{
    async fn recv_from_rx(mut rx: UnboundedReceiver<Self>) {
        let mut peers = HashMap::new();
        let mut discard = vec![];

        while let Some(ev) = rx.recv().await {
            for addr in &discard {
                let _ = peers.remove(addr);
            }

            discard.clear();

            match ev {
                CtrlEvent::Track { addr, sink, auth } => {
                    peers.insert(addr, (sink, auth));
                }

                CtrlEvent::Forget { ref addr } => {
                    peers.remove(addr);
                }

                CtrlEvent::SendAll(body) => {
                    for (addr, (sink, _)) in peers.iter_mut() {
                        let result = sink.send(Message::text(body.clone())).await;

                        if result.is_err() {
                            let _ = sink.close().await;
                            discard.push(addr.clone());
                        }
                    }
                }

                CtrlEvent::SendTo { addr, message } => {
                    if let Some((sink, _)) = peers.get_mut(&addr) {
                        let result = sink.send(Message::text(message.clone())).await;

                        if result.is_err() {
                            let _ = sink.close().await;
                            discard.push(addr.clone());
                        }
                    }
                }
            };
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ClientActions {
    Authenticate { token: String },
}

pub fn event_socket(
    rt_handle: Handle,
    mut event_rx: UnboundedReceiver<String>,
) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let (i_tx, i_rx) = unbounded_channel::<CtrlEvent<SocketAddr, String>>();

    let _ev = rt_handle.spawn(CtrlEvent::recv_from_rx(i_rx));

    let forwarder_fut = {
        let i_tx = i_tx.clone();

        async move {
            while let Some(st) = event_rx.recv().await {
                let _ = i_tx.send(st.into_ctrl_event());
            }
        }
    };

    let _forwarder = rt_handle.spawn(forwarder_fut);

    warp::path("ws")
        .and(warp::filters::addr::remote())
        .and(routes::global_filters::with_state(i_tx))
        .and(routes::global_filters::with_state(rt_handle))
        .and(warp::ws())
        .map(
            |addr: Option<SocketAddr>,
             i_tx: UnboundedSender<CtrlEvent<SocketAddr, String>>,
             rt_handle: Handle,
             ws: warp::ws::Ws| {
                ws.on_upgrade(move |websocket| async move {
                    let addr = match addr {
                        Some(addr) => addr,
                        None => return,
                    };

                    let (m_tx, mut m_rx) = unbounded_channel::<(SocketAddr, Message)>();
                    let (ws_tx, mut ws_rx) = websocket.split();

                    'auth_loop: while let Some(Ok(x)) = ws_rx.next().await {
                        if x.is_text() {
                            if let Ok(ClientActions::Authenticate { token }) =
                                serde_json::from_slice(x.as_bytes())
                            {
                                if let Ok(token_data) = auth::jwt_check(token) {
                                    let _ = i_tx.send(CtrlEvent::Track {
                                        addr,
                                        sink: ws_tx,
                                        auth: auth::Wrapper(token_data),
                                    });

                                    let _ = i_tx.send(CtrlEvent::SendTo {
                                        addr,
                                        message: events::Message {
                                            id: -1,
                                            event_type: events::PushEventType::EventAuthOk,
                                        }
                                        .to_string(),
                                    });

                                    break 'auth_loop;
                                }
                            }

                            let _ = i_tx.send(CtrlEvent::SendTo {
                                addr,
                                message: events::Message {
                                    id: -1,
                                    event_type: events::PushEventType::EventAuthErr,
                                }
                                .to_string(),
                            });
                        }
                    }

                    let m_tx = m_tx.clone();

                    rt_handle.spawn(async move {
                        while let Some(Ok(message)) = ws_rx.next().await {
                            if m_tx.send((addr, message)).is_err() {
                                break;
                            }
                        }

                        i_tx.send(CtrlEvent::Forget { addr })
                    });

                    'outer: loop {
                        tokio::select! {
                            biased;
                            _ = tokio::signal::ctrl_c() => {
                                break 'outer;
                            }

                            message = m_rx.recv() => {
                                let (_addr, _message) = match message {
                                    Some(p) => p,
                                    None => break 'outer,
                                };
                            }

                            else => break 'outer,
                        }
                    }

                    tokio::task::yield_now().await;
                })
            },
        )
}
