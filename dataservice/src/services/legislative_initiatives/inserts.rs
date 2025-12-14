mod documents;
mod references;
mod topics;
mod votes;

pub use documents::*;
pub use references::*;
use somes_scraper::RelatedDelegate;
use sqlx::{Acquire, Postgres, Transaction};
pub use topics::*;
pub use votes::*;

pub async fn insert_delegates_batch<'a>(
    tx: &mut Transaction<'a, Postgres>,
    delegates: &[RelatedDelegate],
    legis_init_id: i32,
) -> sqlx::Result<()> {
    if delegates.is_empty() {
        return Ok(());
    }

    let mut sp = tx.begin().await?;

    let ids = vec![legis_init_id; delegates.len()];
    let delegate_ids: Vec<i32> = delegates.iter().map(|d| d.delegate_id as i32).collect();
    let texts: Vec<Option<String>> = delegates.iter().map(|d| d.text.clone()).collect();

    let res = sqlx::query!(
        "INSERT INTO legis_init_delegates (legis_init_id, delegate_id, delegate_text) 
         SELECT * FROM UNNEST($1::int4[], $2::int4[], $3::text[]) 
         ON CONFLICT (legis_init_id, delegate_id) 
         DO UPDATE SET delegate_text = EXCLUDED.delegate_text",
        &ids,
        &delegate_ids,
        &texts as &[Option<String>]
    )
    .execute(&mut *sp)
    .await;

    match res {
        Ok(_) => {
            sp.commit().await?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}
