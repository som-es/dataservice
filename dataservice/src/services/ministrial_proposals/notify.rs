use chrono_tz::Europe::Vienna;
use combx::GovProposal;
use sqlx::{PgPool, query};
use std::collections::HashMap;

use crate::email::{MAILER, send_mail};

#[derive(Debug, Clone)]
pub(crate) struct SimpleGovOfficial {
    pub id: i32,
    pub name: String,
    pub image_url: Option<String>,
    pub current_party: Option<String>,
    pub party: Option<String>,
    pub primary_mandate: String,
    pub active_mandates: Option<Vec<String>>,
}

pub(crate) async fn get_active_gov_officals(
    pg_pool: &PgPool,
) -> sqlx::Result<Vec<SimpleGovOfficial>> {
    sqlx::query_as!(
        SimpleGovOfficial,
        "
       SELECT DISTINCT ON (delegates.id)
               delegates.id,
               delegates.name,
               mandates.party,
               delegates.party AS current_party,
               delegates.image_url,
               mandates.name as primary_mandate,
               COALESCE(mandate_groups.mandate_array, '{}') AS active_mandates
           FROM
               mandates
           INNER JOIN
               delegates ON delegates.id = mandates.delegate_id
           LEFT JOIN
               (SELECT
                   delegate_id,
                   ARRAY_AGG(name) AS mandate_array
               FROM
                   mandates
               WHERE
                   is_gov_official
                   AND end_date IS NULL
               GROUP BY
                   delegate_id) AS mandate_groups
               ON delegates.id = mandate_groups.delegate_id
           WHERE
               mandates.is_gov_official
       "
    )
    .fetch_all(pg_pool)
    .await
}

