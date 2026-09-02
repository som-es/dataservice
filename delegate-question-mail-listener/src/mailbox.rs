use crate::config::Config;
use crate::ListenerResult;
use imap::ClientBuilder;

pub async fn fetch_messages(config: &Config) -> ListenerResult<Vec<Vec<u8>>> {
    let config = config.clone();
    tokio::task::spawn_blocking(move || fetch_imap_messages(&config)).await?
}

fn fetch_imap_messages(config: &Config) -> ListenerResult<Vec<Vec<u8>>> {
    let client = ClientBuilder::new(&config.host, config.port)
        .mode(config.tls_mode.into())
        .danger_skip_tls_verify(config.tls_skip_verify)
        .connect()?;
    let mut session = client
        .login(&config.username, &config.password)
        .map_err(|error| error.0)?;
    session.select(&config.mailbox)?;

    let uids = session.uid_search("ALL")?;
    let mut messages = Vec::new();
    for uid in uids {
        let fetched = session.uid_fetch(uid.to_string(), "RFC822")?;
        for message in fetched.iter() {
            if let Some(body) = message.body() {
                messages.push(body.to_vec());
            }
        }
    }
    session.logout()?;
    Ok(messages)
}
