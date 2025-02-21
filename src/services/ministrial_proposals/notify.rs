use std::collections::HashMap;

use somes_scraper::ministrial_proposals::MinistrialProposal;
use sqlx::{query, PgPool};

use crate::email::{send_mail, MAILER};


pub async fn find_notifiers_ministrial_props<'a>(
    pool: &PgPool,
    legis_inits: &'a [(i32, MinistrialProposal)],
) -> HashMap<String, Vec<&'a (i32, MinistrialProposal)>> {
    let ids = query!("select id, email from somes_user where is_email_hashed is false and send_new_vote_results_mails")
        .fetch_all(pool)
        .await
        .unwrap_or_default();
    let mut to_notify_emails = HashMap::new();
    for id in ids {
        let user_topics = query!("select topic_name from user_topics inner join unique_topics ut on ut.id = topic_id where user_id = $1", id.id).fetch_all(pool).await.unwrap_or_default().into_iter().map(|x| x.topic_name).collect::<Vec<_>>();

        let relevant_legis_inits = legis_inits
            .iter()
            .filter(|legis_init| {
                for topic in &legis_init.1.eurovoc_topics {
                    if user_topics.contains(topic) {
                        return true;
                    }
                }
                false
            })
            .collect::<Vec<_>>();

        if !relevant_legis_inits.is_empty() {
            to_notify_emails.insert(id.email, relevant_legis_inits);
        }
    }
    to_notify_emails
}

pub fn send_new_ministrial_prop_info_mail(
    mail_to: &str,
    ministrial_props: &[&(i32, MinistrialProposal)],
) -> Result<(), Box<dyn std::error::Error>> {
    let ministrial_props = ministrial_props.iter().map(|x| (*x).clone()).collect::<Vec<_>>();

    let mail_to = mail_to.to_string();
    std::thread::spawn(move || {
        let mut ministrial_props_html = String::new();
        for (idx, (id, ministrial_prop)) in ministrial_props.iter().enumerate() {
            if idx >= 5 {
                if ministrial_props.len() >= 6 {
                    ministrial_props_html.push_str(&"Und weitere ...");
                }
                break;
            }

            let ministrial_prop_html_entry = format!(
                r#"
                <a href="https://somes.at/alpha/vote_result?id={id}">
                    <div style="display: flex; flex-direction: row; justify-content: space-between; width: 100%;">
                        <div style="border: 1px solid black; width: 50%; display: flex; justify-content: center; align-items: center;">
                            {desc}
                        </div>
                        <div style="border: 1px solid black; width: 60%;">
                            <img style="width: 70%;" src="data:image/png;base64"/>
                        </div>
                    </div>
                </a>
            "#,
                desc = ministrial_prop.description
            );

            ministrial_props_html.push_str(&ministrial_prop_html_entry);
        }

        log::info!("Sending composed new ministrial prop mail to {mail_to}");
        let content = crate::email::MINISTRIAL_PROP_EMAIL_TEMPLATE.replace("{*MINISTRIAL_PROPS*}", &ministrial_props_html);
        if let Err(e) = send_mail(&MAILER, &mail_to, "Neue Ministerialentwürfe auf somes", content) {
            log::error!("could not send ministrial prop info mail: {e:?}");
        }
    });

    Ok(())
}

pub fn notify_users(
    user_ministrial_props: &HashMap<String, Vec<&(i32, MinistrialProposal)>>,
) {
    for (mail_to, ministrial_props) in user_ministrial_props {
        if let Err(e) = send_new_ministrial_prop_info_mail(mail_to, ministrial_props) {
            log::error!("could not send legis info mail: {e:?}");
        }
    }
    // for notifier_mail in find_notifiers(pool, legis_init_topics).await {}
}
