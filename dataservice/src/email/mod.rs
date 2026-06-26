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
pub const TEST_MAIL_RECEIVER: &str = dotenv!("TEST_MAIL_RECEIVER");

pub const LEGIS_INIT_EMAIL_TEMPLATE: &str = include_str!("legis_init_email_template.html");
pub const VOTE_RESULT_ENTRY_TEMPLATE: &str = include_str!("vote_result_entry_template.html");
pub const MINISTRIAL_PROP_EMAIL_TEMPLATE: &str =
    include_str!("ministrial_prop_email_template.html");
pub const MINISTRIAL_PROP_ENTRY_TEMPLATE: &str =
    include_str!("ministrial_prop_entry_template.html");
pub const DECREE_EMAIL_TEMPLATE: &str = include_str!("decree_email_template.html");
pub const DECREE_ENTRY_TEMPLATE: &str = include_str!("decree_entry_template.html");
pub const DELEGATE_ACTIVITY_TEMPLATE: &str = include_str!("delegate_activity_template.html");
pub const DELEGATE_ENTRY_TEMPLATE: &str = include_str!("delegate_entry_template.html");

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

#[cfg(test)]
mod tests {
    use super::*;

    const DUMMY_DELEGATE_IMG: &str = "https://somes.at/images/somes_logo.png";

    fn make_vote_entry() -> String {
        VOTE_RESULT_ENTRY_TEMPLATE
            .replace("{*VOTE_RESULT_SOMES_URL*}", "https://somes.at/vote_result/XXVIII/A/164")
            .replace("{*VOTE_RESULT_DESCRIPTION*}", "Bundesgesetz, mit dem das Allgemeine Sozialversicherungsgesetz geändert wird (ASVG-Novelle 2025)")
            .replace("{*VOTE_RESULT_GP*}", "XXVIII")
            .replace("{*VOTE_RESULT_TYPE*}", "Gesetz")
            .replace("{*VOTE_RESULT_OUTCOME*}", "Angenommen")
            .replace(
                "{*VOTE_RESULT_DATE_BADGE*}",
                r#"<span class="dm-badge" style="display: inline-block; background-color: #fae9da; color: #000000; font-family: Arial, sans-serif; font-size: 12px; line-height: 16px; font-weight: 600; border-radius: 6px; padding: 4px 8px; margin: 0 6px 6px 0; white-space: nowrap;">15.03.2026</span>"#,
            )
            .replace(
                "{*VOTE_RESULT_PARTY_VOTES*}",
                r#"ÖVP <span style="color: #22c55e;">✓</span>  SPÖ <span style="color: #22c55e;">✓</span>  FPÖ <span style="color: #ef4444;">✗</span>  Grüne <span style="color: #ef4444;">✗</span>  NEOS <span style="color: #22c55e;">✓</span>"#,
            )
    }

    fn make_proposal_entry() -> String {
        MINISTRIAL_PROP_ENTRY_TEMPLATE
            .replace("{*PROPOSAL_SOMES_URL*}", "https://somes.at/gov_proposal/XXVIII/ME/60")
            .replace("{*PROPOSAL_DELEGATE_IMG_URL*}", DUMMY_DELEGATE_IMG)
            .replace("{*PROPOSAL_GOV_OFFICIAL_NAME*}", "Mag. Johannes Rauch")
            .replace("{*PROPOSAL_DESCRIPTION*}", "Entwurf eines Bundesgesetzes, mit dem das Gesundheits- und Krankenpflegegesetz sowie das Pflegefonds-Gesetz geändert werden")
            .replace("{*PROPOSAL_MINISTRY*}", "Gesundheit")
            .replace("{*PROPOSAL_DATE*}", "15.01.2026")
            .replace("{*PROPOSAL_GP*}", "XXVIII")
    }

    fn make_decree_entry() -> String {
        DECREE_ENTRY_TEMPLATE
            .replace("{*DECREE_SOMES_URL*}", "https://somes.at/decree/RIS-123456")
            .replace("{*DECREE_DELEGATE_IMG_URL*}", DUMMY_DELEGATE_IMG)
            .replace("{*DECREE_GOV_OFFICIAL_NAME*}", "Dr. Karoline Edtstadler")
            .replace("{*DECREE_DESCRIPTION*}", "Verordnung der Bundesministerin für EU und Verfassung über die Durchführung der Volksbegehren 2026")
            .replace("{*DECREE_MINISTRY*}", "Verfassung")
            .replace("{*DECREE_DATE*}", "10.02.2026")
            .replace("{*DECREE_GP*}", "XXVIII")
    }

