use clap::{Parser, ValueEnum};
use delegate_question_mail::{parse_incoming_reply, IncomingReply};
use imap::{ClientBuilder, ConnectionMode};
use sqlx::PgPool;
use std::{error::Error, time::Duration};
#[cfg(any(test, feature = "pop3"))]
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
};
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

type ListenerResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum MailProtocol {
    Imap,
    #[cfg(any(test, feature = "pop3"))]
    Pop3,
}

#[derive(Debug, Parser)]
#[command(about = "Polls an inbound mailbox for delegate question replies")]
struct ListenerArgs {
    #[arg(long, env = "DATASERVICE_URL")]
    database_url: String,
    #[arg(long, env = "MAIL_INBOUND_PROTOCOL", value_enum, default_value_t = MailProtocol::Imap)]
    protocol: MailProtocol,
    #[arg(long, env = "MAIL_INBOUND_HOST")]
    host: String,
    #[arg(long, env = "MAIL_INBOUND_PORT")]
    port: Option<u16>,
    #[arg(long, env = "MAIL_INBOUND_USERNAME")]
    username: String,
    #[arg(long, env = "MAIL_INBOUND_PASSWORD")]
    password: String,
    #[arg(long, env = "MAIL_INBOUND_MAILBOX", default_value = "INBOX")]
    mailbox: String,
    #[arg(long, env = "MAIL_INBOUND_POLL_SECONDS", default_value_t = 15)]
    poll_seconds: u64,
}

#[derive(Clone)]
struct ListenerConfig {
    database_url: String,
    protocol: MailProtocol,
    host: String,
    port: u16,
    username: String,
    password: String,
    mailbox: String,
    poll_interval: Duration,
}

impl ListenerConfig {
    fn from_args(args: ListenerArgs) -> Self {
        let port = args.port.unwrap_or_else(|| {
            match args.protocol {
                MailProtocol::Imap => "993".to_string(),
                #[cfg(any(test, feature = "pop3"))]
                MailProtocol::Pop3 => "1110".to_string(),
            }
            .parse()
            .expect("default port is valid")
        });

        Self {
            database_url: args.database_url,
            protocol: args.protocol,
            host: args.host,
            port,
            username: args.username,
            password: args.password,
            mailbox: args.mailbox,
            poll_interval: Duration::from_secs(args.poll_seconds),
        }
    }
}

#[tokio::main]
async fn main() -> ListenerResult<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = ListenerConfig::from_args(ListenerArgs::parse());
    let pool = PgPool::connect(&config.database_url).await?;

    log_startup(&config);
    loop {
        match fetch_messages(&config).await {
            Ok(messages) => {
                for raw_message in messages {
                    let reply = parse_incoming_reply(&raw_message);
                    if let Err(error) = store_reply(&pool, reply).await {
                        error!(%error, "could not store incoming reply");
                    }
                }
            }
            Err(error) => error!(%error, "could not fetch incoming mail"),
        }

        tokio::time::sleep(config.poll_interval).await;
    }
}

fn log_startup(config: &ListenerConfig) {
    let protocol = match config.protocol {
        MailProtocol::Imap => "IMAP",
        #[cfg(any(test, feature = "pop3"))]
        MailProtocol::Pop3 => "POP3",
    };
    info!(
        protocol,
        host = %config.host,
        port = config.port,
        poll_seconds = config.poll_interval.as_secs(),
        "delegate question listener started"
    );
    #[cfg(any(test, feature = "pop3"))]
    if matches!(config.protocol, MailProtocol::Pop3) {
        warn!("POP3 mode is intended for local Mailpit testing; use IMAP for production mailboxes");
    }
}

async fn fetch_messages(config: &ListenerConfig) -> ListenerResult<Vec<Vec<u8>>> {
    let config = config.clone();
    tokio::task::spawn_blocking(move || match config.protocol {
        MailProtocol::Imap => fetch_imap_messages(&config),
        #[cfg(any(test, feature = "pop3"))]
        MailProtocol::Pop3 => fetch_pop3_messages(&config),
    })
    .await?
}

