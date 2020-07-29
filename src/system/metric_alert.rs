// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use sysinfo::System;
use chrono::{DateTime, Utc, Duration};
use crate::messages::metric_message::MetricMessage;

/// Configuration of alert, conditions that trigger the alert
#[derive(Clone)]
pub struct AlertConfig;

impl AlertConfig {
    pub fn condition_checker(&self) -> SystemCapacityObserver {
        SystemCapacityObserver {
            system: {
                use sysinfo::{SystemExt, RefreshKind};

                let r = RefreshKind::new().with_disks().with_memory().with_cpu();
                let mut s = System::new_with_specifics(r);
                s.refresh_disks();
                s.refresh_memory();
                s.refresh_cpu();
                s
            },
            memory: MemoryEstimator::new(),
        }
    }
}

struct MemoryEstimator {
    usage: Vec<(DateTime<Utc>, u64)>,
    keep_last: bool,
}

impl MemoryEstimator {
    pub fn new() -> Self {
        MemoryEstimator {
            usage: Vec::new(),
            keep_last: false,
        }
    }

    pub fn observe(&mut self, timestamp: DateTime<Utc>, usage: u64) {
        // TODO:
        if let Some(&(_, ref _last_usage)) = self.usage.last() {
            if self.keep_last {
                self.usage.last_mut().map(|l| *l = (timestamp, usage));
            } else {
                self.usage.push((timestamp, usage));
            }
        }
    }

    pub fn estimate(&self, _available: u64) -> Option<Duration> {
        // TODO:
        None
    }

    pub fn status(&self) -> u64 {
        self.usage.last().cloned().unwrap_or((Utc::now(), 0)).1
    }
}

pub struct SystemCapacityObserver {
    system: System,
    memory: MemoryEstimator,
}

impl SystemCapacityObserver {
    pub fn observe(&mut self, message: &MetricMessage) {
        self.memory.observe(message.0.timestamp.clone(), message.0.memory.usage.clone());
    }

    pub fn alert(&self) -> Vec<String> {
        use sysinfo::SystemExt;

        let mut alerts = Vec::new();
        if let Some(estimate) = self.memory.estimate(self.system.get_available_memory()) {
            if estimate < Duration::minutes(10) {
                alerts.push(format!("Warning, memory will exhaust estimated in {}", estimate));
            }
        }

        alerts
    }

    pub fn status(&self) -> Vec<String> {
        let memory = self.memory.status();
        let gb = (memory as f64) / (0x40000000 as f64);
        vec![format!("Memory usage: {:.2} GiB", gb)]
    }
}