    fn make_delegate_entry() -> String {
        let vote1 = make_vote_entry();
        let vote2 = VOTE_RESULT_ENTRY_TEMPLATE
            .replace("{*VOTE_RESULT_SOMES_URL*}", "https://somes.at/vote_result/XXVIII/I/42")
            .replace("{*VOTE_RESULT_DESCRIPTION*}", "Dringliche Anfrage betreffend Budgetkonsolidierung und Sparmaßnahmen im öffentlichen Bereich")
            .replace("{*VOTE_RESULT_GP*}", "XXVIII")
            .replace("{*VOTE_RESULT_TYPE*}", "Namentliche Abstimmung")
            .replace("{*VOTE_RESULT_OUTCOME*}", "14.03.2026")
            .replace("{*VOTE_RESULT_DATE_BADGE*}", "")
            .replace("{*VOTE_RESULT_PARTY_VOTES*}", "Dafür");

        let proposals_section = format!(
            r#"<h3 class="dm-section-title" style="font-family: Arial, sans-serif; font-size: 14px; font-weight: 700; letter-spacing: 0.5px; text-transform: uppercase; margin: 20px 0 8px 0; color: #555555;">Eingebrachte Anträge</h3>{vote1}{vote2}"#
        );
        let decrees_section = format!(
            r#"<h3 class="dm-section-title" style="font-family: Arial, sans-serif; font-size: 14px; font-weight: 700; letter-spacing: 0.5px; text-transform: uppercase; margin: 20px 0 8px 0; color: #555555;">Namentliche Abstimmungen</h3>{v}"#,
            v = vote2.clone()
        );

        DELEGATE_ENTRY_TEMPLATE
            .replace("{*DELEGATE_IMG_URL*}", DUMMY_DELEGATE_IMG)
            .replace("{*DELEGATE_NAME*}", "Mag. Herbert Kickl")
            .replace("{*DELEGATE_PARTY*}", "FPÖ")
            .replace("{*DECREES*}", "")
            .replace("{*GOV_PROPOSALS*}", "")
            .replace("{*ISSUED_PROPOSALS*}", &proposals_section)
            .replace("{*NAMED_VOTES*}", &decrees_section)
            .replace("{*SPEECHES*}", "")
            .replace("{*CALL_TO_ORDERS*}", "")
    }

    #[test]
    fn test_send_template_preview() {
        let mail_to = TEST_MAIL_RECEIVER;

        // 1. Delegate activity email
        let delegate_html = make_delegate_entry();
        let delegate_mail = DELEGATE_ACTIVITY_TEMPLATE.replace("{*DELEGATES*}", &delegate_html);
        send_mail(&MAILER, mail_to, "[Test] Neue Abgeordnetenaktivität auf somes", delegate_mail)
            .expect("could not send delegate activity preview");

        // 2. Vote result / legis_init email
        let votes_html = format!("{}{}", make_vote_entry(), make_vote_entry());
        let legis_mail = LEGIS_INIT_EMAIL_TEMPLATE
            .replace("{*TITLE*}", "Neue Abstimmungsergebnisse")
            .replace("{*DESCRIPTION*}", "Der Nationalrat hat kürzlich verschiedene Abstimmungen gehalten, die zu Ihrer Interessenswahl passen.")
            .replace("{*VOTE_RESULTS*}", &votes_html);
        send_mail(&MAILER, mail_to, "[Test] Neue Abstimmungsergebnisse auf somes", legis_mail)
            .expect("could not send legis init preview");

        // 3. Ministerial proposal email
        let props_html = format!("{}{}", make_proposal_entry(), make_proposal_entry());
        let prop_mail = MINISTRIAL_PROP_EMAIL_TEMPLATE.replace("{*MINISTRIAL_PROPS*}", &props_html);
        send_mail(&MAILER, mail_to, "[Test] Neue Ministerialentwürfe auf somes", prop_mail)
            .expect("could not send ministerial prop preview");

        // 4. Decree email
        let decrees_html = format!("{}{}", make_decree_entry(), make_decree_entry());
        let decree_mail = DECREE_EMAIL_TEMPLATE.replace("{*DECREES*}", &decrees_html);
        send_mail(&MAILER, mail_to, "[Test] Neue Verordnungen auf somes", decree_mail)
            .expect("could not send decree preview");

        println!("All 4 test emails sent to {mail_to}");
    }
}
