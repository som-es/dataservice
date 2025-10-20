use std::collections::HashMap;

use combx::GovProposal;
use sqlx::PgPool;
use tokio::sync::mpsc::Receiver;

use crate::{
    ministrial_proposals::{find_notifiers_ministrial_props, notify_users},
    Shutdown,
};

pub async fn notify_on_new_ministrial_props(
    pg_pool: &PgPool,
    new_ministrial_prop_rx: &mut Receiver<Option<(i32, GovProposal)>>,
) {
    let simple_gov_officials = get_active_gov_officals(pg_pool).await;
    if let Err(ref e) = simple_gov_officials {
        log::error!(
            "Could not gather active gov officials -> this will degrade the mail experience: {e:?}"
        )
    }
    let simple_gov_officials = simple_gov_officials
        .unwrap_or_default()
        .into_iter()
        .map(|gov| (gov.id, gov))
        .collect::<HashMap<_, _>>();

    loop {
        let mut acc_ministrial_props = Vec::new();
        loop {
            let new_ministrial_prop = new_ministrial_prop_rx.recv().await;
            match new_ministrial_prop {
                Some(Some(ministrial_prop)) => {
                    acc_ministrial_props.push(ministrial_prop);
                }

                Some(None) => break,
                None => {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    continue;
                }
            }
        }

        let notify_user_ministrial_props =
            find_notifiers_ministrial_props(pg_pool, &acc_ministrial_props).await;
        log::info!(
            "Constructed notifier hashmap with {} entries",
            notify_user_ministrial_props.len()
        );

        notify_users(&notify_user_ministrial_props, &simple_gov_officials);
    }
}

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

async fn get_active_gov_officals(pg_pool: &PgPool) -> sqlx::Result<Vec<SimpleGovOfficial>> {
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

pub async fn notify_on_new_ministrial_props_service(
    mut shutdown: Shutdown,
    mut new_vote_result_rx: Receiver<Option<(i32, GovProposal)>>,
    pg_pool: PgPool,
) {
    while !shutdown.is_shutdown() {
        tokio::select! {
            _ = shutdown.recv() => {
                println!("shutting down");
                // return ok?
                break;
            }
            _ = async {
                notify_on_new_ministrial_props(&pg_pool, &mut new_vote_result_rx).await
            } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use combx::{create_gov_officials_map, GovProposal};
    use somes_scraper::{
        ministrial_proposals::{GegenstandInfo, MinistrialProposalScraper},
        AustriaScraper,
    };

    use crate::{
        connect_pg, mailer::new_ministrial_prop::get_active_gov_officals,
        ministrial_proposals::send_new_ministrial_prop_info_mail, today,
    };

    #[tokio::test]
    async fn test_send_new_ministerial_prop_mail() {
        let pg_pool = connect_pg().await;
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
            due_to: today(),
        }).await.unwrap();
        let gov_officials = Arc::new(create_gov_officials_map().await);
        let gov_proposal =
            GovProposal::from_ministerial_proposal(ministerial_proposal, gov_officials);
        println!("gov proposal: {gov_proposal:?}");
        let gov_proposals = [&(0, gov_proposal)];
        send_new_ministrial_prop_info_mail("", &gov_proposals, &simple_gov_officials).unwrap();
        loop {}
    }
}
