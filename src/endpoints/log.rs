use crate::storage::{MessageStore, log_indexes::{LogLevel, ParseLogLevel}, LogFilters};
use failure::Error;
use warp::{
    Filter, Reply, Rejection,
    reply::{with_status, json, WithStatus, Json},
    http::StatusCode,
};
use std::convert::TryInto;
use serde::{Serialize, Deserialize};
use itertools::Itertools;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct LogCursor {
    pub cursor_id: Option<u64>,
    pub limit: Option<usize>,
    pub level: Option<String>,
    pub timestamp: Option<String>,
}

impl LogCursor {
    fn get_timestamp(&self) -> Result<Option<u128>, Error> {
        if let Some(ref ts) = self.timestamp {
            Ok(Some(ts.parse()?))
        } else {
            Ok(None)
        }
    }

    fn get_level(&self) -> Result<Option<LogLevel>, ParseLogLevel> {
        if let Some(ref level) = self.level {
            Ok(Some(level.parse()?))
        } else {
            Ok(None)
        }
    }
}

impl TryInto<LogFilters> for LogCursor {
    type Error = Error;

    fn try_into(self) -> Result<LogFilters, Self::Error> {
        Ok(LogFilters {
            level: self.get_level()?,
            date: self.get_timestamp()?,
        })
    }
}

pub fn log(storage: MessageStore) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone + Sync + Send + 'static {
    warp::path!("v2" / "log")
        .and(warp::query::query())
        .map(move |cursor: LogCursor| -> WithStatus<Json> {
            let limit = cursor.limit.unwrap_or(100);
            let cursor_id = cursor.cursor_id.clone();
            match cursor.try_into() {
                Ok(filters) => match storage.log().get_cursor(cursor_id, limit, filters) {
                    Ok(msgs) => with_status(json(&msgs.into_iter().map(|log| {
                        log.compress()
                    }).collect_vec()), StatusCode::OK),
                    Err(err) => with_status(json(&format!("database error: {}", err)), StatusCode::INTERNAL_SERVER_ERROR),
                },
                Err(level_error) => with_status(json(&format!("invalid type-name: {}", level_error)), StatusCode::BAD_REQUEST),
            }
        })
}