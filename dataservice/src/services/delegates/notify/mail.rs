use std::collections::HashMap;

use crate::email::{send_mail, DELEGATE_ACTIVITY_TEMPLATE, MAILER};

pub(super) fn send_delegate_activity_mail(
    mail_to: &str,
    html_delegate_activities: &HashMap<i32, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if html_delegate_activities.is_empty() {
        return Ok(());
    }

    let mut delegate_entries = html_delegate_activities
        .iter()
        .collect::<Vec<(&i32, &String)>>();
    delegate_entries.sort_by_key(|(delegate_id, _)| **delegate_id);

    let delegates_html = delegate_entries
        .into_iter()
        .map(|(_, html)| html.as_str())
        .collect::<Vec<_>>()
        .join("");

    let content = DELEGATE_ACTIVITY_TEMPLATE.replace("{*DELEGATES*}", &delegates_html);
    log::info!("Sending composed delegate activity mail to {mail_to}");

    send_mail(
        &MAILER,
        mail_to,
        "Neue Abgeordnetenaktivität auf somes",
        content,
    )
}
