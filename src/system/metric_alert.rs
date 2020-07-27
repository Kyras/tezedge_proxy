// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use crate::messages::metric_message::MetricMessage;
use std::fmt;

#[derive(Clone)]
/// Configuration of alert, conditions that trigger the alert
pub struct AlertCondition {
    pub memory_usage_threshold: u64,
}

pub struct Alert {
    memory: Option<u64>,
    cpu: Option<()>,
}

impl fmt::Display for Alert {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(bytes) = self.memory {
            writeln!(f, "memory usage too high: {} bytes", bytes)?;
        }
        if let Some(()) = self.cpu {
            writeln!(f, "cpu usage too high")?;
        }
        Ok(())
    }
}

impl AlertCondition {
    pub fn check(&self, message: &MetricMessage) -> Option<Alert> {
        let memory_usage = message.0.memory.usage;
        if memory_usage >= self.memory_usage_threshold {
            Some(Alert {
                memory: Some(memory_usage),
                cpu: None,
            })
        } else {
            None
        }
    }
}
