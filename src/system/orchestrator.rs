// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use tokio::sync::mpsc::{
    UnboundedSender, unbounded_channel,
};
use tracing::{trace, error};
use std::{
    collections::{HashMap, hash_map::Entry},
};
use crate::{
    system::prelude::*,
    messages::tcp_packet::Packet,
};
use crate::system::processor::spawn_processor;

#[cfg(not(debug_assertions))]
use tracing::field::display;

pub fn spawn_packet_orchestrator(settings: SystemSettings) -> UnboundedSender<Packet> {
    let (sender, mut receiver) = unbounded_channel::<Packet>();

    tokio::spawn(async move {
        let mut packet_processors = HashMap::new();
        let message_processor = spawn_processor(settings.clone());
        let settings = settings;
        let store = settings.storage.clone();
        loop {
            if let Some(packet) = receiver.recv().await {
                let packet: Packet = packet;
                let entry = if packet_processors.contains_key(&packet.destination_address()) {
                    packet_processors.entry(packet.destination_address())
                } else {
                    packet_processors.entry(packet.source_address())
                };

                let mut prev_value;
                let mut occupied_entry;
                let processor;

                let src = packet.source_address();
                let dst = packet.destination_address();
                let settings = settings.clone();

                // Packet is closing connection
                if packet.is_closing() {
                    if let Entry::Occupied(entry) = entry {
                        // There is still running processor, this packet will notify it to shut down
                        prev_value = entry.remove();
                        processor = &mut prev_value;
                    } else {
                        // Processor is already shut down, ignore the packet
                        continue;
                    }
                } else if packet.is_opening() {
                    // Is packet is opening new connection
                    let message_processor = message_processor.clone();
                    processor = entry.or_insert_with(move || {
                        // If processor does not exists, create new one
                        trace!(
                            source = display(src),
                            destination = display(dst),
                            "spawning p2p parser"
                        );
                        spawn_p2p_parser(src, message_processor, settings)
                    });
                } else {
                    if let Entry::Occupied(entry) = entry {
                        occupied_entry = entry;
                        processor = occupied_entry.get_mut();
                    } else {
                        if packet.payload().len() > 0 {
                            trace!(
                                source = display(src),
                                destination = display(dst),
                                "processor does not exists"
                            );
                        }
                        continue;
                    }
                };

                store.stat().capture_data(packet.payload().len());
                match processor.send(packet) {
                    Ok(()) => {
                        trace!("sent packet to p2p");
                    }
                    Err(_) => {
                        error!("p2p parser channel closed abruptly");
                    }
                }
            } else {
                error!("packet consuming channel closed unexpectedly");
                break;
            }
        }
    });

    return sender;
}