pub async fn find_notifiers_ministrial_props<'a>(
    pool: &PgPool,
    gov_props: &'a [(i32, GovProposal)],
) -> HashMap<String, Vec<&'a (i32, GovProposal)>> {
    let ids = query!("select id, email, send_new_ministrial_prop_mails, send_new_ministrial_prop_by_favo_mails from somes_user where is_email_hashed is false and (send_new_ministrial_prop_mails or send_new_ministrial_prop_by_favo_mails)")
        .fetch_all(pool)
        .await
        .unwrap_or_default();
    let mut to_notify_emails = HashMap::new();
    for id in ids {
        let user_topics = query!("select topic_name from user_topics inner join unique_eurovoc_topics ut on ut.id_as_hash = topic_id where user_id = $1", id.id).fetch_all(pool).await.unwrap_or_default().into_iter().map(|x| x.topic_name).collect::<Vec<_>>();
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
                if id.send_new_ministrial_prop_mails {
                    let topics = if gov_prop.eurovoc_topics.is_empty() {
                        gov_prop
                            .ai_summary
                            .as_ref()
                            .map(|summary| summary.full_summary.topics.clone())
                            .unwrap_or_default()
                    } else {
                        gov_prop
                            .eurovoc_topics
                            .iter()
                            .map(|x| x.topic.clone())
                            .collect::<Vec<_>>()
                    };
                    for topic in &topics {
                        if user_topics.contains(topic) {
                            return true;
                        }
                    }
                }

                if id.send_new_ministrial_prop_by_favo_mails {
                    favo_dels
                        .iter()
                        .find(|favo_del| {
                            // TODO: prevent that the same user gets multiple mails for the same ministrial prop if he has multiple favorite delegates
                            gov_prop.ministerial_issuers.contains(&favo_del.delegate_id)
                        })
                        .is_some()
                } else {
                    false
                }
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
                    ministrial_props_html.push_str("Und weitere ...");
                }
                break;
            }

            // TODO: display multiple gov officials if there are multiple ministerial issuers / this would also prevent multiple mails to the same user if he has multiple favorite delegates
            // https://github.com/som-es/somes/issues/113
            let Some(gov_official) = gov_officials.get(&ministrial_prop.ministerial_issuers[0])
            else {
                continue;
            };
            let description = ministrial_prop
                .ai_summary
                .as_ref()
                .map(|summary| &summary.short_title)
                .unwrap_or(&ministrial_prop.ministrial_proposal.description);

            // let proposal_date = Vienna
            use chrono::TimeZone;
            let proposal_date = Vienna.from_utc_datetime(
                &ministrial_prop
                    .ministrial_proposal
                    .raw_data_created_at
                    .naive_utc(),
            );

            let ministrial_prop_html_entry = crate::email::MINISTRIAL_PROP_ENTRY_TEMPLATE
                .replace("{*PROPOSAL_DESCRIPTION*}", description)
                .replace(
                    "{*PROPOSAL_MINISTRY*}",
                    &ministrial_prop.ministrial_proposal.ressort_shortform,
                )
                .replace(
                    "{*PROPOSAL_DATE*}",
                    &proposal_date.date_naive().format("%d.%m.%Y").to_string(),
                )
                .replace("{*PROPOSAL_GP*}", &ministrial_prop.ministrial_proposal.gp)
                .replace("{*PROPOSAL_GOV_OFFICIAL_NAME*}", &gov_official.name)
                .replace(
                    "{*PROPOSAL_DELEGATE_IMG_URL*}",
                    gov_official
                        .image_url
                        .as_ref()
                        .unwrap_or(&"https://somes.at/images/somes_logo.png".to_string()),
                )
                .replace(
                    "{*PROPOSAL_SOMES_URL*}",
                    &format!(
                        "https://somes.at/gov_proposal/{gp}/ME/{inr}",
                        gp = ministrial_prop.ministrial_proposal.gp,
                        inr = ministrial_prop.ministrial_proposal.inr
                    ),
                );

            ministrial_props_html.push_str(&ministrial_prop_html_entry);
        }

        log::info!("Sending composed new ministrial prop mail to {mail_to}");
        let content = crate::email::MINISTRIAL_PROP_EMAIL_TEMPLATE
            .replace("{*MINISTRIAL_PROPS*}", &ministrial_props_html);
        // println!("content: {content}");
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

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use combx::{GovProposal, create_gov_officials_map};
    use somes_scraper::{
        AustriaScraper,
        ministrial_proposals::{GegenstandInfo, MinistrialProposalScraper},
    };

    use crate::{
        connect_pg2,
        ministrial_proposals::{
            notify::get_active_gov_officals, send_new_ministrial_prop_info_mail,
        },
        today,
    };

    #[tokio::test]
    async fn test_send_new_ministerial_prop_mail() {
        let pg_pool = connect_pg2().await;
        let simple_gov_officials = get_active_gov_officals(&pg_pool).await;

        let simple_gov_officials = simple_gov_officials
            .unwrap_or_default()
            .into_iter()
            .map(|gov| (gov.id, gov))
            .collect::<HashMap<_, _>>();

        let scraper = AustriaScraper::default();
        let ministerial_proposal = scraper.get_ministrial_proposal(GegenstandInfo {
            path: "/gegenstand/XXVIII/ME/60".into(),
            ressort:
                "Bundesministerium für Arbeit, Soziales, Gesundheit, Pflege und Konsumentenschutz"
                    .into(),
            ressort_shortform: "kp".into(),
            due_to: today().date_naive(),
        }).await.unwrap();
        let gov_officials = Arc::new(create_gov_officials_map().await);
        let gov_proposal =
            GovProposal::from_ministerial_proposal(ministerial_proposal, gov_officials);
        println!("gov proposal: {gov_proposal:?}");
        let gov_proposals = [&(0, gov_proposal)];
        send_new_ministrial_prop_info_mail(
            crate::email::TEST_MAIL_RECEIVER,
            &gov_proposals,
            &simple_gov_officials,
        )
        .unwrap();

        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}
