// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use crate::system::SystemSettings;

// TODO: error handling, select proper container not first `containers_info.first()`
pub async fn metric_collector(settings: SystemSettings) {
    use std::collections::HashMap;
    use crate::messages::metric_message::{MetricMessage, ContainerInfo};
    use tokio::time;

    let url = settings.cadvisor_url
        .join("api/v1.3/docker")
        .unwrap();

    tokio::spawn(async move {
        loop {
            let r = reqwest::get(url.clone()).await.unwrap().text().await.unwrap();
            let info = serde_json::from_str::<HashMap<String, ContainerInfo>>(r.as_str()).unwrap();
            let containers_info = info.values().collect::<Vec<_>>();
            let container_info = containers_info.first().unwrap();
            let messages = container_info.stats.clone().into_iter().map(MetricMessage).collect();
            settings.storage.metric().store_message_array(messages).unwrap();

            time::delay_for(settings.metrics_fetch_interval).await;
        }
    });
}
