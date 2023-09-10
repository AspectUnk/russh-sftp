use std::{sync::Arc, time::Duration};

use bytes::Bytes;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{mpsc, oneshot, Mutex},
    time::timeout,
};

use crate::{
    error::Error,
    protocol::{Extended, ExtendedReply, Init, Packet, Version},
};

use super::{run, Handler};

pub type SharedData = Mutex<Vec<(u32, oneshot::Sender<Packet>)>>;

pub(crate) struct SessionInner {
    version: Option<u32>,
    requests: Arc<SharedData>,
}

impl SessionInner {
    pub async fn get_request(&mut self, id: u32) -> Result<Option<oneshot::Sender<Packet>>, Error> {
        if id != 0 && self.version.is_none() {
            return Err(Error::UnexpectedBehavior("unexpected packet".to_owned()));
        } else if id == 0 && self.version.is_some() {
            return Err(Error::UnexpectedBehavior(
                "duplicate version packet".to_owned(),
            ));
        }

        let mut requests = self.requests.lock().await;
        match requests.iter().position(|&(i, _)| i == id) {
            Some(idx) => Ok(Some(requests.remove(idx).1)),
            None => Ok(None),
        }
    }
}

#[async_trait]
impl Handler for SessionInner {
    type Error = Error;

    async fn version(&mut self, version: Version) -> Result<(), Self::Error> {
        if let Some(tx) = self.get_request(0).await? {
            self.version = Some(version.version);
            let _ = tx.send(version.into_packet());
        }

        Ok(())
    }

    async fn extended_reply(&mut self, reply: ExtendedReply) -> Result<(), Self::Error> {
        if let Some(tx) = self.get_request(reply.id).await? {
            let _ = tx.send(reply.into_packet());
        }

        Ok(())
    }
}

pub struct RawSftpSession {
    tx: mpsc::UnboundedSender<Bytes>,
    requests: Arc<SharedData>,
    last_req_id: u32,
}

impl RawSftpSession {
    pub fn new<S>(stream: S) -> Self
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let arc = Arc::new(Mutex::new(Vec::new()));
        let inner = SessionInner {
            version: None,
            requests: arc.clone(),
        };

        Self {
            tx: run(stream, inner),
            requests: arc,
            last_req_id: 0,
        }
    }

    async fn exchange(&self, id: u32, packet: Packet) -> Result<Packet, Error> {
        let (tx, rx) = oneshot::channel();
        
        self.requests.lock().await.push((id, tx));
        self.tx.send(Bytes::try_from(packet)?)?;
        
        // todo: remove from requests
        Ok(timeout(Duration::from_secs(10), rx).await??)
    }

    pub async fn init(&self) -> Result<Version, Error> {
        let result = self.exchange(0, Init::default().into_packet()).await?;
        if let Packet::Version(version) = result {
            Ok(version)
        } else {
            Err(Error::UnexpectedBehavior("unexpected pkt".to_owned()))
        }
    }

    pub async fn extended(
        &mut self,
        request: String,
        data: Vec<u8>,
    ) -> Result<ExtendedReply, Error> {
        self.last_req_id += 1;

        let extended = Extended {
            id: self.last_req_id,
            request,
            data,
        };
        let result = self.exchange(self.last_req_id, extended.into_packet()).await?;
        if let Packet::ExtendedReply(reply) = result {
            Ok(reply)
        } else {
            Err(Error::UnexpectedBehavior("unexpected pkt".to_owned()))
        }
    }
}
