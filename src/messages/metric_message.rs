use serde::{Serialize, Deserialize};
use storage::persistent::{Decoder, Encoder, SchemaError};
use chrono::{DateTime, Utc, TimeZone};
use std::collections::BTreeMap;

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
    //#[serde(with = "date_format")]
    pub timestamp: DateTime<Utc>,

    #[serde(default)]
    pub cpu: CpuStats,

    #[serde(default)]
    pub diskio: DiskIoStats,

    #[serde(default)]
    pub memory: MemoryStats,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
// All CPU usage metrics are cumulative from the creation of the container
pub struct CpuStats {
    pub usage: CpuUsage,
    pub cfs: CpuCFS,
    pub schedstat: Schedstat,
	// Smoothed average of number of runnable threads x 1000.
	// We multiply by thousand to avoid using floats, but preserving precision.
	// Load is smoothed over the last 10 seconds. Instantaneous value can be read
	// from LoadStats.NrRunning.
    pub load_average: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
// CPU usage time statistics.
pub struct CpuUsage {
	// Total CPU usage.
	// Unit: nanoseconds.
    pub total: u64,

    // Per CPU/core usage of the container.
	// Unit: nanoseconds.
    pub per_cpu_usage: Vec<u64>,

	// Time spent in user space.
	// Unit: nanoseconds.
    pub user: u64,

	// Time spent in kernel space.
	// Unit: nanoseconds.
    pub system: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
// Cpu Completely Fair Scheduler statistics.
pub struct CpuCFS {
	// Total number of elapsed enforcement intervals.
    pub periods: u64,

	// Total number of times tasks in the cgroup have been throttled.
    pub throttled_periods: u64,

	// Total time duration for which tasks in the cgroup have been throttled.
	// Unit: nanoseconds.
    pub throttled_time: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
// Cpu Aggregated scheduler statistics
pub struct Schedstat {
	// https://www.kernel.org/doc/Documentation/scheduler/sched-stats.txt

	// time spent on the cpu
    pub run_time: u64,

    // time spent waiting on a runqueue
    pub runqueue_time: u64,

	// # of timeslices run on this cpu
    pub run_periods: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiskIoStats {
    #[serde(default)]
    pub io_service_bytes: Vec<PerDiskStats>,
    #[serde(default)]
    pub io_serviced: Vec<PerDiskStats>,
    #[serde(default)]
    pub io_queued: Vec<PerDiskStats>,
    #[serde(default)]
    pub sectors: Vec<PerDiskStats>,
    #[serde(default)]
    pub io_service_time: Vec<PerDiskStats>,
    #[serde(default)]
    pub io_wait_time: Vec<PerDiskStats>,
    #[serde(default)]
    pub io_merged: Vec<PerDiskStats>,
    #[serde(default)]
    pub io_time: Vec<PerDiskStats>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerDiskStats {
    pub device: String,
    pub major: u64,
    pub minor: u64,
    pub stats: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStats {
	// Current memory usage, this includes all memory regardless of when it was
	// accessed.
	// Units: Bytes.
    pub usage: u64,

	// Maximum memory usage recorded.
	// Units: Bytes.
    pub max_usage: u64,

	// Number of bytes of page cache memory.
	// Units: Bytes.
    pub cache: u64,

	// The amount of anonymous and swap cache memory (includes transparent
	// hugepages).
	// Units: Bytes.
    pub rss: u64,

	// The amount of swap currently used by the processes in this cgroup
	// Units: Bytes.
    pub swap: u64,

	// The amount of memory used for mapped files (includes tmpfs/shmem)
    pub mapped_file: u64,

	// The amount of working set memory, this includes recently accessed memory,
	// dirty memory, and kernel memory. Working set is <= "usage".
	// Units: Bytes.
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

#[cfg(test)]
#[test]
fn deserialize_container_info_map_from_json() {
    use std::collections::HashMap;

    // curl 'http://localhost:8080/api/v1.3/docker' > tests/metrics_data.json
    const DATA: &str = include_str!("../../tests/metrics_data.json");
    assert!(serde_json::from_str::<HashMap<String, ContainerInfo>>(DATA).is_ok());
}
