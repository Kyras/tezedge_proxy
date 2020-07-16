// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use storage::persistent::{KeyValueStoreWithSchema, KeyValueSchema};
use std::sync::Arc;
use rocksdb::{DB};
use tracing::{info, error};
use std::sync::atomic::{AtomicU64, Ordering};
use crate::messages::log_message::LogMessage;
use storage::{StorageError, IteratorMode, Direction};
use crate::storage::log_storage::secondary_indexes::{LevelIndex, LogLevel, TimestampIndex};
use crate::storage::secondary_index::SecondaryIndex;
use crate::storage::sorted_intersect::sorted_intersect;

#[cfg(not(debug_assertions))]
use tracing::field::display;

pub type LogStorageKV = dyn KeyValueStoreWithSchema<LogStore> + Sync + Send;

#[derive(Debug, Default, Clone)]
pub struct LogFilters {
    pub level: Option<LogLevel>,
    pub date: Option<u128>,
}

impl LogFilters {
    pub fn empty(&self) -> bool {
        self.level.is_none() && self.date.is_none()
    }
}

#[derive(Clone)]
pub struct LogStore {
    kv: Arc<LogStorageKV>,
    level_index: LevelIndex,
    timestamp_index: TimestampIndex,
    count: Arc<AtomicU64>,
    seq: Arc<AtomicU64>,
}

#[allow(dead_code)]
impl LogStore {
    pub fn new(kv: Arc<DB>) -> Self {
        Self {
            kv: kv.clone(),
            level_index: LevelIndex::new(kv.clone()),
            timestamp_index: TimestampIndex::new(kv),
            count: Arc::new(AtomicU64::new(0)),
            seq: Arc::new(AtomicU64::new(0)),
        }
    }

    fn index(&self) -> u64 {
        self.seq.load(Ordering::SeqCst)
    }

    fn inc_count(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn reserve_index(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::SeqCst)
    }

    pub fn make_indexes(&self, primary_index: u64, value: &LogMessage) -> Result<(), StorageError> {
        self.level_index.store_index(&primary_index, value)?;
        self.timestamp_index.store_index(&primary_index, value)
    }

    pub fn delete_indexes(&self, primary_index: u64, value: &LogMessage) -> Result<(), StorageError> {
        self.level_index.delete_index(&primary_index, value)?;
        self.timestamp_index.delete_index(&primary_index, value)
    }

    pub fn put_message(&self, index: u64, msg: &mut LogMessage) -> Result<(), StorageError> {
        msg.id = Some(index);
        if self.kv.contains(&index)? {
            self.kv.merge(&index, &msg)?;
        } else {
            self.kv.put(&index, &msg)?;
            self.inc_count();
        }
        self.make_indexes(index, msg)?;
        Ok(())
    }

    pub fn store_message(&self, msg: &mut LogMessage) -> Result<u64, StorageError> {
        let index = self.reserve_index();
        msg.id = Some(index);
        self.kv.put(&index, &msg)?;
        self.make_indexes(index, &msg)?;
        self.inc_count();
        Ok(index)
    }

    pub fn get_cursor(&self, cursor_index: Option<u64>, limit: usize, filters: LogFilters) -> Result<Vec<LogMessage>, StorageError> {
        let mut ret = Vec::with_capacity(limit);
        if filters.empty() {
            ret.extend(self.cursor_iterator(cursor_index)?.map(|(_, v)| v).take(limit));
        } else {
            let mut iters: Vec<Box<dyn Iterator<Item=u64>>> = Default::default();
            if let Some(level) = filters.level {
                iters.push(self.level_iterator(cursor_index, level)?);
            }
            if let Some(timestamp) = filters.date {
                iters.push(self.timestamp_iterator(cursor_index, timestamp)?);
            }
            ret.extend(self.load_indexes(sorted_intersect(iters, limit).into_iter()));
        }
        Ok(ret)
    }

    fn cursor_iterator<'a>(&'a self, cursor_index: Option<u64>) -> Result<Box<dyn 'a + Iterator<Item=(u64, LogMessage)>>, StorageError> {
        Ok(Box::new(self.kv.iterator(IteratorMode::From(&cursor_index.unwrap_or(std::u64::MAX), Direction::Reverse))?
            .filter_map(|(k, v)| {
                k.ok().and_then(|key| Some((key, v.ok()?)))
            })))
    }

    pub fn level_iterator<'a>(&'a self, cursor_index: Option<u64>, level: LogLevel) -> Result<Box<dyn 'a + Iterator<Item=u64>>, StorageError> {
        Ok(Box::new(self.level_index.get_concrete_prefix_iterator(&cursor_index.unwrap_or(std::u64::MAX), level)?
            .filter_map(|(_, value)| {
                value.ok()
            })))
    }

    pub fn timestamp_iterator<'a>(&'a self, cursor_index: Option<u64>, timestamp: u128) -> Result<Box<dyn 'a + Iterator<Item=u64>>, StorageError> {
        println!("Getting the timestamp iterator");
        Ok(Box::new(self.timestamp_index.get_iterator(&cursor_index.unwrap_or(std::u64::MAX), timestamp, Direction::Reverse)?
            .filter_map(|(_, value)| {
                value.ok()
            })))
    }

    fn load_indexes<Iter: 'static + Iterator<Item=u64>>(&self, indexes: Iter) -> impl Iterator<Item=LogMessage> + 'static {
        let kv = self.kv.clone();
        indexes.filter_map(move |index| {
            match kv.get(&index) {
                Ok(Some(value)) => {
                    Some(value)
                }
                Ok(None) => {
                    info!(index = index, "no value found");
                    None
                }
                Err(err) => {
                    error!(index = index, error = display(&err), "failed to load value");
                    None
                }
            }
        })
    }
}

