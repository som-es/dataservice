use somes_common_lib::Document;
use sqlx::{Acquire, Postgres, Transaction};

pub async fn insert_ministerial_documents_batch<'a>(
    tx: &mut Transaction<'a, Postgres>,
    documents: &[Document],
    legis_init_id: i32,
) -> sqlx::Result<()> {
    let mut sp = tx.begin().await?;

    sqlx::query!(
        "delete from ministrial_proposals_documents where ministrial_proposal_id = $1",
        legis_init_id
    )
    .execute(&mut *sp)
    .await?;

    let documents: Vec<&Document> = documents
        .iter()
        .filter(|d| d.document_url.is_some())
        .collect();

    if !documents.is_empty() {
        let titles: Vec<Option<String>> = documents.iter().map(|d| d.title.clone()).collect();
        let types: Vec<Option<String>> =
            documents.iter().map(|d| d.document_type.clone()).collect();
        let urls: Vec<Option<String>> = documents.iter().map(|d| d.document_url.clone()).collect();
        let ids = vec![legis_init_id; titles.len()];

        sqlx::query!(
            "
                INSERT INTO ministrial_proposals_documents (ministrial_proposal_id, title, document_type, document_url) 
                SELECT * FROM UNNEST($1::int4[], $2::text[], $3::text[], $4::text[])
                ON CONFLICT (document_url, ministrial_proposal_id)
                DO UPDATE SET
                    title = EXCLUDED.title,
                    document_type = EXCLUDED.document_type,
                    document_url = EXCLUDED.document_url
            ",
            &ids, &titles as &[Option<String>], &types as &[Option<String>], &urls as &[Option<String>]
        )
        .execute(&mut *sp)
        .await?;
    }

    sp.commit().await?;
    Ok(())
}
