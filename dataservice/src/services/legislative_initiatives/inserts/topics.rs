use combx::Topic;
use sqlx::{Acquire, Postgres, Transaction};

pub async fn insert_topics_batch<'a>(
    tx: &mut Transaction<'a, Postgres>,
    topics: &[Topic],
    legis_init_id: i32,
) -> sqlx::Result<()> {
    if topics.is_empty() {
        return Ok(());
    }

    let mut sp = tx.begin().await?;

    let topic_names: Vec<String> = topics.iter().map(|t| t.topic.clone()).collect();
    let ids = vec![legis_init_id; topics.len()];

    let r1 = sqlx::query!(
        "INSERT INTO unique_topics (topic_name) 
         SELECT * FROM UNNEST($1::text[]) 
         ON CONFLICT (topic_name) DO NOTHING",
        &topic_names
    )
    .execute(&mut *sp)
    .await;

    if let Err(e) = r1 {
        return Err(e);
    }

    let r2 = sqlx::query!(
        "INSERT INTO topics_legis_init (topic, legislative_initiatives_id) 
         SELECT * FROM UNNEST($1::text[], $2::int4[]) 
         ON CONFLICT (legislative_initiatives_id, topic) 
         DO UPDATE SET topic = EXCLUDED.topic",
        &topic_names,
        &ids
    )
    .execute(&mut *sp)
    .await;

    match r2 {
        Ok(_) => {
            sp.commit().await?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub async fn insert_eurovoc_topics_batch<'a>(
    tx: &mut Transaction<'a, Postgres>,
    topics: &[Topic],
    legis_init_id: i32,
) -> sqlx::Result<()> {
    if topics.is_empty() {
        return Ok(());
    }

    let mut sp = tx.begin().await?;

    let topic_names: Vec<String> = topics.iter().map(|t| t.topic.clone()).collect();
    let ids = vec![legis_init_id; topics.len()];

    let r1 = sqlx::query!(
        "INSERT INTO unique_eurovoc_topics (topic_name) 
         SELECT * FROM UNNEST($1::text[]) 
         ON CONFLICT (topic_name) DO NOTHING",
        &topic_names
    )
    .execute(&mut *sp)
    .await;

    if let Err(e) = r1 {
        return Err(e);
    }

    let r2 = sqlx::query!(
        "INSERT INTO eurovoc_topics_legis_init (topic, legislative_initiatives_id) 
         SELECT * FROM UNNEST($1::text[], $2::int4[]) 
         ON CONFLICT (legislative_initiatives_id, topic) 
         DO UPDATE SET topic = EXCLUDED.topic",
        &topic_names,
        &ids
    )
    .execute(&mut *sp)
    .await;

    match r2 {
        Ok(_) => {
            sp.commit().await?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub async fn insert_other_keyword_topics_batch<'a>(
    tx: &mut Transaction<'a, Postgres>,
    topics: &[Topic],
    legis_init_id: i32,
) -> sqlx::Result<()> {
    if topics.is_empty() {
        return Ok(());
    }

    let mut sp = tx.begin().await?;

    let topic_names: Vec<String> = topics.iter().map(|t| t.topic.clone()).collect();
    let ids = vec![legis_init_id; topics.len()];

    let res = sqlx::query!(
        "INSERT INTO other_keyword_topics_legis_init (topic, legislative_initiatives_id) 
         SELECT * FROM UNNEST($1::text[], $2::int4[]) 
         ON CONFLICT (legislative_initiatives_id, topic) 
         DO UPDATE SET topic = EXCLUDED.topic",
        &topic_names,
        &ids
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
