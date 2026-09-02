use clap::{Parser, ValueEnum};
use imap::ConnectionMode;
use std::time::Duration;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum TlsMode {
    AutoTls,
    Auto,
    StartTls,
    Tls,
    Plaintext,
}

impl TlsMode {
    pub fn as_str(self) -> &'static str {
        match self {
            TlsMode::AutoTls => "auto-tls",
            TlsMode::Auto => "auto",
            TlsMode::StartTls => "starttls",
            TlsMode::Tls => "tls",
            TlsMode::Plaintext => "plaintext",
        }
    }
}

impl From<TlsMode> for ConnectionMode {
    fn from(mode: TlsMode) -> Self {
        match mode {
            TlsMode::AutoTls => ConnectionMode::AutoTls,
            TlsMode::Auto => ConnectionMode::Auto,
            TlsMode::StartTls => ConnectionMode::StartTls,
            TlsMode::Tls => ConnectionMode::Tls,
            TlsMode::Plaintext => ConnectionMode::Plaintext,
        }
    }
}

#[derive(Debug, Parser)]
#[command(about = "Polls an inbound mailbox for delegate question replies")]
pub struct ListenerArgs {
    #[arg(long, env = "DATASERVICE_URL")]
    database_url: String,
    #[arg(long, env = "MAIL_INBOUND_HOST")]
    host: String,
    #[arg(long, env = "MAIL_INBOUND_PORT", default_value_t = 993)]
    port: u16,
    #[arg(long, env = "MAIL_INBOUND_USERNAME")]
    username: String,
    #[arg(long, env = "MAIL_INBOUND_PASSWORD")]
    password: String,
    #[arg(long, env = "MAIL_INBOUND_MAILBOX", default_value = "INBOX")]
    mailbox: String,
    #[arg(long, env = "MAIL_INBOUND_POLL_SECONDS", default_value_t = 15)]
    poll_seconds: u64,
    #[arg(long, env = "MAIL_INBOUND_TLS_MODE", value_enum, default_value_t = TlsMode::AutoTls)]
    tls_mode: TlsMode,
    #[arg(
        long,
        env = "MAIL_INBOUND_TLS_SKIP_VERIFY",
        default_value_t = false,
        hide = true
    )]
    tls_skip_verify: bool,
}

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub mailbox: String,
    pub poll_interval: Duration,
    pub tls_mode: TlsMode,
    pub tls_skip_verify: bool,
}

impl Config {
    pub fn from_args(args: ListenerArgs) -> Self {
        Self {
            database_url: args.database_url,
            host: args.host,
            port: args.port,
            username: args.username,
            password: args.password,
            mailbox: args.mailbox,
            poll_interval: Duration::from_secs(args.poll_seconds),
            tls_mode: args.tls_mode,
            tls_skip_verify: args.tls_skip_verify,
        }
    }
}
