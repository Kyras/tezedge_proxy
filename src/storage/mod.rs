// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

mod p2p_storage;
mod log_storage;
mod rpc_storage;
mod stat_storage;
mod metric_storage;
mod secondary_index;

pub use p2p_storage::{P2pStore, P2pFilters};
pub use log_storage::{LogStore, LogFilters};
pub use rpc_storage::{RpcStore, RpcFilters};
pub use metric_storage::{MetricStore, MetricFilters};
pub(crate) use p2p_storage::secondary_indexes as p2p_indexes;
pub(crate) use log_storage::secondary_indexes as log_indexes;
pub(crate) use rpc_storage::secondary_indexes as rpc_indexes;

use rocksdb::{DB, ColumnFamilyDescriptor};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
    net::IpAddr,
};
use storage::persistent::KeyValueSchema;
use crate::storage::stat_storage::StatStore;

#[derive(Clone)]
pub struct MessageStore {
    p2p_db: P2pStore,
    log_db: LogStore,
    rpc_db: RpcStore,
    stat_db: Arc<StatStore>,
    metric_db: MetricStore,
    raw_db: Arc<DB>,
    max_db_size: Option<u64>,
}

impl MessageStore {
    pub fn new(db: Arc<DB>) -> Self {
        Self {
            p2p_db: P2pStore::new(db.clone()),
            log_db: LogStore::new(db.clone()),
            rpc_db: RpcStore::new(db.clone()),
            stat_db: Arc::new(StatStore::new()),
            metric_db: MetricStore::new(db.clone()),
            raw_db: db,
            max_db_size: None,
        }
    }

    pub fn p2p(&self) -> &P2pStore {
        &self.p2p_db
    }

    pub fn log(&self) -> &LogStore {
        &self.log_db
    }

    pub fn rpc(&self) -> &RpcStore {
        &self.rpc_db
    }

    pub fn stat(&self) -> &StatStore {
        &self.stat_db
    }

    pub fn metric(&self) -> &MetricStore {
        &self.metric_db
    }
}

pub fn cfs() -> Vec<ColumnFamilyDescriptor> {
    vec![
        P2pStore::descriptor(),
        LogStore::descriptor(),
        RpcStore::descriptor(),
        p2p_indexes::RemoteAddrIndex::descriptor(),
        p2p_indexes::TypeIndex::descriptor(),
        p2p_indexes::IncomingIndex::descriptor(),
        p2p_indexes::SourceTypeIndex::descriptor(),
        log_indexes::LevelIndex::descriptor(),
        log_indexes::TimestampIndex::descriptor(),
        rpc_indexes::RemoteAddrIndex::descriptor(),
    ]
}

pub fn get_ts() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
}

fn encode_address(addr: &IpAddr) -> u128 {
    match addr {
        &IpAddr::V6(addr) => u128::from(addr),
        &IpAddr::V4(addr) => u32::from(addr) as u128,
    }
}

#[allow(dead_code)]
fn decode_address(value: u128) -> IpAddr {
    use std::net::{Ipv4Addr, Ipv6Addr};
    if value & 0xFFFFFFFFFFFFFFFFFFFFFFFF00000000 == 0 {
        IpAddr::V4(Ipv4Addr::from(value as u32))
    } else {
        IpAddr::V6(Ipv6Addr::from(value))
    }
}

pub fn dissect(mut number: u32) -> Vec<u32> {
    let mut ret: Vec<u32> = Vec::with_capacity(number.count_ones() as usize);
    while number != 0 {
        let value = 0x1 << number.trailing_zeros();
        number = !value & number;
        ret.push(value);
    }
    ret
}

pub mod sorted_intersect {
    use std::cmp::Ordering;

    pub fn sorted_intersect<I>(mut iters: Vec<I>, limit: usize) -> Vec<I::Item>
        where
            I: Iterator,
            I::Item: Ord,
    {
        let mut ret = Default::default();
        if iters.len() == 0 {
            return ret;
        } else if iters.len() == 1 {
            let iter = iters.iter_mut().next().unwrap();
            ret.extend(iter.take(limit));
            return ret;
        }
        let mut heap = Vec::with_capacity(iters.len());
        // Fill the heap with values
        if !fill_heap(iters.iter_mut(), &mut heap) {
            // Hit an exhausted iterator, finish
            return ret;
        }

        while ret.len() < limit {
            if is_hit(&heap) {
                // We hit intersected item
                if let Some((item, _)) = heap.pop() {
                    // Push it into the intersect values
                    ret.push(item);
                    // Clear the rest of the heap
                    heap.clear();
                    // Build a new heap from new values
                    if !fill_heap(iters.iter_mut(), &mut heap) {
                        // Hit an exhausted iterator, finish
                        return ret;
                    }
                } else {
                    // Hit an exhausted iterator, finish
                    return ret;
                }
            } else {
                // Remove max element from the heap
                if let Some((_, iter_num)) = heap.pop() {
                    if let Some(item) = iters[iter_num].next() {
                        // Insert replacement from the corresponding iterator to heap
                        heap.push((item, iter_num));
                        heapify(&mut heap);
                    } else {
                        // Hit an exhausted iterator, finish
                        return ret;
                    }
                } else {
                    // Hit an exhausted iterator, finish
                    return ret;
                }
            }
        }

        ret
    }

    fn heapify<Item: Ord>(heap: &mut Vec<(Item, usize)>) {
        heap.sort_by(|(a, _), (b, _)| a.cmp(b));
    }

    fn fill_heap<'a, Item: Ord, Inner: 'a + Iterator<Item=Item>, Outer: Iterator<Item=&'a mut Inner>>(iters: Outer, heap: &mut Vec<(Inner::Item, usize)>) -> bool {
        for (i, iter) in iters.enumerate() {
            let value = iter.next();
            if let Some(value) = value {
                heap.push((value, i))
            } else {
                return false;
            }
        }
        heapify(heap);
        true
    }

    fn is_hit<Item: Ord>(heap: &Vec<(Item, usize)>) -> bool {
        let value = heap.iter().next().map(|(value, _)|
            heap.iter().fold((value, true), |(a, eq), (b, _)| {
                (b, eq & (a.cmp(b) == Ordering::Equal))
            })
        );

        if let Some((_, true)) = value {
            true
        } else {
            false
        }
    }
}