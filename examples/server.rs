use async_trait::async_trait;
use log::{info, LevelFilter};
use russh::{
    server::{Auth, Msg, Session},
    Channel, ChannelId,
};
use russh_keys::key::KeyPair;
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::Mutex;

#[derive(Clone)]
struct Server {
    clients: Arc<Mutex<HashMap<(usize, ChannelId), Channel<Msg>>>>,
    id: usize,
}

impl Server {
    pub async fn get_channel(&mut self, channel_id: ChannelId) -> Channel<Msg> {
        let mut clients = self.clients.lock().await;
        clients.remove(&(self.id, channel_id)).unwrap()
    }
}

impl russh::server::Server for Server {
    type Handler = Self;

    fn new_client(&mut self, _: Option<SocketAddr>) -> Self::Handler {
        let s = self.clone();
        self.id += 1;
        s
    }
}

#[async_trait]
impl russh::server::Handler for Server {
    type Error = anyhow::Error;

    async fn auth_password(self, user: &str, password: &str) -> Result<(Self, Auth), Self::Error> {
        info!("credentials: {}, {}", user, password);
        Ok((self, Auth::Accept))
    }

    async fn channel_open_session(
        mut self,
        channel: Channel<Msg>,
        session: Session,
    ) -> Result<(Self, bool, Session), Self::Error> {
        {
            let mut clients = self.clients.lock().await;
            clients.insert((self.id, channel.id()), channel);
        }
        Ok((self, true, session))
    }

    async fn subsystem_request(
        mut self,
        channel_id: ChannelId,
        name: &str,
        mut session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        info!("subsystem: {}", name);

        if name == "sftp" {
            let channel = self.get_channel(channel_id).await;
            russh_sftp::server::run(channel).await;

            session.channel_success(channel_id);
        } else {
            session.channel_failure(channel_id);
        }

        Ok((self, session))
    }
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .init();

    let config = russh::server::Config {
        auth_rejection_time: Duration::from_secs(3),
        auth_rejection_time_initial: Some(Duration::from_secs(0)),
        keys: vec![KeyPair::generate_ed25519().unwrap()],
        connection_timeout: Some(Duration::from_secs(3600)),
        ..Default::default()
    };

    let server = Server {
        clients: Arc::new(Mutex::new(HashMap::new())),
        id: 0,
    };

    russh::server::run(Arc::new(config), ("0.0.0.0", 22), server)
        .await
        .unwrap();
}
