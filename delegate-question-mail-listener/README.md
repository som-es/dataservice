# Delegate Question Mail Listener

> Historical local Mailpit notes: upstream has removed POP3 support and the
> `pop3` feature. The POP3 commands below and the retained `tests/mailpit_e2e.rs`
> test cannot currently run. The listener now uses IMAP; `--once` remains supported.
> Upstream also caches message IDs during a run, so the retry description below
> no longer applies to failures within the same process.

This service polls an inbound mailbox, matches replies through `In-Reply-To` and `References`, and stores matching answers in the dataservice database.

The service shares mail parsing and message ID handling with the API through the `delegate-question-mail` library crate.

For production mailboxes, prefer IMAP. POP3 support is kept for local Mailpit testing.
Messages are not deleted from the mailbox by this service. If storing an answer fails, the same inbound message can be retried on the next poll. Duplicate answers are prevented by the `raw_message_id` unique constraint.

## Local Mailpit

Mailpit is only used as a local test mailbox. Start it with POP3 enabled and configure the listener as follows:

```env
MAIL_INBOUND_PROTOCOL=pop3
MAIL_INBOUND_HOST=127.0.0.1
MAIL_INBOUND_PORT=1110
MAIL_INBOUND_USERNAME=test
MAIL_INBOUND_PASSWORD=test
MAIL_INBOUND_POLL_SECONDS=5
```

The existing `DATASERVICE_URL` variable is also required. Start the listener from the workspace root:

```bash
cargo run -p delegate-question-mail-listener --features pop3
```

The same values can also be passed as CLI flags:

```bash
cargo run -p delegate-question-mail-listener --features pop3 -- \
  --database-url postgres://postgres:postgres@127.0.0.1/dataservice \
  --protocol pop3 \
  --host 127.0.0.1 \
  --port 1110 \
  --username test \
  --password test
```

## Production IMAP

Use a dedicated inbound mailbox for question replies:

```env
MAIL_INBOUND_PROTOCOL=imap
MAIL_INBOUND_HOST=imap.example.org
MAIL_INBOUND_PORT=993
MAIL_INBOUND_USERNAME=questions@example.org
MAIL_INBOUND_PASSWORD=replace-me
MAIL_INBOUND_MAILBOX=INBOX
MAIL_INBOUND_POLL_SECONDS=15
```

The listener does not send mail. It only reads incoming messages and records replies whose headers reference a stored outgoing question message ID.

## Mailpit end-to-end test

The ignored integration test sends a real SMTP message to Mailpit, polls it through POP3,
and verifies the stored answer, question status, and duplicate protection in PostgreSQL.
It selects an existing user and delegate, creates isolated test records, and removes them
again before finishing.

Start Mailpit with SMTP on port `1025` and POP3 on port `1110`, ensure `DATABASE_URL`
points to a migrated local dataservice database, then run:

```bash
cargo test -p delegate-question-mail-listener --features pop3 \
  --test mailpit_e2e -- --ignored --nocapture
```

For a single manual poll, pass `--once` to the listener.
