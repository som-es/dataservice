use combx::DbReference;
use sqlx::{Acquire, Postgres, Transaction};

pub async fn insert_references_batch<'a>(
    tx: &mut Transaction<'a, Postgres>,
    references: &[DbReference],
    legislative_initiatives_id: i32,
) -> sqlx::Result<()> {
    if references.is_empty() {
        return Ok(());
    }

    let mut sp = tx.begin().await?;

    let init_ids = vec![legislative_initiatives_id; references.len()];
    let gps: Vec<String> = references.iter().map(|r| r.gp.clone()).collect();
    let ityps: Vec<String> = references.iter().map(|r| r.ityp.clone()).collect();
    let inrs: Vec<i32> = references.iter().map(|r| r.inr as i32).collect();

    let res = sqlx::query!(
        "INSERT INTO legis_inits_refs (origin_legis_init_id, ref_gp, ref_ityp, ref_inr)
         SELECT * FROM UNNEST($1::int4[], $2::text[], $3::text[], $4::int4[])
         ON CONFLICT DO NOTHING",
        &init_ids,
        &gps,
        &ityps,
        &inrs
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
