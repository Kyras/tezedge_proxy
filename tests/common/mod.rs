// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use std::env;

pub const DEFAULT_LIMIT: usize = 100;

pub async fn get_rpc_as_json(url: &str) -> Result<serde_json::value::Value, serde_json::error::Error> {
    let res = reqwest::get(url)
        .await.unwrap()
        .text()
        .await.unwrap();
    serde_json::from_str(&res)
}

/// Make x number of rpc calls to the node
pub async fn make_rpc_calls(x: i32) {
    let node_url = node_url();
    for _ in 0..x {
        let _ = get_rpc_as_json(&format!("{}/{}", node_url, "chains/main/blocks/head")).await.unwrap();
    }
}

pub fn debugger_url() -> String {
    env::var("DEBUGGER_URL")
        .unwrap()
}

pub fn node_url() -> String {
    env::var("NODE_URL")
        //.unwrap_or("http://116.202.128.230:48732".to_string())
        .unwrap()
}

pub fn websocket_port() -> String {
    env::var("WEBSOCKET_PORT")
        //.unwrap_or("http://116.202.128.230:48732".to_string())
        .unwrap()
}