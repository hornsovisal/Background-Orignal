use std::{
    pin::Pin,
    task::{ready, Context, Poll},
};

use futures::{Stream, StreamExt};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
};
use tokio_tungstenite::WebSocketStream;

use crate::{error::Error, events::Events};

pub(crate) trait AsyncSocket: AsyncRead + AsyncWrite + Unpin + Send + 'static {}
impl<T> AsyncSocket for T where T: AsyncRead + AsyncWrite + Unpin + Send + 'static {}

pub struct TcpService {
    listener: TcpListener,
}

impl TcpService {
    pub async fn new(addr: String) -> Self {
        let listener = TcpListener::bind(&addr).await.unwrap();
        Self { listener }
    }
}
impl Stream for TcpService {
    type Item = Result<TcpStream, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.listener.poll_accept(cx)? {
            Poll::Ready((stream, _)) => Poll::Ready(Some(Ok(stream))),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct WsStream {
    ws_stream: WebSocketStream<Box<dyn AsyncSocket>>,
}

impl WsStream {
    pub async fn new(stream: impl AsyncSocket) -> Result<Self, Error> {
        let socket: Box<dyn AsyncSocket> = Box::new(stream);

        Ok(Self {
            ws_stream: tokio_tungstenite::accept_async(socket).await?,
        })
    }
}

impl Stream for WsStream {
    type Item = Result<Events, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match ready!(self.ws_stream.poll_next_unpin(cx)?) {
            Some(msg) => {
                if let Ok(msg) = msg.to_text() {
                    let event = match serde_json::from_str(msg) {
                        Ok(events) => events,
                        Err(_e) => {
                            // panic!("{_e}");
                            Events::Other(msg.to_string())},
                    };
                    Poll::Ready(Some(Ok(event)))
                } else {
                    Poll::Ready(Some(Err(Error::InvalidMessage)))
                }
            }
            None => Poll::Ready(None),
        }
    }
}
