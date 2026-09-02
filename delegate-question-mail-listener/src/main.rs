mod config;
mod env;
mod mailbox;
mod store;

use crate::config::{Config, ListenerArgs};
use clap::Parser;
use delegate_question_mail::parse_incoming_reply;
use sqlx::PgPool;
use std::error::Error;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

pub type ListenerResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[tokio::main]
async fn main() -> ListenerResult<()> {
    let env_files = env::load_env();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    for file in env_files {
        info!(file = %file.display(), "loaded env file");
    }

    let config = Config::from_args(ListenerArgs::parse());
    let pool = PgPool::connect(&config.database_url).await?;

    log_startup(&config);
    loop {
        match mailbox::fetch_messages(&config).await {
            Ok(messages) => {
                for raw_message in messages {
                    let reply = parse_incoming_reply(&raw_message);
                    if let Err(error) = store::store_reply(&pool, reply).await {
                        error!(%error, "could not store incoming reply");
                    }
                }
            }
            Err(error) => error!(%error, "could not fetch incoming mail"),
        }

        tokio::time::sleep(config.poll_interval).await;
    }
}

fn log_startup(config: &Config) {
    info!(
        host = %config.host,
        port = config.port,
        tls_mode = config.tls_mode.as_str(),
        poll_seconds = config.poll_interval.as_secs(),
        "delegate question listener started"
    );
    if config.tls_skip_verify {
        warn!("TLS certificate verification is disabled; never enable this against a production mailbox");
    }
}
