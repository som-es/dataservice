# Delegate question mail listener

The listener reads an IMAP mailbox, matches `In-Reply-To` or `References` against
`delegate_questions.outgoing_message_id`, and stores replies in PostgreSQL.
An answer and its question's `answered` status are committed in one transaction.
The unique `raw_message_id` prevents duplicate answers, including after a restart.
Failed writes are retried on the next poll. Messages are not deleted by the listener.

## Automated GreenMail tests

From the dataservice workspace:

```bash
./delegate-question-mail-listener/dev/test-mailflow.sh
```

Requirements:

- Docker Engine with Compose v2, running and accessible to your user.
- Python 3.10+, Cargo/Rust, and the normal native build dependencies of the project.
- The sibling `../scraper` and `../common-scrapes` repositories used by the mail parser.
- Network access on the first run to download container images and Rust dependencies.

No running API, existing database, `.env`, Mailpit, or manual test data is needed.
If Docker on your machine requires sudo, authenticate first and use:

```bash
sudo -v
DOCKER='sudo -n docker' ./delegate-question-mail-listener/dev/test-mailflow.sh
```

Do not run the entire script with `sudo`: Cargo should run as your normal user,
using your Rust installation and build cache. The runner checks this before
starting containers. From the `dev/` directory, use
`DOCKER='sudo -n docker' ./test-mailflow.sh` after `sudo -v`.

The runner:

1. Starts `dev/docker-compose.test.yml` under a unique Compose project name.
   GreenMail and PostgreSQL use randomly assigned ports bound to localhost. The
   database lives in tmpfs; the interactive development containers are unaffected.
2. Waits for PostgreSQL, SMTP and authenticated IMAPS to be ready.
3. Creates minimal user/delegate parent keys and applies the actual
   `2026-06-26-120000_delegate_questions/up.sql` migration. This is a focused
   listener schema, not the application's complete database schema.
4. Copies the real listener, parser and their sibling dependencies into a temporary
   workspace, preserving the workspace dependency declarations. This avoids Cargo
   resolving unrelated workspace members such as `eurovoc`. The original sources,
   lockfile and environment files are not changed. An existing Cargo.lock is used
   as a starting point; build artifacts are cached in `target/mailflow-tests`.
5. Runs the parser's Rust unit tests and builds the actual listener against the
   test database (including SQLx compile-time query validation).
6. Sends real SMTP messages to GreenMail, waits for IMAP delivery, runs the listener,
   and checks the database. Each case resets the mailbox and question tables.
7. Prints the test results, shows container logs on failure, and removes the
   containers and temporary workspace in a `finally` block, including on Ctrl+C.

The Python runner uses only the standard library; it needs no pip packages.
External operations have timeouts, and readiness checks poll with a deadline.
An exit code of zero means all parser and integration tests passed. A killed host
or `SIGKILL` cannot run cleanup; any leftover projects are named `mailflow-test-*`.

### Coverage

- Correct question, sender, body and `answered` status; unrelated questions unchanged.
- Duplicate protection when a new listener process reads the same mail again.
- Matching via `References` without `In-Reply-To`.
- Unknown references, missing references, empty body and missing sender are ignored.
- A message without `Message-ID` is ignored. This malformed case uses IMAP APPEND
  because an SMTP server may automatically add that header.
- HTML entities, Base64, quoted-printable UTF-8 and multipart replies with quoted text.
- A database trigger deliberately rejects the status update: the answer insert
  must roll back, `--once` must fail, and the next attempt must succeed.
- The continuously running listener retries a failed write without a restart.
- Wrong IMAP credentials produce a nonzero exit code in `--once` mode.

These tests cover incoming mail through storage. Creating/approving questions via
HTTP, outgoing API mail delivery, authorization, and search indexing are outside
this suite. The existing API database tests cover a separate part of that flow.
The old POP3/Mailpit test has been replaced; the listener uses IMAP.

## Interactive development mailbox

The existing GreenMail + Roundcube environment is still available separately:

```bash
docker compose -f delegate-question-mail-listener/dev/docker-compose.yml up -d
```

SMTP is on `localhost:2525`, IMAPS on `localhost:10993`, and Roundcube on
`http://localhost:17080`. The test account is `questions@somes.local` with password
`questions`. Configure the listener with your development database:

```env
DATASERVICE_URL=postgres://user:password@localhost/dataservice
MAIL_INBOUND_HOST=127.0.0.1
MAIL_INBOUND_PORT=10993
MAIL_INBOUND_USERNAME=questions@somes.local
MAIL_INBOUND_PASSWORD=questions
MAIL_INBOUND_TLS_MODE=tls
MAIL_INBOUND_TLS_SKIP_VERIFY=true
MAIL_INBOUND_POLL_SECONDS=1
```

The TLS exception is needed only for GreenMail's self-signed development certificate.
Run `cargo run -p delegate-question-mail-listener -- --once` for one poll, or omit
`--once` for continuous polling. A one-shot poll exits nonzero if fetching or storing
mail fails. Invalid/unrelated messages are skipped without failing the poll.

`dev/simulate-answer.sh <question-id>` sends an answer to an existing question using
its stored message ID. It reads `DATASERVICE_URL` from the listener's `.env` and
requires `psql`. `dev/send-reply.py` can also send custom test messages directly.
