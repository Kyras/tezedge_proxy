// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use sysinfo::System;
use std::fmt;
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
        }
    }
}

pub struct SystemCapacityObserver {
    system: System,
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

impl SystemCapacityObserver {
    pub fn observe(&mut self, message: &MetricMessage) {
        let _ = (message, &mut self.system);
        // TODO:
    }

    pub fn alert(&self) -> Option<Alert> {
        // TODO:
        None
    }
}
