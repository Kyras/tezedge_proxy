// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

pub mod common;

use common::{debugger_url, get_rpc_as_json};

const V2_ENDPOINT: &str = "v2/p2p";
const EXPECTED_MESSAGES: usize = 6;

#[tokio::test]
/// Test content of p2p endpoints, if it contains correct count of messages.
async fn tests_p2p_correct_test_output() {
    let debugger_url = debugger_url();
    let base_endpoint = format!("{}/{}", debugger_url, V2_ENDPOINT);
    let response = get_rpc_as_json(&base_endpoint)
        .await.unwrap();
    let values = response.as_array()
        .expect("expected array of messages");
    assert_eq!(values.len(), EXPECTED_MESSAGES, "expected four parsed messages");
    for value in values {
        use serde_json::Value;
        let value = value.as_object()
            .expect("expected array of objects");

        let values: &[(&'static str, &'static dyn Fn(&Value) -> bool, &str)] = &[
            ("id", &Value::is_number, "number"),
            ("incoming", &Value::is_boolean, "boolean"),
            ("remote_addr", &Value::is_string, "string"),
            ("source_type", &Value::is_string, "string"),
            ("timestamp", &Value::is_number, "number"),
            ("type", &Value::is_string, "string"),

        ];

        for (field_name, type_check, type_name) in values {
            let field = value.get(*field_name)
                .expect(&format!("{} should be set", field_name));
            assert!(!field.is_null(), "{} must be set", field_name);
            assert!(type_check(field), "{} should be {}", field_name, type_name);
        }
    }
}

#[tokio::test]
async fn test_p2p_limit() {
    let debugger_url = debugger_url();
    let base_endpoint = format!("{}/{}", debugger_url, V2_ENDPOINT);
    for limit in 0..=EXPECTED_MESSAGES {
        let response = get_rpc_as_json(&format!("{}?limit={}", base_endpoint, limit))
            .await.unwrap();
        assert_eq!(response.as_array().unwrap().len(), limit);
    }
}

#[ignore]
#[tokio::test]
async fn test_p2p_cursor() {
    let debugger_url = debugger_url();
    let base_endpoint = format!("{}/{}", debugger_url, V2_ENDPOINT);
    for cursor_id in 0..=EXPECTED_MESSAGES {
        let response = get_rpc_as_json(&format!("{}?cursor_id={}", base_endpoint, cursor_id))
            .await.unwrap();
        assert_eq!(response[0]["id"], cursor_id, "{}", cursor_id);
    }
}

#[tokio::test]
async fn test_p2p_types() {
    let debugger_url = debugger_url();
    let base_endpoint = format!("{}/{}", debugger_url, V2_ENDPOINT);
    let values: &[(&str, usize)] = &[
        ("metadata", 2),
        ("advertise", 0),
        ("connection_message", 2),
    ];
    for (r#type, count) in values {
        let response = get_rpc_as_json(&format!("{}?types={}", base_endpoint, r#type))
            .await.unwrap();
        assert_eq!(response.as_array().unwrap().len(), *count, "{}", r#type);
    }
}

#[tokio::test]
async fn test_p2p_combination_types() {
    let debugger_url = debugger_url();
    let base_endpoint = format!("{}/{}", debugger_url, V2_ENDPOINT);
    let values: &[(&str, usize)] = &[
        ("metadata,connection_message", 4),
        ("metadata,advertise", 2),
        ("connection_message,advertise", 2),
        ("connection_message,metadata", 4),
        ("advertise,connection_message", 2),
        ("advertise,metadata", 2),
        ("advertise,metadata,connection_message", 4),
    ];
    for (r#type, number) in values {
        let response = get_rpc_as_json(&format!("{}?types={}", base_endpoint, *r#type))
            .await.unwrap();
        assert_eq!(response.as_array().unwrap().len(), *number, "{}", r#type);
    }
}