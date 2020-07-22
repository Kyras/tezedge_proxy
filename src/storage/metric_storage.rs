// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use std::{
    sync::{Arc, atomic::{AtomicU64, Ordering}},
    time::{SystemTime, Duration},
};
use rocksdb::DB;
use storage::{StorageError, Direction, IteratorMode, persistent::{KeyValueStoreWithSchema, KeyValueSchema}};
use crate::messages::metric_message::MetricMessage;

#[derive(Debug, Default, Clone)]
pub struct MetricFilters {
    pub start: Option<SystemTime>,
    pub end: Option<SystemTime>,
}

impl MetricFilters {
    /// take events that happened last `secs` seconds
    pub fn recent(self, secs: u64) -> Self {
        let now = SystemTime::now();
        let start = now - Duration::from_secs(secs);
        MetricFilters {
            start: Some(start),
            end: Some(now),
        }
    }

    pub fn empty(&self) -> bool {
        self.start.is_none() && self.end.is_none()
    }
}

#[derive(Clone)]
pub struct MetricStore {
    kv: Arc<dyn KeyValueStoreWithSchema<MetricStore> + Send + Sync>,
    count: Arc<AtomicU64>,
    seq: Arc<AtomicU64>,
}

impl KeyValueSchema for MetricStore {
    type Key = u64;
    type Value = MetricMessage;

    fn name() -> &'static str { "metric_message_storage" }
}

// TODO: indices
impl MetricStore {
    pub fn new(kv: Arc<DB>) -> Self {
        MetricStore {
            kv: kv.clone(),
            count: Arc::new(AtomicU64::new(0)),
            seq: Arc::new(AtomicU64::new(0)),
        }
    }

    fn cursor_iterator<'a>(&'a self, cursor_index: Option<u64>) -> Result<Box<dyn 'a + Iterator<Item=(u64, MetricMessage)>>, StorageError> {
        Ok(Box::new(self.kv.iterator(IteratorMode::From(&cursor_index.unwrap_or(std::u64::MAX), Direction::Reverse))?
            .filter_map(|(k, v)| {
                k.ok().and_then(|key| Some((key, v.ok()?)))
            })))
    }

    pub fn store_message_array(&self, messages: Vec<MetricMessage>) -> Result<u64, StorageError> {
        if messages.is_empty() {
            Ok(self.seq.load(Ordering::SeqCst))
        } else {
            let l = messages.len() as u64;
            let index = self.seq.fetch_add(l, Ordering::SeqCst);
            for message in messages {
                self.kv.put(&index, &message)?;
            }
            //self.make_indices()?;
            self.count.fetch_add(l, Ordering::SeqCst);
            Ok(index)
        }
    }

    pub fn get_cursor(&self, cursor_index: Option<u64>, limit: usize, filters: MetricFilters) -> Result<Vec<MetricMessage>, StorageError> {
        let mut ret = Vec::with_capacity(limit);
        if filters.empty() {
            ret.extend(self.cursor_iterator(cursor_index)?.take(limit).map(|(_key, value)| value));
        } else {
            unimplemented!()
            //let mut iters: Vec<Box<dyn Iterator<Item=u64>>> = Default::default();
            //ret.extend(self.load_indexes(sorted_intersect(iters, limit).into_iter()));
        }
        Ok(ret)
    }
}
