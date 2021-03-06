use std::{
    path::Path,
    net::SocketAddr,
    collections::HashMap,
};
use tokio::sync::mpsc;
use tezos_conversation::Identity;
use sniffer::{EventId, SocketId};

use crate::{
    messages::p2p_message::{SourceType, P2pMessage},
    system::SystemSettings,
};
use super::{
    connection::Connection,
    connection_parser,
    report::{Report, ConnectionReport},
};

pub struct Message {
    pub payload: Vec<u8>,
    pub incoming: bool,
    pub counter: u64,
    pub event_id: EventId,
}

#[derive(Debug, Clone, Copy)]
pub enum Command {
    GetReport,
    Terminate,
}

pub struct ProcessingConnectionResult {
    pub have_identity: bool,
}

pub struct Parser {
    identity_cache: Option<Identity>,
    tx_report: mpsc::Sender<Report>,
    rx_connection_report: mpsc::Receiver<ConnectionReport>,
    tx_connection_report: mpsc::Sender<ConnectionReport>,
    working_connections: HashMap<SocketId, Connection>,
    closed_connections: Vec<ConnectionReport>,
}

impl Parser {
    pub fn new(tx_report: mpsc::Sender<Report>) -> Self {
        let (tx_connection_report, rx_connection_report) = mpsc::channel(0x100);
        Parser {
            identity_cache: None,
            tx_report,
            rx_connection_report,
            tx_connection_report,
            working_connections: HashMap::new(),
            closed_connections: Vec::new(),
        }
    }

    pub async fn execute(&mut self, command: Command) {
        let report = self.execute_inner(command).await;
        match self.tx_report.send(report).await {
            Ok(()) => (),
            Err(_) => (),
        }
    }

    async fn execute_inner(&mut self, command: Command) -> Report {
        self.working_connections.iter_mut().for_each(|(_, c)| c.send_command(command));

        match command {
            Command::GetReport => {
                let mut working_connections = Vec::new();
                while let Ok(report) = self.rx_connection_report.try_recv() {
                    working_connections.push(report);
                }
                let mut closed_connections = self.closed_connections.clone();
                closed_connections.iter_mut().for_each(|report| report.metadata = None);
        
                Report::prepare(closed_connections, working_connections)        
            },
            Command::Terminate => {
                debug_assert!(self.rx_connection_report.try_recv().is_err(), "should not have reports to receive");
                // TODO: this is the final report, compare it with ocaml report
                Report::prepare(self.closed_connections.clone(), Vec::new())        
            }
        }
    }

    /// Try to load identity lazy from one of the well defined paths
    fn load_identity() -> Result<Identity, ()> {
        let identity_paths = [
            "/tmp/volume/identity.json".to_string(),
            "/tmp/volume/data/identity.json".to_string(),
            format!("{}/.tezos-node/identity.json", std::env::var("HOME").unwrap()),
        ];

        for path in &identity_paths {
            if !Path::new(path).is_file() {
                continue;
            }
            match Identity::from_path(path.clone()) {
                Ok(identity) => {
                    tracing::info!(file_path = tracing::field::display(&path), "loaded identity");
                    return Ok(identity);
                },
                Err(err) => {
                    tracing::warn!(error = tracing::field::display(&err), "identity file does not contains valid identity");
                },
            }
        }

        Err(())
    }

    fn try_load_identity(&mut self) -> Option<Identity> {
        if self.identity_cache.is_none() {
            self.identity_cache = Self::load_identity().ok();
        }
        self.identity_cache.clone()
    }

    pub async fn process_connect(
        &mut self,
        settings: &SystemSettings,
        id: EventId,
        remote_address: SocketAddr,
        db: &mpsc::UnboundedSender<P2pMessage>,
        source_type: SourceType,
    ) -> ProcessingConnectionResult {
        let have_identity = if let Some(identity) = self.try_load_identity() {
            let parser = connection_parser::Parser {
                identity,
                settings: settings.clone(),
                source_type,
                remote_address,
                id: id.socket_id.clone(),
                db: db.clone(),
            };
            let connection = Connection::spawn(self.tx_connection_report.clone(), parser);
            if let Some(old) = self.working_connections.insert(id.socket_id, connection) {
                match old.join().await {
                    Ok(report) => self.closed_connections.push(report),
                    Err(error) => tracing::error!(
                        error = tracing::field::display(&error),
                        msg = "P2P failed to join task which was processing the connection",
                    ),
                }
            }
            true
        } else {
            false
        };
        ProcessingConnectionResult { have_identity }
    }

    pub async fn process_close(&mut self, event_id: EventId) {
        // can safely drop the old connection
        if let Some(old) = self.working_connections.remove(&event_id.socket_id) {
            match old.join().await {
                Ok(report) => self.closed_connections.push(report),
                Err(error) => tracing::error!(
                    error = tracing::field::display(&error),
                    msg = "P2P failed to join task which was processing the connection",
                ),
            }
        }
    }

    pub fn process_data(&mut self, message: Message) {
        match self.working_connections.get_mut(&message.event_id.socket_id) {
            Some(connection) => connection.process(message),
            None => {
                // It is possible due to race condition,
                // when we consider to ignore connection, we do not create
                // connection structure in userspace, and send to bpf code 'ignore' command.
                // However, the bpf code might already sent us some message.
                // It is safe to ignore this message if it goes right after appearing
                // new P2P connection which we ignore.
                tracing::warn!(
                    id = tracing::field::display(&message.event_id),
                    msg = "P2P receive message for absent connection",
                )
            },
        }
    }
}
