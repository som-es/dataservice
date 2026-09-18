use std::sync::Arc;

use qdrant_client::Qdrant;
use somai_client::SomAiClient;

use crate::ai_tasks::{somai_connect, somai_url};

pub fn qdrant_url() -> String {
    std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://127.0.0.1:6334".to_string())
}

pub async fn connect_ai_client() -> Option<Arc<SomAiClient>> {
    let somai_url = somai_url();
    match somai_connect().await {
        Ok(ai_client) => {
            log::info!("Connected to somes ai api at {somai_url}");
            Some(ai_client)
        }
        Err(e) => {
            log::error!("Could not connect to somes ai api at {somai_url}: {e}");
            None
        }
    }
}

pub async fn connect_qdrant_client() -> Option<Qdrant> {
    let qdrant_url = qdrant_url();
    let qdrant_client = match Qdrant::from_url(&qdrant_url)
        .timeout(std::time::Duration::from_secs(30))
        .keep_alive_while_idle()
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(qdrant_client) => qdrant_client,
        Err(e) => {
            log::error!("Could not create qdrant client for {qdrant_url}: {e}");
            return None;
        }
    };

    match qdrant_client.health_check().await {
        Ok(info) => {
            log::info!(
                "Connected to {} {} at {qdrant_url}",
                info.title,
                info.version
            );
            Some(qdrant_client)
        }
        Err(e) => {
            log::error!("Could not connect to qdrant at {qdrant_url}: {e}");
            None
        }
    }
}
