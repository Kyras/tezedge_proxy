// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use tracing::{error, info};
use tokio::{
    io,
    net::UdpSocket,
};
use crate::system::SystemSettings;
use crate::messages::log_message::LogMessage;

#[cfg(not(debug_assertions))]
use tracing::field::display;

pub async fn syslog_producer(settings: SystemSettings) -> io::Result<()> {
    let socket = UdpSocket::bind(("0.0.0.0", settings.syslog_port)).await?;
    info!(port = 13131, "started listening for syslog");
    tokio::spawn(async move {
        let mut socket = socket;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = socket.recv(&mut buffer)
                .await.unwrap(); // This unwrap is safe, as socket was bound before reading
            let datagram = &buffer[..read];
            if let Ok(log) = std::str::from_utf8(&datagram) {
                let msg = syslog_loose::parse_message(log);
                let mut log_msg = LogMessage::from(msg);
                if let Err(err) = settings.storage.log().store_message(&mut log_msg) {
                    error!(error = display(&err), "failed to store log");
                }
            }
        }
    });
    Ok(())
}