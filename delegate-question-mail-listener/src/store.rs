use crate::ListenerResult;
use delegate_question_mail::IncomingReply;
use sqlx::PgPool;
use tracing::{debug, info, warn};

#[derive(Debug)]
struct Question {
    id: i64,
    recipient_email: String,
}

/// Returns true only when the reply is stored or already present in the database.
/// Skipped replies remain eligible for a later poll (e.g. a not-yet-visible question).
pub async fn store_reply(pool: &PgPool, reply: IncomingReply) -> ListenerResult<bool> {
    let Some(raw_message_id) = reply.message_id else {
        warn!("skipping incoming mail without Message-ID");
        return Ok(false);
    };
    if reply.related_message_ids.is_empty() {
        debug!(
            raw_message_id,
            "skipping incoming mail without In-Reply-To/References"
        );
        return Ok(false);
    }
    if reply.body.is_empty() {
        warn!(raw_message_id, "skipping empty reply");
        return Ok(false);
    }
    let Some(sender_email) = reply.sender_email else {
        warn!(
            raw_message_id,
            "skipping incoming mail without From address"
        );
        return Ok(false);
    };

    let question = sqlx::query_as!(
        Question,
        "
        SELECT id, recipient_email
        FROM delegate_questions
        WHERE outgoing_message_id = ANY($1)
        ORDER BY id DESC
        LIMIT 1
        ",
        &reply.related_message_ids
    )
    .fetch_optional(pool)
    .await?;

    let Some(question) = question else {
        debug!(
            raw_message_id,
            related_message_ids = ?reply.related_message_ids,
            "skipping incoming mail without matching delegate question"
        );
        return Ok(false);
    };

    /*if !question.recipient_email.eq_ignore_ascii_case(&sender_email) {
        warn!(
            raw_message_id,
            sender_email,
            recipient_email = %question.recipient_email,
            question_id = question.id,
            "skipping incoming mail from an address that was not asked"
        );
        return Ok(false);
    }*/

    let question_id = question.id;

    let mut transaction = pool.begin().await?;
    let inserted = sqlx::query_scalar!(
        "
        INSERT INTO delegate_question_answers (question_id, sender_email, body, raw_message_id)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (raw_message_id) DO NOTHING
        RETURNING id
        ",
        question_id,
        sender_email,
        reply.body,
        raw_message_id
    )
    .fetch_optional(&mut *transaction)
    .await?;

    if inserted.is_some() {
        sqlx::query!(
            "
            UPDATE delegate_questions
            SET status = 'answered', updated_at = NOW()
            WHERE id = $1
            ",
            question_id
        )
        .execute(&mut *transaction)
        .await?;
    }

    transaction.commit().await?;
    if inserted.is_some() {
        info!(
            question_id,
            raw_message_id, "stored delegate question answer"
        );
    } else {
        debug!(question_id, raw_message_id, "reply was already stored");
    }

    Ok(true)
}