impl KeyValueSchema for LogStore {
    type Key = u64;
    type Value = LogMessage;

    fn name() -> &'static str { "log_storage" }
}

pub(crate) mod secondary_indexes {
    use storage::persistent::{KeyValueStoreWithSchema, KeyValueSchema, Decoder, SchemaError, Encoder};
    use std::{
        sync::Arc,
        str::FromStr,
    };
    use tracing::warn;
    use rocksdb::{DB, ColumnFamilyDescriptor, Options, SliceTransform};
    use crate::storage::LogStore;
    use serde::{Serialize, Deserialize};
    use failure::{Fail};
    use crate::storage::secondary_index::SecondaryIndex;
    use std::convert::{TryFrom, TryInto};

    #[cfg(not(debug_assertions))]
    use tracing::field::display;
    
    pub type LevelIndexKV = dyn KeyValueStoreWithSchema<LevelIndex> + Sync + Send;

    #[derive(Clone)]
    pub struct LevelIndex {
        kv: Arc<LevelIndexKV>,
    }

    impl LevelIndex {
        pub fn new(kv: Arc<DB>) -> Self {
            Self { kv }
        }
    }

    impl AsRef<(dyn KeyValueStoreWithSchema<LevelIndex> + 'static)> for LevelIndex {
        fn as_ref(&self) -> &(dyn KeyValueStoreWithSchema<LevelIndex> + 'static) {
            self.kv.as_ref()
        }
    }

    impl KeyValueSchema for LevelIndex {
        type Key = LogLevelKey;
        type Value = <LogStore as KeyValueSchema>::Key;

        fn descriptor() -> ColumnFamilyDescriptor {
            let mut cf_opts = Options::default();
            cf_opts.set_prefix_extractor(SliceTransform::create_fixed_prefix(std::mem::size_of::<LogLevel>()));
            cf_opts.set_memtable_prefix_bloom_ratio(0.2);
            ColumnFamilyDescriptor::new(Self::name(), cf_opts)
        }

