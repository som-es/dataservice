#!/usr/bin/env bash
# Simulate a delegate answering one of your questions, through the mail sandbox.
#
#   ./dev/simulate-answer.sh              # newest question
#   ./dev/simulate-answer.sh 7            # question with id 7
#   REPLY_BODY='Kurz und knapp.' ./dev/simulate-answer.sh 7
#
# Needs the GreenMail box from dev/docker-compose.yml and the listener running.
# The only contract: In-Reply-To must equal delegate_questions.outgoing_message_id.
set -euo pipefail
cd "$(dirname "$0")/.."

uri="$(sed -n 's/^DATASERVICE_URL=//p' .env | head -1)"
[[ -n "$uri" ]] || { echo "no DATASERVICE_URL in $(pwd)/.env" >&2; exit 1; }

filter="q.id = (SELECT max(id) FROM delegate_questions)"
[[ "${1:-}" =~ ^[0-9]+$ ]] && filter="q.id = $1"

row="$(psql "$uri" -AtF'|' -c "
  SELECT q.id, q.outgoing_message_id, q.recipient_email, q.recipient_name, q.subject, q.status
  FROM delegate_questions q
  WHERE $filter")"
[[ -n "$row" ]] || { echo "no such question: ${1:-<latest>}" >&2; exit 1; }

IFS='|' read -r qid outgoing recipient name subject status <<<"$row"
echo "question #$qid  status=$status  reply-from=$recipient  in-reply-to=$outgoing"

IN_REPLY_TO="$outgoing" REFERENCES="$outgoing" \
REPLY_FROM="$recipient" REPLY_TO="${REPLY_TO:-questions@somes.local}" \
REPLY_SUBJECT="${REPLY_SUBJECT:-Aw: $subject}" \
REPLY_BODY="${REPLY_BODY:-Vielen Dank fuer die Anfrage. $name meldet sich naechste Woche.}" \
  python3 dev/send-reply.py
