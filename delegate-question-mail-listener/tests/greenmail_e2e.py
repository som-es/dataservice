"""Real SMTP -> IMAPS -> listener -> PostgreSQL tests, with disposable Compose services."""
import argparse
import imaplib
import os
from pathlib import Path
import re
import shlex
import shutil
import smtplib
import ssl
import subprocess
import sys
import tempfile
import time
import unittest
import uuid

LISTENER = Path(__file__).resolve().parents[1]
WORKSPACE = LISTENER.parent
USERNAME = "questions@somes.local"
PASSWORD = "questions"


def run(command, **kwargs):
    return subprocess.run(command, check=True, text=True, timeout=120, **kwargs)


def eventually(check, timeout=30):
    deadline = time.monotonic() + timeout
    last_error = None
    while time.monotonic() < deadline:
        try:
            if check():
                return
        except (OSError, imaplib.IMAP4.error, smtplib.SMTPException,
                subprocess.CalledProcessError) as error:
            last_error = error
        time.sleep(0.2)
    raise AssertionError(f"Timed out waiting for condition; last error: {last_error}")


class Environment:
    def __init__(self):
        self.compose = shlex.split(os.environ.get("DOCKER", "docker")) + [
            "compose", "-p", f"mailflow-test-{uuid.uuid4().hex[:12]}",
            "-f", str(LISTENER / "dev/docker-compose.test.yml"),
        ]
        self.temp = tempfile.TemporaryDirectory(prefix="mailflow-test-")
        self.build_root = Path(self.temp.name)

    def sql(self, sql):
        result = run(self.compose + ["exec", "-T", "postgres", "psql", "-X",
                     "-U", "mailflow", "-d", "mailflow_test", "-v", "ON_ERROR_STOP=1",
                     "-Atq"], input=sql, capture_output=True)
        return result.stdout.strip()

    def port(self, service, port):
        output = run(self.compose + ["port", service, str(port)], capture_output=True)
        return int(output.stdout.strip().rsplit(":", 1)[1])

    def imap(self):
        # GreenMail's certificate is self-signed; this context is used only for
        # this runner's disposable container on localhost.
        client = imaplib.IMAP4_SSL("127.0.0.1", self.imap_port,
                                  ssl_context=ssl._create_unverified_context(), timeout=10)
        client.login(USERNAME, PASSWORD)
        client.select("INBOX")
        return client

    def ready(self):
        with smtplib.SMTP("127.0.0.1", self.smtp_port, timeout=3) as smtp:
            smtp.noop()
        with self.imap() as mailbox:
            return mailbox.noop()[0] == "OK"

    def start(self):
        run(self.compose + ["up", "-d", "--wait", "--wait-timeout", "90"])
        self.smtp_port = self.port("mail", 3025)
        self.imap_port = self.port("mail", 3993)
        self.database_url = (
            f"postgres://mailflow:mailflow-test-only@127.0.0.1:{self.port('postgres', 5432)}"
            "/mailflow_test"
        )
        eventually(self.ready, timeout=60)
        self.sql((LISTENER / "tests/fixtures/parents.sql").read_text())
        self.sql((WORKSPACE / "migrations/2026-06-26-120000_delegate_questions/up.sql").read_text())

    def build(self):
        # Compile the real sources in a small temporary workspace. Cargo otherwise
        # resolves unrelated members (including the optional sibling eurovoc repo).
        workspace = self.build_root / "dataservice"
        workspace.mkdir()
        ignored = shutil.ignore_patterns(".git", "target", ".env", ".env.*", "__pycache__")
        for name in ("delegate-question-mail", "delegate-question-mail-listener"):
            shutil.copytree(WORKSPACE / name, workspace / name, ignore=ignored)
        for name in ("scraper", "common-scrapes"):
            shutil.copytree(WORKSPACE.parent / name, self.build_root / name, ignore=ignored)
        original = (WORKSPACE / "Cargo.toml").read_text()
        manifest, count = re.subn(
            r"(?ms)^(members\s*=\s*)\[.*?\]",
            'members = ["delegate-question-mail", "delegate-question-mail-listener"]',
            original, count=1,
        )
        if count != 1:
            raise RuntimeError("Could not locate workspace members in Cargo.toml")
        (workspace / "Cargo.toml").write_text(manifest)
        lock = WORKSPACE / "Cargo.lock"
        if lock.exists():
            shutil.copy2(lock, workspace / "Cargo.lock")
        env = os.environ.copy()
        env.update(DATABASE_URL=self.database_url, SQLX_OFFLINE="false")
        env["CARGO_TARGET_DIR"] = str(WORKSPACE / "target/mailflow-tests")
        for args in (["test", "-p", "delegate-question-mail", "--lib"],
                     ["build", "-p", "delegate-question-mail-listener"]):
            subprocess.run(["cargo", *args], cwd=workspace, env=env, check=True, timeout=1800)
        self.binary = Path(env["CARGO_TARGET_DIR"]) / "debug/delegate-question-mail-listener"

    def command(self, once=True, password=PASSWORD):
        return [str(self.binary), "--database-url", self.database_url,
                "--host", "127.0.0.1", "--port", str(self.imap_port),
                "--username", USERNAME, "--password", password, "--mailbox", "INBOX",
                "--tls-mode", "tls", "--tls-skip-verify", "--poll-seconds", "1",
                *(["--once"] if once else [])]

    def poll(self, success=True, **kwargs):
        result = subprocess.run(self.command(**kwargs), cwd=self.build_root,
                                capture_output=True, text=True, timeout=20,
                                env={**os.environ, "RUST_LOG": "info"})
        if (result.returncode == 0) != success:
            raise AssertionError(f"Unexpected exit {result.returncode}\n{result.stdout}\n{result.stderr}")
        return result

    def close(self, failed):
        try:
            if failed:
                run(self.compose + ["logs", "--no-color", "--tail", "100"])
        finally:
            try:
                run(self.compose + ["down", "--volumes", "--remove-orphans"])
            finally:
                self.temp.cleanup()


