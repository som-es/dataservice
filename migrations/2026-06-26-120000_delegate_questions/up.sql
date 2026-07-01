CREATE TABLE delegate_questions (
    id BIGSERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES somes_user(id),
    delegate_id INTEGER NOT NULL REFERENCES delegates(id),
    recipient_email VARCHAR(255) NOT NULL,
    recipient_kind VARCHAR(16) NOT NULL
        CHECK (recipient_kind IN ('delegate', 'party')),
    recipient_name VARCHAR(255) NOT NULL,
    subject VARCHAR(255) NOT NULL,
    body TEXT NOT NULL,
    outgoing_message_id VARCHAR(998) NOT NULL UNIQUE,
    status VARCHAR(16) NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'sending', 'sent', 'failed', 'answered', 'rejected')),
    sent_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX delegate_questions_delegate_id_created_at_idx
    ON delegate_questions (delegate_id, created_at DESC);

CREATE INDEX delegate_questions_user_id_created_at_idx
    ON delegate_questions (user_id, created_at DESC);

CREATE TABLE delegate_question_answers (
    id BIGSERIAL PRIMARY KEY,
    question_id BIGINT NOT NULL REFERENCES delegate_questions(id),
    sender_email VARCHAR(255) NOT NULL,
    body TEXT NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    raw_message_id VARCHAR(998) NOT NULL UNIQUE
);