        fn name() -> &'static str {
            "log_level_index"
        }
    }

    impl SecondaryIndex<LogStore> for LevelIndex {
        type FieldType = LogLevel;

        fn accessor(value: &<LogStore as KeyValueSchema>::Value) -> Option<Self::FieldType> {
            match value.level.parse() {
                Ok(level) => Some(level),
                Err(_) => {
                    warn!(level = display(&value.level), "got invalid log level");
                    None
                }
            }
        }

        fn make_index(key: &<LogStore as KeyValueSchema>::Key, value: Self::FieldType) -> LogLevelKey {
            LogLevelKey::new(value, key.clone())
        }

        fn make_prefix_index(value: Self::FieldType) -> LogLevelKey {
            LogLevelKey::prefix(value)
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LogLevelKey {
        pub level: LogLevel,
        pub index: u64,
    }

    impl LogLevelKey {
        pub fn new(level: LogLevel, index: u64) -> Self {
            Self {
                level,
                index: std::u64::MAX.saturating_sub(index),
            }
        }

        pub fn prefix(level: LogLevel) -> Self {
            Self {
                level,
                index: 0,
            }
        }
    }

    /// * bytes layout: `[level(1)][padding(7)][index(8)]`
    impl Decoder for LogLevelKey {
        fn decode(bytes: &[u8]) -> Result<Self, SchemaError> {
            if bytes.len() != 16 {
                return Err(SchemaError::DecodeError);
            }
            let level_value = &bytes[0..1];
            let _padding_value = &bytes[1..7 + 1];
            let index_value = &bytes[7 + 1..];
            // level
            let level = level_value[0].try_into().map_err(|_| SchemaError::DecodeError)?;
            // index
            let mut index = [0u8; 8];
            for (x, y) in index.iter_mut().zip(index_value) {
                *x = *y;
            }
            let index = u64::from_be_bytes(index);
            Ok(Self {
                level,
                index,
            })
        }
    }

    /// * bytes layout: `[level(1)][padding(7)][index(8)]`
    impl Encoder for LogLevelKey {
        fn encode(&self) -> Result<Vec<u8>, SchemaError> {
            let mut buf = Vec::with_capacity(16);

            buf.extend_from_slice(&[self.level.clone() as u8]);
            buf.extend_from_slice(&[0u8; 7]);
            buf.extend_from_slice(&self.index.to_be_bytes());

            if buf.len() != 16 {
                println!("{:?} - {:?}", self, buf);
                Err(SchemaError::EncodeError)
            } else {
                Ok(buf)
            }
        }
    }

    #[repr(u8)]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum LogLevel {
        Trace = 0x1 << 0,
        Debug = 0x1 << 1,
        Info = 0x1 << 2,
        Notice = 0x1 << 3,
        Warning = 0x1 << 4,
        Error = 0x1 << 5,
        Fatal = 0x1 << 6,
    }

    #[derive(Debug, Fail)]
    pub enum ParseLogLevel {
        #[fail(display = "Invalid log level name {}", _0)]
        InvalidName(String),
        #[fail(display = "Invalid log level value {}", _0)]
        InvalidValue(u8),
    }

    impl TryFrom<u8> for LogLevel {
        type Error = ParseLogLevel;

        fn try_from(value: u8) -> Result<Self, ParseLogLevel> {
            match value {
                x if x == Self::Trace as u8 => { Ok(Self::Trace) }
                x if x == Self::Debug as u8 => { Ok(Self::Debug) }
                x if x == Self::Info as u8 => { Ok(Self::Info) }
                x if x == Self::Notice as u8 => { Ok(Self::Notice) }
                x if x == Self::Warning as u8 => { Ok(Self::Warning) }
                x if x == Self::Error as u8 => { Ok(Self::Error) }
                x if x == Self::Fatal as u8 => { Ok(Self::Fatal) }
                x => Err(ParseLogLevel::InvalidValue(x)),
            }
        }
    }

    impl FromStr for LogLevel {
        type Err = ParseLogLevel;

        fn from_str(level: &str) -> Result<Self, Self::Err> {
            let level = level.to_lowercase();
            Ok(match level.as_ref() {
                "trace" => Self::Trace,
                "debug" => Self::Debug,
                "info" => Self::Info,
                "notice" => Self::Notice,
                "warn" | "warning" => Self::Warning,
                "error" => Self::Error,
                "fatal" => Self::Fatal,
                _ => return Err(ParseLogLevel::InvalidName(level)),
            })
        }
    }

    // Timestamp
    pub type TimestampIndexKV = dyn KeyValueStoreWithSchema<TimestampIndex> + Sync + Send;

    #[derive(Clone)]
    pub struct TimestampIndex {
        kv: Arc<TimestampIndexKV>,
    }

    impl AsRef<(dyn KeyValueStoreWithSchema<TimestampIndex> + 'static)> for TimestampIndex {
        fn as_ref(&self) -> &(dyn KeyValueStoreWithSchema<TimestampIndex> + 'static) {
            self.kv.as_ref()
        }
    }

    impl KeyValueSchema for TimestampIndex {
        type Key = TimestampKey;
        type Value = <LogStore as KeyValueSchema>::Key;

        fn name() -> &'static str {
            "log_timestamp_index"
        }
    }

    impl TimestampIndex {
        pub fn new(kv: Arc<DB>) -> Self {
            Self { kv }
        }
    }

    impl SecondaryIndex<LogStore> for TimestampIndex {
        type FieldType = u128;

        fn accessor(value: &<LogStore as KeyValueSchema>::Value) -> Option<Self::FieldType> {
            Some(value.date)
        }

        fn make_index(key: &<LogStore as KeyValueSchema>::Key, value: Self::FieldType) -> TimestampKey {
            TimestampKey::new(value, key.clone())
        }

        fn make_prefix_index(value: Self::FieldType) -> TimestampKey {
            TimestampKey::prefix(value)
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TimestampKey {
        pub timestamp: u128,
        pub index: u64,
    }

    impl TimestampKey {
        pub fn new(timestamp: u128, index: u64) -> Self {
            Self {
                timestamp,
                index,
            }
        }

        pub fn prefix(timestamp: u128) -> Self {
            Self {
                timestamp,
                index: 0,
            }
        }
    }

    /// * bytes layout: `[timestamp(16)][index(8)]`
    impl Decoder for TimestampKey {
        fn decode(bytes: &[u8]) -> Result<Self, SchemaError> {
            if bytes.len() != 24 {
                return Err(SchemaError::DecodeError);
            }

            let timestamp_value = &bytes[0..16];
            let index_value = &bytes[16..];

            // timestamp
            let mut timestamp = [0u8; 16];
            for (x, y) in timestamp.iter_mut().zip(timestamp_value) {
                *x = *y;
            }
            let timestamp = u128::from_be_bytes(timestamp);

            // index
            let mut index = [0u8; 8];
            for (x, y) in index.iter_mut().zip(index_value) {
                *x = *y;
            }
            let index = u64::from_be_bytes(index);

            Ok(Self {
                timestamp,
                index,
            })
        }
    }

    /// * bytes layout: `[timestamp(16)][index(8)]`
    impl Encoder for TimestampKey {
        fn encode(&self) -> Result<Vec<u8>, SchemaError> {
            let mut buf = Vec::with_capacity(24);

            buf.extend_from_slice(&self.timestamp.to_be_bytes());
            buf.extend_from_slice(&self.index.to_be_bytes());

            if buf.len() != 24 {
                println!("{:?} - {:?}", self, buf);
                Err(SchemaError::EncodeError)
            } else {
                Ok(buf)
            }
        }
    }
}