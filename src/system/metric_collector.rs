// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use crate::system::SystemSettings;

pub async fn metric_collector(settings: SystemSettings) {
    use tokio::time;
    use reqwest::Url;
    use tracing::error;
    use std::{fmt, collections::HashMap};
    use crate::{
        messages::metric_message::{MetricMessage, ContainerInfo},
        storage::MetricStore,
    };

    enum MetricCollectionError {
        Reqwest(reqwest::Error),
        Io(reqwest::Error),
        DeserializeJson(serde_json::Error),
        StoreMessage(storage::StorageError),
    }
    
    impl fmt::Display for MetricCollectionError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            use MetricCollectionError::*;
    
            match self {
                &Reqwest(ref e) => write!(f, "error performing http request: {}", e),
                &Io(ref e) => write!(f, "io error during http request: {}", e),
                &DeserializeJson(ref e) => write!(f, "failed to deserialize json: {}", e),
                &StoreMessage(ref e) => write!(f, "failed to store message in database: {}", e),
            }
        }
    }
    
    async fn fetch_and_store(url: &Url, storage: &MetricStore) -> Result<(), MetricCollectionError> {
        let r = reqwest::get(url.clone())
            .await
            .map_err(MetricCollectionError::Reqwest)?
            .text()
            .await
            .map_err(MetricCollectionError::Io)?;
        let info = serde_json::from_str::<HashMap<String, ContainerInfo>>(r.as_str())
            .map_err(MetricCollectionError::DeserializeJson)?;
        let containers_info = info.values().collect::<Vec<_>>();

        if let Some(container_info) = containers_info.into_iter().find(|c| c.spec.tezos_node()) {
            let messages = container_info.stats.clone().into_iter().map(MetricMessage).collect();
            storage
                .store_message_array(messages)
                .map_err(MetricCollectionError::StoreMessage)?;
        }
        Ok(())
    }

    // unwrap is safe because joining constant
    let url = settings.cadvisor_url
        .join("api/v1.3/docker")
        .unwrap();

    tokio::spawn(async move {
        loop {
            fetch_and_store(&url, settings.storage.metric())
                .await
                .unwrap_or_else(|e|
                    error!(error = tracing::field::display(&e), "failed to fetch and store metrics")
                );
            time::delay_for(settings.metrics_fetch_interval).await;
        }
    });
}
