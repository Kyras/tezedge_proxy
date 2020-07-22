// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use std::{
    sync::Arc,
    time::{SystemTime, Duration},
};
use rocksdb::DB;
use storage::persistent::{KeyValueStoreWithSchema, KeyValueSchema};
use crate::messages::metric_message::ContainerStatsMessage;

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
}

#[derive(Clone)]
pub struct MetricStore {
    kv: Arc<dyn KeyValueStoreWithSchema<MetricStore> + Send + Sync>,
}

impl KeyValueSchema for MetricStore {
    type Key = u64;
    type Value = ContainerStatsMessage;

    fn name() -> &'static str { "metric_message_storage" }
}

impl MetricStore {
    pub fn new(kv: Arc<DB>) -> Self {
        MetricStore {
            kv: kv.clone(),
        }
    }
}