class MailflowTests(unittest.TestCase):
    environment = None

    def setUp(self):
        self.e = self.environment
        with self.e.imap() as mailbox:
            mailbox.store("1:*", "+FLAGS", "\\Deleted")
            mailbox.expunge()
        self.e.sql("""
            DROP TRIGGER IF EXISTS fail_answer_status ON delegate_questions;
            DROP FUNCTION IF EXISTS fail_answer_status();
            TRUNCATE delegate_question_answers, delegate_questions RESTART IDENTITY;
        """)

    def question(self):
        message_id = f"<question-{uuid.uuid4()}@somes.local>"
        question_id = self.e.sql(f"""
            INSERT INTO delegate_questions
                (user_id, delegate_id, recipient_email, recipient_kind, recipient_name,
                 subject, body, outgoing_message_id, status)
            VALUES (1, 1, 'delegate@somes.local', 'delegate', 'Test Delegate',
                    'Test question', 'What is your answer?', '{message_id}', 'sent')
            RETURNING id;
        """)
        return int(question_id), message_id

    def send(self, related=None, *, reference=False, body="Danke für die Antwort.",
             message_id=True, sender=True, content_type="text/plain; charset=UTF-8",
             encoding=None):
        headers = ["To: questions@somes.local", "Subject: Re: Test question",
                   f"Content-Type: {content_type}"]
        if sender:
            headers.append("From: Test Delegate <delegate@somes.local>")
        if message_id:
            headers.append(f"Message-ID: <reply-{uuid.uuid4()}@somes.local>")
        if related:
            headers.append(f"{'References' if reference else 'In-Reply-To'}: {related}")
        if encoding:
            headers.append(f"Content-Transfer-Encoding: {encoding}")
        with self.e.imap() as mailbox:
            before = len(mailbox.search(None, "ALL")[1][0].split())
        with smtplib.SMTP("127.0.0.1", self.e.smtp_port, timeout=10) as smtp:
            smtp.sendmail("delegate@somes.local", [USERNAME],
                          ("\r\n".join(headers) + "\r\n\r\n" + body + "\r\n").encode())
        def delivered():
            with self.e.imap() as mailbox:
                return len(mailbox.search(None, "ALL")[1][0].split()) > before
        eventually(delivered)

    def state(self, question_id):
        return self.e.sql(f"""
            SELECT q.status, count(a.id) FROM delegate_questions q
            LEFT JOIN delegate_question_answers a ON a.question_id=q.id
            WHERE q.id={question_id} GROUP BY q.id;
        """)

    def test_reply_is_stored_once_even_after_listener_restart(self):
        question, related = self.question()
        unrelated, _ = self.question()
        self.send(related)
        self.e.poll()
        self.assertEqual(self.state(question), "answered|1")
        self.assertEqual(self.state(unrelated), "sent|0")
        self.assertEqual(self.e.sql("SELECT sender_email, body FROM delegate_question_answers"),
                         "delegate@somes.local|Danke für die Antwort.")
        self.e.poll()
        self.assertEqual(self.state(question), "answered|1")

    def test_references_without_in_reply_to(self):
        question, related = self.question()
        self.send(f"<older@somes.local> {related}", reference=True)
        self.e.poll()
        self.assertEqual(self.state(question), "answered|1")

    def test_invalid_messages_are_ignored(self):
        question, related = self.question()
        for options in ({"related": "<unknown@somes.local>"}, {},
                        {"related": related, "body": ""},
                        {"related": related, "sender": False}):
            with self.subTest(options=options):
                self.send(**options)
                self.e.poll()
                self.assertEqual(self.state(question), "sent|0")

    def test_missing_message_id_is_ignored(self):
        # SMTP servers may insert a Message-ID. IMAP APPEND preserves the exact
        # malformed message, so this case exercises the listener deterministically.
        question, related = self.question()
        raw = (f"From: delegate@somes.local\r\nIn-Reply-To: {related}\r\n"
               "Content-Type: text/plain\r\n\r\nAnswer without ID\r\n").encode()
        with self.e.imap() as mailbox:
            self.assertEqual(mailbox.append("INBOX", None, None, raw)[0], "OK")
        self.e.poll()
        self.assertEqual(self.state(question), "sent|0")

    def test_mime_reply_bodies(self):
        cases = [
            ({"content_type": "text/html", "body": "<p>Hallo &amp; danke</p>"}, "Hallo & danke"),
            ({"encoding": "base64", "body": "SGFsbG8="}, "Hallo"),
            ({"encoding": "quoted-printable", "body": "Gr=C3=BC=C3=9Fe"}, "Grüße"),
            ({"content_type": "multipart/alternative; boundary=test-part",
              "body": "--test-part\r\nContent-Type: text/plain\r\n\r\nDanke\r\n"
                      "> Alte Frage\r\n--test-part\r\nContent-Type: text/html\r\n\r\n"
                      "<p>Danke</p>\r\n--test-part--"}, "Danke"),
        ]
        for options, expected in cases:
            with self.subTest(expected=expected):
                question, related = self.question()
                self.send(related, **options)
                self.e.poll()
                self.assertEqual(self.state(question), "answered|1")
                self.assertEqual(self.e.sql(
                    f"SELECT body FROM delegate_question_answers WHERE question_id={question}"), expected)

    def fail_status_updates(self):
        self.e.sql("""
            CREATE FUNCTION fail_answer_status() RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN RAISE EXCEPTION 'intentional test failure'; END $$;
            CREATE TRIGGER fail_answer_status BEFORE UPDATE ON delegate_questions
            FOR EACH ROW EXECUTE FUNCTION fail_answer_status();
        """)

    def test_failed_status_update_rolls_back_and_can_be_retried(self):
        question, related = self.question()
        self.send(related)
        self.fail_status_updates()
        self.e.poll(success=False)
        self.assertEqual(self.state(question), "sent|0")
        self.e.sql("DROP TRIGGER fail_answer_status ON delegate_questions;")
        self.e.poll()
        self.assertEqual(self.state(question), "answered|1")

    def test_running_listener_retries_after_database_error(self):
        question, related = self.question()
        self.send(related)
        self.fail_status_updates()
        with tempfile.NamedTemporaryFile(mode="w+") as output:
            process = subprocess.Popen(self.e.command(once=False), cwd=self.e.build_root,
                                       stdout=output, stderr=subprocess.STDOUT,
                                       env={**os.environ, "RUST_LOG": "info"})
            try:
                def failed_once():
                    if process.poll() is not None:
                        self.fail(f"Listener exited before retry: {Path(output.name).read_text()}")
                    return "intentional test failure" in Path(output.name).read_text()
                eventually(failed_once)
                self.assertEqual(self.state(question), "sent|0")
                self.e.sql("DROP TRIGGER fail_answer_status ON delegate_questions;")
                eventually(lambda: self.state(question) == "answered|1")
                self.assertIsNone(process.poll())
            finally:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=10)

    def test_once_reports_mailbox_login_failure(self):
        self.e.poll(success=False, password="incorrect-test-password")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    if os.geteuid() == 0 and os.environ.get("SUDO_USER"):
        parser.exit(2, "Run the tests as your normal user so Cargo uses your Rust installation.\n"
                    "Give only Docker sudo access:\n"
                    "  sudo -v\n"
                    "  DOCKER='sudo -n docker' ./test-mailflow.sh  # from dev/\n")
    if shutil.which("cargo") is None:
        parser.exit(2, "Cargo was not found in PATH. Load your Rust environment first\n"
                    "(usually: source ~/.cargo/env), then run the tests again.\n")
    environment = Environment()
    failed = True
    try:
        environment.start()
        environment.build()
        MailflowTests.environment = environment
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(MailflowTests)
        result = unittest.TextTestRunner(verbosity=2).run(suite)
        failed = not result.wasSuccessful()
        return int(failed)
    finally:
        environment.close(failed)


if __name__ == "__main__":
    sys.exit(main())
