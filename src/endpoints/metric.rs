// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use crate::storage::{MessageStore, MetricFilters};
use warp::{
    Filter, Reply, Rejection,
    reply::{with_status, json},
    http::StatusCode,
};
use serde::{Serialize, Deserialize};
use warp::reply::{WithStatus, Json};
use chrono::{DateTime, Utc};

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct MetricCursor {
    pub cursor_id: Option<u64>,
    pub limit: Option<usize>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

impl Into<MetricFilters> for MetricCursor {
    fn into(self) -> MetricFilters {
        MetricFilters {
            start: self.start_time,
            end: self.end_time,
        }
    }
}

pub fn metric(storage: MessageStore) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone + Sync + Send + 'static {
    warp::path!("v2" / "metric")
        .and(warp::query::query())
        .map(move |cursor: MetricCursor| -> WithStatus<Json> {
            match storage.metric().get_cursor(cursor.cursor_id, cursor.limit.unwrap_or(100), cursor.into()) {
                Ok(msgs) => with_status(json(&msgs), StatusCode::OK),
                Err(err) => with_status(json(&format!("database error: {}", err)), StatusCode::INTERNAL_SERVER_ERROR),
            }
        })
}
