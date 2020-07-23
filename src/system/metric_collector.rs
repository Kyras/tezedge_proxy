// Copyright (c) SimpleStaking and Tezedge Contributors
// SPDX-License-Identifier: MIT

use crate::system::SystemSettings;

/// infinitely performs http requests to cadvisor and put response into db
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
        // perform http GET request 
        let r = reqwest::get(url.clone())
            .await
            .map_err(MetricCollectionError::Reqwest)?
            .text()
            .await
            .map_err(MetricCollectionError::Io)?;
        // deserialize the response as a json object, assume it is `ContainerInfo` map
        let info = serde_json::from_str::<HashMap<String, ContainerInfo>>(r.as_str())
            .map_err(MetricCollectionError::DeserializeJson)?;

        // find the first container that contains tezos node, assume there is single such container
        if let Some(container_info) = info.into_iter().find(|&(_, ref i)| i.tezos_node()) {
            // take stats from the `ContainerInfo` object, wrap it as `MetricMessage`
            // should optimize to noop, because `MetricMessage` is just a newtype
            let messages = container_info.1.stats.into_iter().map(MetricMessage).collect();
            // write into db
            storage
                .store_message_array(messages)
                .map_err(MetricCollectionError::StoreMessage)?;
        }
        Ok(())
    }

    // prepare url to fetch statistics from docker containers
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
            // this interval should be less equal to 
            // `--housekeeping_interval` of the cadvisor in the docker-compose.*.yml config
            time::delay_for(settings.metrics_fetch_interval).await;
        }
    });
}
