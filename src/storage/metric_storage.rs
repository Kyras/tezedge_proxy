// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use rocksdb::DB;
use storage::{StorageError, IteratorMode, persistent::{KeyValueStoreWithSchema, KeyValueSchema}};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use crate::messages::metric_message::{MetricMessage, MetricMessageKey};

#[derive(Debug, Default, Clone)]
pub struct MetricFilters {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

impl MetricFilters {
    pub fn empty(&self) -> bool {
        self.start.is_none() && self.end.is_none()
    }
}

#[derive(Clone)]
pub struct MetricStore {
    kv: Arc<dyn KeyValueStoreWithSchema<MetricStore> + Send + Sync>,
}

impl KeyValueSchema for MetricStore {
    type Key = MetricMessageKey;
    type Value = MetricMessage;

    fn name() -> &'static str { "metric_message_storage" }
}

impl MetricStore {
    pub fn new(kv: Arc<DB>) -> Self {
        MetricStore {
            kv: kv.clone(),
        }
    }

    pub fn store_message_array(&self, messages: Vec<MetricMessage>) -> Result<(), StorageError> {
        for message in messages {
            self.kv.put(&MetricMessageKey(message.0.timestamp), &message)?;
        }
        Ok(())
    }

    pub fn get_cursor(&self, cursor_index: Option<u64>, limit: usize, filters: MetricFilters) -> Result<Vec<MetricMessage>, StorageError> {
        // helper;
        // it will be statically monomorphized into one of the 4 variants,
        // depends on the type of iterator
        fn skip_take<I>(it: I, index: Option<u64>, limit: usize) -> Vec<MetricMessage>
        where
            I: Iterator<Item = MetricMessage>,
        {
            match index {
                Some(index) => it.skip(index as usize).take(limit).collect(),
                None => it.take(limit).collect(),
            }
        }

        // TODO: maybe it should be optimized and write more idiomatically
        let it = self.kv.iterator(IteratorMode::End)?.filter_map(|(_, v)| v.ok());
        let ret = match (filters.start, filters.end) {
            (None, None) =>
                skip_take(it, cursor_index, limit),
            (Some(start), None) =>
                skip_take(it.take_while(|x| x.0.timestamp > start), cursor_index, limit),
            (None, Some(end)) =>
                skip_take(it.skip_while(|x| x.0.timestamp > end), cursor_index, limit),
            (Some(start), Some(end)) =>
                skip_take(it.skip_while(|x| x.0.timestamp > end).take_while(|x| x.0.timestamp > start), cursor_index, limit),
        };

        Ok(ret)
    }
}
