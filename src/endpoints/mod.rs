// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

pub mod p2p;
pub mod rpc;
pub mod log;
pub mod stat;
pub mod metric;

use warp::{
    Filter, Reply,
    reject::Rejection,
    reply::with::header,
};
use crate::storage::MessageStore;
use crate::endpoints::p2p::p2p;
use crate::endpoints::rpc::rpc;
use crate::endpoints::log::log;
use crate::endpoints::stat::stat;
use crate::endpoints::metric::metric;

pub fn routes(storage: MessageStore) -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone + Sync + Send + 'static {
    warp::get().and(
        p2p(storage.clone())
            .or(rpc(storage.clone()))
            .or(log(storage.clone()))
            .or(stat(storage.clone()))
            .or(metric(storage.clone()))
    )
        .with(header("Content-Type", "application/json"))
        .with(header("Access-Control-Allow-Origin", "*"))
}