fn fetch_imap_messages(config: &ListenerConfig) -> ListenerResult<Vec<Vec<u8>>> {
    let client = ClientBuilder::new(&config.host, config.port)
        .mode(ConnectionMode::AutoTls)
        .connect()?;
    let mut session = client
        .login(&config.username, &config.password)
        .map_err(|error| error.0)?;
    session.select(&config.mailbox)?;

    let uids = session.uid_search("ALL")?;
    let mut messages = Vec::new();
    for uid in uids {
        let fetched = session.uid_fetch(uid.to_string(), "RFC822")?;
        for message in fetched.iter() {
            if let Some(body) = message.body() {
                messages.push(body.to_vec());
            }
        }
    }
    session.logout()?;
    Ok(messages)
}

#[cfg(any(test, feature = "pop3"))]
fn fetch_pop3_messages(config: &ListenerConfig) -> ListenerResult<Vec<Vec<u8>>> {
    let stream = TcpStream::connect((&*config.host, config.port))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;

    expect_ok(&mut reader)?;
    pop3_command(
        &mut writer,
        &mut reader,
        &format!("USER {}", config.username),
    )?;
    pop3_command(
        &mut writer,
        &mut reader,
        &format!("PASS {}", config.password),
    )?;

    writer.write_all(b"LIST\r\n")?;
    writer.flush()?;
    expect_ok(&mut reader)?;
    let message_numbers = read_pop3_multiline(&mut reader)?
        .into_iter()
        .filter_map(|line| line.split_whitespace().next()?.parse::<u32>().ok())
        .collect::<Vec<_>>();

    let mut messages = Vec::new();
    for number in message_numbers {
        writer.write_all(format!("RETR {number}\r\n").as_bytes())?;
        writer.flush()?;
        expect_ok(&mut reader)?;
        let message = read_pop3_multiline(&mut reader)?.join("\r\n");
        messages.push(message.into_bytes());
    }

    writer.write_all(b"QUIT\r\n")?;
    writer.flush()?;
    Ok(messages)
}

#[cfg(any(test, feature = "pop3"))]
fn pop3_command(
    writer: &mut TcpStream,
    reader: &mut BufReader<TcpStream>,
    command: &str,
) -> ListenerResult<()> {
    writer.write_all(format!("{command}\r\n").as_bytes())?;
    writer.flush()?;
    expect_ok(reader)
}

#[cfg(any(test, feature = "pop3"))]
fn expect_ok(reader: &mut BufReader<TcpStream>) -> ListenerResult<()> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    if line.starts_with("+OK") {
        Ok(())
    } else {
        Err(format!("POP3 server rejected command: {}", line.trim()).into())
    }
}

#[cfg(any(test, feature = "pop3"))]
fn read_pop3_multiline(reader: &mut BufReader<TcpStream>) -> ListenerResult<Vec<String>> {
    let mut lines = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let line = line.trim_end_matches(['\r', '\n']);
        if line == "." {
            return Ok(lines);
        }
        lines.push(line.strip_prefix("..").unwrap_or(line).to_string());
    }
}

async fn store_reply(pool: &PgPool, reply: IncomingReply) -> ListenerResult<()> {
    let Some(raw_message_id) = reply.message_id else {
        warn!("skipping incoming mail without Message-ID");
        return Ok(());
    };
    if reply.related_message_ids.is_empty() {
        debug!(
            raw_message_id,
            "skipping incoming mail without In-Reply-To/References"
        );
        return Ok(());
    }
    if reply.body.is_empty() {
        warn!(raw_message_id, "skipping empty reply");
        return Ok(());
    }
    let Some(sender_email) = reply.sender_email else {
        warn!(
            raw_message_id,
            "skipping incoming mail without From address"
        );
        return Ok(());
    };

    let question_id = sqlx::query_scalar!(
        "
        SELECT id
        FROM delegate_questions
        WHERE outgoing_message_id = ANY($1)
        ORDER BY id DESC
        LIMIT 1
        ",
        &reply.related_message_ids
    )
    .fetch_optional(pool)
    .await?;

    let Some(question_id) = question_id else {
        debug!(
            raw_message_id,
            related_message_ids = ?reply.related_message_ids,
            "skipping incoming mail without matching delegate question"
        );
        return Ok(());
    };

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
    .fetch_optional(pool)
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
        .execute(pool)
        .await?;
        info!(
            question_id,
            raw_message_id, "stored delegate question answer"
        );
    } else {
        debug!(question_id, raw_message_id, "reply was already stored");
    }

    Ok(())
}
