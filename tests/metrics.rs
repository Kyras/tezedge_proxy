// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

pub mod common;

use common::{cadvisor_url, get_rpc_as_json};

const V_1_3_ENDPOINT: &str = "api/v1.3";

#[tokio::test]
async fn metrics() {
    let url = format!(
        "{}/{}/docker/tezedge-debugger_ocaml-node_1",
        cadvisor_url(),
        V_1_3_ENDPOINT,
    );
    let response = get_rpc_as_json(&url).await.unwrap();
    response.as_object().unwrap().values().for_each(|v| {
        let stats = v.as_object().unwrap().get("stats").unwrap().as_array().unwrap();
        stats.iter().for_each(|v| assert!(v.as_object().unwrap().keys().find(|&x| x == "memory").is_some()));
    });
}
