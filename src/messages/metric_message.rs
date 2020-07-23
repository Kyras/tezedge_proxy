use serde::{Serialize, Deserialize};
use storage::persistent::{Decoder, Encoder, SchemaError};
use chrono::{DateTime, Utc, TimeZone};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricMessage(pub ContainerStats);

#[derive(Debug, Clone)]
pub struct MetricMessageKey(pub DateTime<Utc>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    #[serde(default)]
    pub id: String,

    pub name: String,

    #[serde(default)]
    pub aliases: Vec<String>,

    #[serde(default)]
    pub namespace: String,

    #[serde(default)]
    pub subcontainers: Vec<ContainerInfo>,

    pub spec: ContainerSpec,

    #[serde(default)]
    pub stats: Vec<ContainerStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSpec {
    #[serde(default)]
    image: String,
}

impl ContainerSpec {
    pub fn tezos_node(&self) -> bool {
        self.image.find("tezos/tezos").is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStats {
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

impl Decoder for MetricMessageKey {
    fn decode(bytes: &[u8]) -> Result<Self, SchemaError> {
        use byteorder::{ByteOrder, BigEndian};

        let t = BigEndian::read_i64(&bytes[..8]);
        Ok(MetricMessageKey(Utc.timestamp(t, 0)))
    }
}

impl Encoder for MetricMessageKey {
    fn encode(&self) -> Result<Vec<u8>, SchemaError> {
        use byteorder::{ByteOrder, BigEndian};

        let t = self.0.timestamp();
        let mut v = Vec::with_capacity(8);
        v.resize(8, 0);
        BigEndian::write_i64(v.as_mut(), t);
        Ok(v)
    }
}

impl Decoder for MetricMessage {
    fn decode(bytes: &[u8]) -> Result<Self, SchemaError> {
        serde_cbor::from_slice(bytes)
            .map_err(|_| SchemaError::DecodeError)
    }
}

impl Encoder for MetricMessage {
    fn encode(&self) -> Result<Vec<u8>, SchemaError> {
        serde_cbor::to_vec(self)
            .map_err(|_| SchemaError::EncodeError)
    }
}

impl Decoder for ContainerStats {
    fn decode(bytes: &[u8]) -> Result<Self, SchemaError> {
        serde_cbor::from_slice(bytes)
            .map_err(|_| SchemaError::DecodeError)
    }
}

impl Encoder for ContainerStats {
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

#[cfg(test)]
#[test]
fn deserialize_container_info_map_from_json() {
    use std::collections::HashMap;

    // curl 'http://localhost:8080/api/v1.3/docker' > tests/metrics_data.json
    const DATA: &str = include_str!("../../tests/metrics_data.json");
    let info = serde_json::from_str::<HashMap<String, ContainerInfo>>(DATA).unwrap();
    println!("{:#?}", info);
}
