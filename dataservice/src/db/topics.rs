use combx::Topic;
use sqlx::{Acquire, Postgres, Transaction};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TopicType {
    Standard,
    Eurovoc,
    OtherKeyword,
}

pub enum TableType {
    LegislativeInitiative,
    MinisterialProposal,
}

pub struct TableInfo {
    pub id_field: String,
    pub unique_table: Option<String>,
    pub table_name: String,
}

fn get_table_info(topic_type: &TopicType, table_type: &TableType) -> TableInfo {
    let table_prefix = match topic_type {
        TopicType::Standard => "topics",
        TopicType::Eurovoc => "eurovoc_topics",
        TopicType::OtherKeyword => "other_keyword_topics",
    };

    let (table_suffix, id_field) = match table_type {
        TableType::LegislativeInitiative => ("legis_init", "legislative_initiatives_id"),
        TableType::MinisterialProposal => ("ministrial_proposals", "ministrial_proposal_id"),
    };

    TableInfo {
        table_name: format!("{table_prefix}_{table_suffix}"),
        unique_table: None,
        id_field: id_field.to_string(),
    }
}

pub async fn insert_topics_generic_batch<'a>(
    tx: &mut Transaction<'a, Postgres>,
    topics: &[Topic],
    db_entry_id: i32,
    topic_type: TopicType,
    table_type: TableType,
) -> sqlx::Result<()> {
    if topics.is_empty() {
        return Ok(());
    }

    let info = get_table_info(&topic_type, &table_type);
    let mut sp = tx.begin().await?;

    let topic_names: Vec<String> = topics.iter().map(|t| t.topic.clone()).collect();
    let ids: Vec<i32> = vec![db_entry_id; topics.len()];

    if let Some(unique_table) = info.unique_table
        && TopicType::Eurovoc != topic_type
    {
        let unique_sql = format!(
            "INSERT INTO {unique_table} (topic_name)
             SELECT * FROM UNNEST($1::text[])
             ON CONFLICT (topic_name) DO NOTHING"
        );

        sqlx::query(&unique_sql)
            .bind(&topic_names)
            .execute(&mut *sp)
            .await?;
    }

    let insert_topics = format!(
        "INSERT INTO {} (topic, {})
         SELECT * FROM UNNEST($1::text[], $2::int4[])
         ON CONFLICT ({}, topic)
         DO UPDATE SET topic = EXCLUDED.topic",
        info.table_name, info.id_field, info.id_field
    );

    sqlx::query(&insert_topics)
        .bind(&topic_names)
        .bind(&ids)
        .execute(&mut *sp)
        .await?;

    sp.commit().await?;
    Ok(())
}
