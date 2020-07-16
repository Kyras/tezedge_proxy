// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use tracing::error;
use tokio::sync::mpsc::{
    UnboundedSender, unbounded_channel,
};
use async_trait::async_trait;
use crate::system::SystemSettings;
use crate::messages::p2p_message::P2pMessage;
use crate::storage::MessageStore;

#[cfg(not(debug_assertions))]
use tracing::field::display;

type ProcessorTrait = dyn Processor + Sync + Send + 'static;

#[async_trait]
pub trait Processor {
    async fn process(&mut self, msg: P2pMessage);
}

pub fn spawn_processor(settings: SystemSettings) -> UnboundedSender<P2pMessage> {
    let (sender, mut receiver) = unbounded_channel::<P2pMessage>();

    tokio::spawn(async move {
        let mut processors: Vec<Box<ProcessorTrait>> = Default::default();
        processors.push(Box::new(DatabaseProcessor::new(settings.storage.clone())));
        loop {
            if let Some(message) = receiver.recv().await {
                for processor in processors.iter_mut() {
                    processor.process(message.clone()).await;
                }
            } else {
                error!("p2p processing channel closed unexpectedly");
                break;
            }
        }
    });

    sender
}

struct DatabaseProcessor {
    store: MessageStore,
    sender: UnboundedSender<P2pMessage>,
}

impl DatabaseProcessor {
    pub fn new(store: MessageStore) -> Self {
        let ret = Self {
            sender: Self::start_database_task(store.clone()),
            store,
        };

        ret
    }

    fn start_database_task(store: MessageStore) -> UnboundedSender<P2pMessage> {
        let (sender, mut receiver) = unbounded_channel::<P2pMessage>();
        tokio::spawn(async move {
            loop {
                if let Some(mut msg) = receiver.recv().await {
                    if let Err(err) = store.p2p().store_message(&mut msg) {
                        error!(error = display(&err), "failed to store message");
                    }
                }
            }
        });
        sender
    }
}

#[async_trait]
impl Processor for DatabaseProcessor {
    async fn process(&mut self, mut msg: P2pMessage) {
        loop {
            if let Err(err) = self.sender.send(msg) {
                error!(error = display(&err), "database channel closed abruptly");
                msg = err.0;
                self.sender = Self::start_database_task(self.store.clone());
            } else {
                return;
            }
        }
    }
}