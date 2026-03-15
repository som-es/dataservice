use dotenvy_macro::dotenv;
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, Message,
    SmtpTransport, Transport,
};
use once_cell::sync::Lazy;

// use crate::{EMAIL_EXPIRATION_SECONDS, VERIFICATION_CONTENT, VERIFICATION_SUBJECT};

// env vars?

pub const SMTP_USERNAME: &str = dotenv!("SMTP_USERNAME");
pub const SMTP_PASSWORD: &str = dotenv!("SMTP_PASSWORD");
pub const MAIL_FROM_DISPLAY: &str = dotenv!("MAIL_FROM_DISPLAY");
pub const MAIL_SERVER: &str = dotenv!("MAIL_SERVER");

pub const LEGIS_INIT_EMAIL_TEMPLATE: &str = include_str!("legis_init_email_template.html");
pub const VOTE_RESULT_ENTRY_TEMPLATE: &str = include_str!("vote_result_entry_template.html");
pub const MINISTRIAL_PROP_EMAIL_TEMPLATE: &str =
    include_str!("ministrial_prop_email_template.html");
pub const MINISTRIAL_PROP_ENTRY_TEMPLATE: &str =
    include_str!("ministrial_prop_entry_template.html");

pub static MAILER: Lazy<SmtpTransport> = Lazy::new(|| {
    let creds = Credentials::new(SMTP_USERNAME.to_string(), SMTP_PASSWORD.to_string());
    log::info!("Connecting to email relay...");

    SmtpTransport::starttls_relay(MAIL_SERVER)
        .expect("Email relay not available.")
        .credentials(creds)
        .build()
});

pub fn send_mail(
    mailer: &SmtpTransport,
    mail_to: &str,
    subject: &str,
    content: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let from = format!("somes info <{}>", SMTP_USERNAME).parse()?;
    let to = format!("Recipient <{mail_to}>").parse()?;

    let email = Message::builder()
        .from(from)
        .to(to)
        .subject(subject)
        .header(ContentType::TEXT_HTML)
        .body(content)?;

    mailer.send(&email)?;
    Ok(())
}
