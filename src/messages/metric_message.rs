use serde::{Serialize, Deserialize};
use storage::persistent::{Decoder, Encoder, SchemaError};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStatsMessage {
    #[serde(with = "date_format")]
    pub timestamp: DateTime<Utc>,

    #[serde(default)]
    pub memory: MemoryStats,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStats {
    pub usage: u64,
    pub max_usage: u64,
    pub cache: u64,
    pub rss: u64,
    pub swap: u64,
    pub mapped_file: u64,
    pub working_set: u64,
    pub failcnt: u64,

    #[serde(default)]
    pub container_data: MemoryStatsMemoryData,

    #[serde(default)]
    pub hierarchical_data: MemoryStatsMemoryData,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStatsMemoryData {
    pub pgfault: u64,
    pub pgmajfault: u64,    
}

impl Decoder for ContainerStatsMessage {
    fn decode(bytes: &[u8]) -> Result<Self, SchemaError> {
        serde_cbor::from_slice(bytes)
            .map_err(|_| SchemaError::DecodeError)
    }
}

impl Encoder for ContainerStatsMessage {
    fn encode(&self) -> Result<Vec<u8>, SchemaError> {
        serde_cbor::to_vec(self)
            .map_err(|_| SchemaError::EncodeError)
    }
}

impl Decoder for MemoryStats {
    fn decode(bytes: &[u8]) -> Result<Self, SchemaError> {
        serde_cbor::from_slice(bytes)
            .map_err(|_| SchemaError::DecodeError)
    }
}

impl Encoder for MemoryStats {
    fn encode(&self) -> Result<Vec<u8>, SchemaError> {
        serde_cbor::to_vec(self)
            .map_err(|_| SchemaError::EncodeError)
    }
}

impl Decoder for MemoryStatsMemoryData {
    fn decode(bytes: &[u8]) -> Result<Self, SchemaError> {
        serde_cbor::from_slice(bytes)
            .map_err(|_| SchemaError::DecodeError)
    }
}

impl Encoder for MemoryStatsMemoryData {
    fn encode(&self) -> Result<Vec<u8>, SchemaError> {
        serde_cbor::to_vec(self)
            .map_err(|_| SchemaError::EncodeError)
    }
}

// copied from https://serde.rs/custom-date-format.html
mod date_format {
    use chrono::{DateTime, Utc, TimeZone};
    use serde::{self, Deserialize, Serializer, Deserializer};

    const FORMAT: &'static str = "%Y-%m-%dT%H:%M:%S.%fZ";

    pub fn serialize<S>(
        date: &DateTime<Utc>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Utc.datetime_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}
