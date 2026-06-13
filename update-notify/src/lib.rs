use combx::Index;
use redis::{
    AsyncCommands,
    aio::{ConnectionManager, MultiplexedConnection},
};
use serde::Serialize;

pub async fn notify_update_streamed_simple<T: Serialize>(
    index: Index,
    data: &T,
    con: &mut MultiplexedConnection,
) -> combx::Result<()> {
    let stream_id = index.as_str();
    let vote_result = serde_json::to_string(data)?;
    let _: () = con.xadd(stream_id, "*", &[("data", vote_result)]).await?;
    Ok(())
}

pub async fn notify_update_streamed<T: Serialize>(
    index: Index,
    data: &T,
    con: &mut ConnectionManager,
) -> combx::Result<()> {
    let stream_id = index.as_str();
    let vote_result = serde_json::to_string(data)?;
    let mut retried_after_reconnect = false;

    loop {
        let res: redis::RedisResult<()> = con
            .xadd(stream_id, "*", &[("data", vote_result.as_str())])
            .await;

        match res {
            Ok(()) => return Ok(()),
            Err(e)
                if !retried_after_reconnect
                    && (e.is_connection_dropped()
                        || e.is_io_error()
                        || e.is_unrecoverable_error()) =>
            {
                retried_after_reconnect = true;
                log::warn!(
                    "Redis notification failed due to connection error; retrying after reconnect: {e}"
                );
            }
            Err(e) => return Err(e.into()),
        }
    }
}
