// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

/// Abstraction over notification channel

use std::{error::Error, fmt};

#[derive(Clone)]
/// Config provided by messenger admin
pub enum ChannelConfig {
    Slack {
        url: String,
        channel_id: String,
    },
}

impl ChannelConfig {
    pub fn notifier(&self) -> Result<Messenger, Box<dyn Error>> {
        match self {
            &ChannelConfig::Slack {
                ref url,
                ref channel_id,
            } => {
                let client = slack_hook::Slack::new(url.as_str())?;
                Ok(Messenger::Slack {
                    client: client,
                    channel_id: channel_id.clone(),
                })
            },
        }
    }
}

pub enum Messenger {
    Slack {
        client: slack_hook::Slack,
        channel_id: String,
    }
}

impl Messenger {
    pub fn sender(&self) -> Sender {
        match self {
            &Messenger::Slack {
                ref client ,
                ref channel_id,
            } => Sender::Slack {
                sender: client.clone(),
                channel_id: channel_id.clone(),
            },
        }
    }
}

/// The object can be sent another thread and can send notification message
#[derive(Clone)]
pub enum Sender {
    Slack {
        sender: slack_hook::Slack,
        channel_id: String,
    }
}

pub enum SendError {
    Slack(slack_hook::Error),
}

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            &SendError::Slack(ref e) => write!(f, "{}", e),
        }
    }
}

pub enum NotificationMessage {
    Warning(String),
    Info(String),
}

impl Sender {
    pub fn send(&self, msg: &NotificationMessage) -> Result<(), SendError> {
        match self {
            &Sender::Slack { ref sender, ref channel_id } => {
                let payload = match msg {
                    NotificationMessage::Warning(ref msg) => slack_hook::PayloadBuilder::new()
                        .text(msg.as_str())
                        .channel(channel_id.as_str())
                        .username("[e2e][error]")
                        .icon_emoji(":warning:")
                        .build()
                        .map_err(SendError::Slack)?,
                    NotificationMessage::Info(ref msg) => slack_hook::PayloadBuilder::new()
                        .text(msg.as_str())
                        .channel(channel_id.as_str())
                        .username("[e2e][info]")
                        .build()
                        .map_err(SendError::Slack)?,
                };
                sender.send(&payload)
                    .map_err(SendError::Slack)
            },
        }
    }
}
