#![cfg(feature = "pop3")]

use delegate_question_mail::new_question_message_id;
use sqlx::PgPool;
use std::{
    env,
    error::Error,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    process::Command,
};

type TestResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[tokio::test]
#[ignore = "requires a migrated local PostgreSQL database and Mailpit on ports 1025/1110"]
async fn stores_a_real_mailpit_reply_once() -> TestResult<()> {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL")
        .or_else(|_| env::var("DATASERVICE_URL"))
        .map_err(|_| "DATABASE_URL or DATASERVICE_URL must be set")?;
    let pool = PgPool::connect(&database_url).await?;

    let user_id: i32 = sqlx::query_scalar("SELECT id FROM somes_user ORDER BY id LIMIT 1")
        .fetch_optional(&pool)
        .await?
        .ok_or("the test database needs at least one somes_user row")?;
    let delegate_id: i32 = sqlx::query_scalar("SELECT id FROM delegates ORDER BY id LIMIT 1")
        .fetch_optional(&pool)
        .await?
        .ok_or("the test database needs at least one delegates row")?;

    let outgoing_message_id = new_question_message_id();
    let raw_reply_message_id = new_question_message_id().replace("question-", "reply-");
    let question_id: i64 = sqlx::query_scalar(
        "
        INSERT INTO delegate_questions
            (user_id, delegate_id, recipient_email, recipient_kind, recipient_name,
             subject, body, outgoing_message_id, status)
        VALUES ($1, $2, $3, 'delegate', $4, $5, $6, $7, 'sent')
        RETURNING id
        ",
    )
    .bind(user_id)
    .bind(delegate_id)
    .bind("recipient@example.test")
    .bind("Mailpit E2E recipient")
    .bind("Mailpit E2E question")
    .bind("Can this reply be stored?")
    .bind(&outgoing_message_id)
    .fetch_one(&pool)
    .await?;

    let test_result = run_mail_flow(
        &pool,
        &database_url,
        question_id,
        &outgoing_message_id,
        &raw_reply_message_id,
    )
    .await;

    sqlx::query("DELETE FROM delegate_question_answers WHERE question_id = $1")
        .bind(question_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM delegate_questions WHERE id = $1")
        .bind(question_id)
        .execute(&pool)
        .await?;

    test_result
}

async fn run_mail_flow(
    pool: &PgPool,
    database_url: &str,
    question_id: i64,
    outgoing_message_id: &str,
    raw_reply_message_id: &str,
) -> TestResult<()> {
    send_smtp_reply(outgoing_message_id, raw_reply_message_id)?;
    run_listener_once(database_url)?;

    let (body, sender_email): (String, String) = sqlx::query_as(
        "SELECT body, sender_email FROM delegate_question_answers WHERE question_id = $1",
    )
    .bind(question_id)
    .fetch_one(pool)
    .await?;
    let status: String = sqlx::query_scalar("SELECT status FROM delegate_questions WHERE id = $1")
        .bind(question_id)
        .fetch_one(pool)
        .await?;

    if body != "Automated Mailpit reply" {
        return Err(format!("unexpected stored body: {body:?}").into());
    }
    if sender_email != "sender@example.test" {
        return Err(format!("unexpected sender: {sender_email:?}").into());
    }
    if status != "answered" {
        return Err(format!("unexpected question status: {status:?}").into());
    }

    run_listener_once(database_url)?;
    let answer_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM delegate_question_answers WHERE question_id = $1")
            .bind(question_id)
            .fetch_one(pool)
            .await?;
    if answer_count != 1 {
        return Err(format!("expected one answer after retry, found {answer_count}").into());
    }

    Ok(())
}

fn run_listener_once(database_url: &str) -> TestResult<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_delegate-question-mail-listener"))
        .args([
            "--database-url",
            database_url,
            "--protocol",
            "pop3",
            "--host",
            "127.0.0.1",
            "--port",
            "1110",
            "--username",
            "test",
            "--password",
            "test",
            "--once",
        ])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "listener failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(())
}

fn send_smtp_reply(outgoing_message_id: &str, raw_reply_message_id: &str) -> TestResult<()> {
    let stream = TcpStream::connect(("127.0.0.1", 1025))
        .map_err(|error| format!("could not connect to Mailpit SMTP on port 1025: {error}"))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;

    expect_smtp(&mut reader, 220)?;
    smtp_command(&mut writer, &mut reader, "EHLO localhost", 250)?;
    smtp_command(
        &mut writer,
        &mut reader,
        "MAIL FROM:<sender@example.test>",
        250,
    )?;
    smtp_command(
        &mut writer,
        &mut reader,
        "RCPT TO:<questions@example.test>",
        250,
    )?;
    smtp_command(&mut writer, &mut reader, "DATA", 354)?;

    write!(
        writer,
        "From: Test Sender <sender@example.test>\r\n\
         To: questions@example.test\r\n\
         Subject: Re: Mailpit E2E question\r\n\
         Message-ID: {raw_reply_message_id}\r\n\
         In-Reply-To: {outgoing_message_id}\r\n\
         References: {outgoing_message_id}\r\n\
         Content-Type: text/plain; charset=UTF-8\r\n\
         \r\n\
         Automated Mailpit reply\r\n\
         .\r\n"
    )?;
    writer.flush()?;
    expect_smtp(&mut reader, 250)?;
    smtp_command(&mut writer, &mut reader, "QUIT", 221)?;
    Ok(())
}

fn smtp_command(
    writer: &mut TcpStream,
    reader: &mut BufReader<TcpStream>,
    command: &str,
    expected_status: u16,
) -> TestResult<()> {
    writeln!(writer, "{command}\r")?;
    writer.flush()?;
    expect_smtp(reader, expected_status)
}

fn expect_smtp(reader: &mut BufReader<TcpStream>, expected_status: u16) -> TestResult<()> {
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line.len() < 4 {
            return Err(format!("invalid SMTP response: {line:?}").into());
        }
        let status: u16 = line[..3].parse()?;
        if status != expected_status {
            return Err(format!("unexpected SMTP response: {line}").into());
        }
        if line.as_bytes()[3] == b' ' {
            return Ok(());
        }
    }
}
