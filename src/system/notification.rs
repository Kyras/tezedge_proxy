// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

/// Abstraction over notification channel

use std::{error::Error, fmt, thread};

#[derive(Clone)]
/// Config provided by messenger admin
pub enum ChannelConfig {
    Slack {
        token: String,
        channel_id: String,
    },
}

impl ChannelConfig {
    pub fn notifier(&self) -> Result<Messenger, Box<dyn Error>> {
        match self {
            &ChannelConfig::Slack {
                ref token,
                ref channel_id,
            } => {
                let client = slack::RtmClient::login(token.as_str())?;
                let sender = client.sender().clone();
                let event_loop = thread::spawn(move || client.run(&mut SlackHandler).map_err(|e| e.to_string()));
                Ok(Messenger::Slack {
                    event_loop: event_loop,
                    sender: sender,
                    channel_id: channel_id.clone(),
                })
            },
        }
    }
}

pub struct SlackHandler;

impl slack::EventHandler for SlackHandler {
    fn on_event(&mut self, cli: &slack::RtmClient, event: slack::Event) {
        let _ = (cli, event);
    }

    fn on_close(&mut self, cli: &slack::RtmClient) {
        let _ = cli;
    }

    fn on_connect(&mut self, cli: &slack::RtmClient) {
        let _ = cli;
    }
}

pub enum Messenger {
    Slack {
        event_loop: thread::JoinHandle<Result<(), String>>,
        sender: slack::Sender,
        channel_id: String,
    }
}

impl Messenger {
    pub fn sender(&self) -> Sender {
        match self {
            &Messenger::Slack {
                event_loop: _,
                ref sender ,
                ref channel_id,
            } => Sender::Slack {
                sender: sender.clone(),
                channel_id: channel_id.clone(),
            },
        }
    }
}

/// The object can be sent another thread and can send notification message
#[derive(Clone)]
pub enum Sender {
    Slack {
        sender: slack::Sender,
        channel_id: String,
    }
}

pub enum SendError {
    Slack(slack::Error),
}

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            &SendError::Slack(ref e) => write!(f, "{}", e),
        }
    }
}

impl Sender {
    pub fn send(&self, msg: &String) -> Result<(), SendError> {
        match self {
            &Sender::Slack { ref sender, ref channel_id } =>
                sender.send_message(channel_id.as_str(), msg.as_str())
                    .map(|_| ())
                    .map_err(SendError::Slack),
        }
    }
}
