use std::collections::HashMap;
use combx::GovProposal;
use sqlx::{query, PgPool};

use crate::{
    email::{send_mail, MAILER},
    mailer::SimpleGovOfficial,
};

pub async fn find_notifiers_ministrial_props<'a>(
    pool: &PgPool,
    gov_props: &'a [(i32, GovProposal)],
) -> HashMap<String, Vec<&'a (i32, GovProposal)>> {
    let ids = query!("select id, email from somes_user where is_email_hashed is false and (send_new_ministrial_prop_mails or send_new_ministrial_prop_by_favo_mails)")
        .fetch_all(pool)
        .await
        .unwrap_or_default();
    let mut to_notify_emails = HashMap::new();
    for id in ids {
        let user_topics = query!("select topic_name from user_topics inner join unique_eurovoc_topics ut on ut.id = topic_id where user_id = $1", id.id).fetch_all(pool).await.unwrap_or_default().into_iter().map(|x| x.topic_name).collect::<Vec<_>>();
        let favo_dels = query!(
            "select delegate_id from favo_dels where user_id = $1",
            id.id
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        let relevant_gov_props = gov_props
            .iter()
            .filter(|(_id, gov_prop)| {
                for topic in &gov_prop.eurovoc_topics {
                    if user_topics.contains(&topic.topic) {
                        return true;
                    }
                }
                favo_dels
                    .iter()
                    .find(|favo_del| {
                        // TODO: prevent that the same user gets multiple mails for the same ministrial prop if he has multiple favorite delegates
                        gov_prop.ministerial_issuers.contains(&favo_del.delegate_id)
                    })
                    .is_some()
            })
            .collect::<Vec<_>>();

        if !relevant_gov_props.is_empty() {
            to_notify_emails.insert(id.email, relevant_gov_props);
        }
    }
    to_notify_emails
}

pub(crate) fn send_new_ministrial_prop_info_mail(
    mail_to: &str,
    ministrial_props: &[&(i32, GovProposal)],
    gov_officials: &HashMap<i32, SimpleGovOfficial>,
) -> Result<(), Box<dyn std::error::Error>> {
    let ministrial_props = ministrial_props
        .iter()
        .map(|x| (*x).clone())
        .collect::<Vec<_>>();

    let mail_to = mail_to.to_string();
    let gov_officials = gov_officials.clone();
    std::thread::spawn(move || {
        let mut ministrial_props_html = String::new();
        for (idx, (_id, ministrial_prop)) in ministrial_props.iter().enumerate() {
            if idx >= 5 {
                if ministrial_props.len() >= 6 {
                    ministrial_props_html.push_str(&"Und weitere ...");
                }
                break;
            }

            // TODO: display multiple gov officials if there are multiple ministerial issuers / this would also prevent multiple mails to the same user if he has multiple favorite delegates
            // https://github.com/som-es/somes/issues/113
            let Some(gov_official) = gov_officials.get(&ministrial_prop.ministerial_issuers[0])
            else {
                continue;
            };
            let ministrial_prop_html_entry = crate::email::MINISTRIAL_PROP_ENTRY_TEMPLATE
                .replace(
                    "{*PROPOSAL_DESCRIPTION*}",
                    &ministrial_prop.ministrial_proposal.description,
                )
                .replace(
                    "{*PROPOSAL_MINISTRY*}",
                    &ministrial_prop.ministrial_proposal.ressort_shortform,
                )
                .replace(
                    "{*PROPOSAL_DATE*}",
                    &ministrial_prop
                        .ministrial_proposal
                        .created_at
                        .date()
                        .format("%d.%m.%Y")
                        .to_string(),
                )
                .replace("{*PROPOSAL_GP*}", &ministrial_prop.ministrial_proposal.gp)
                .replace("{*PROPOSAL_GOV_OFFICIAL_NAME*}", &gov_official.name)
                .replace(
                    "{*PROPOSAL_DELEGATE_IMG_URL*}",
                    &gov_official
                        .image_url
                        .as_ref()
                        .unwrap_or(&"https://somes.at/images/somes_logo.png".to_string()),
                )
                .replace(
                    "{*PROPOSAL_SOMES_URL*}",
                    &format!(
                        "https://somes.at/alpha/gov_proposal/{gp}/ME/{inr}",
                        gp = ministrial_prop.ministrial_proposal.gp,
                        inr = ministrial_prop.ministrial_proposal.inr
                    ),
                );

            ministrial_props_html.push_str(&ministrial_prop_html_entry);
        }

        log::info!("Sending composed new ministrial prop mail to {mail_to}");
        let content = crate::email::MINISTRIAL_PROP_EMAIL_TEMPLATE
            .replace("{*MINISTRIAL_PROPS*}", &ministrial_props_html);
        println!("content: {content}");
        if let Err(e) = send_mail(
            &MAILER,
            &mail_to,
            "Neue Ministerialentwürfe auf somes",
            content,
        ) {
            log::error!("could not send ministrial prop info mail: {e:?}");
        }
    });

    Ok(())
}

pub(crate) fn notify_users(
    user_ministrial_props: &HashMap<String, Vec<&(i32, GovProposal)>>,
    gov_officials: &HashMap<i32, SimpleGovOfficial>,
) {
    for (mail_to, ministrial_props) in user_ministrial_props {
        if let Err(e) = send_new_ministrial_prop_info_mail(mail_to, ministrial_props, gov_officials)
        {
            log::error!("could not send ministrial prop info mail: {e:?}");
        }
    }
}
