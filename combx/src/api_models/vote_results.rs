mod hidden_vote_translation;
mod named_votes;

use serde::{Deserialize, Serialize};
use somes_scraper::{LegislativeInitiative, Opinion, Reference, RelatedDelegate};
use sqlx::{FromRow, PgPool};

use crate::{
    DbNamedVoteInfo,
    api_models::vote_results::{
        hidden_vote_translation::{gen_code_to_parties_with_date, translate_votes},
        named_votes::create_db_named_votes,
    },
    models::{
        DbLegisDocumentOptional, DbLegislativeInitiativeQuery, DbNamedVotes, DbSpeechWithLink,
        DbVote, Topic,
    },
};

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Deserialize, Serialize, Clone, FromRow, Default)]
pub struct MeiliesearchHelper {
    #[serde(default)]
    pub votes: Vec<String>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(Debug, Deserialize, Serialize, Clone, FromRow, Default)]
pub struct VoteResult {
    pub id: Option<i32>,
    pub legislative_initiative: DbLegislativeInitiativeQuery,
    pub votes: Vec<DbVote>,
    pub speeches: Vec<DbSpeechWithLink>,
    pub named_votes: Option<DbNamedVotes>,
    pub topics: Vec<Topic>,
    pub eurovoc_topics: Vec<Topic>,
    pub other_keyword_topics: Vec<Topic>,
    pub documents: Vec<DbLegisDocumentOptional>,
    pub absences: Option<Vec<i32>>,
    pub issued_by_dels: Vec<RelatedDelegate>,
    pub referenced_by_others_ids: Option<Vec<i32>>,
    pub references: Option<Vec<Reference>>,
    #[serde(default)]
    pub meilisearch_helper: MeiliesearchHelper,
}

impl VoteResult {
    // move to other crate
    pub async fn from_db(pool: &PgPool, legis_init_id: i32) -> Self {
        todo!()
    }
    pub async fn from_legis_init(pool: &PgPool, legis_init: LegislativeInitiative) -> Self {
        let plenary_session_id = retrieve_plenary_session_id(pool, &legis_init).await;

        let db_legis_init = DbLegislativeInitiativeQuery {
            id: 0,
            title: legis_init.title.clone(),
            inr: legis_init.inr as i32,
            emphasis: legis_init.emphasis.clone(),
            description: legis_init.description.clone(),
            ityp: legis_init.ityp.clone(),
            gp: legis_init.gp.clone(),
            accepted: legis_init.accepted.map(|a| a.to_string()),
            created_at: legis_init.date,
            appeared_at: Some(legis_init.created_at),
            updated_at: legis_init.updated_at,
            requires_simple_majority: legis_init.requires_simple_majority,
            pre_declined_type: legis_init
                .hidden_votes_info
                .vote_stage
                .map(|pre| pre.to_string()),
            voted_by_name: legis_init.hidden_votes_info.voted_by_name,
            plenary_session_id,
            doktyp: legis_init.doktyp.clone(),
            is_law: legis_init.is_law.clone(),
            law_accepted: legis_init.law_accepted.clone(),
            law_come_into_effect_date: legis_init
                .come_into_effect_info
                .as_ref()
                .map(|x| x.law_come_into_effect_date.clone())
                .flatten(),
            law_expires_on_date: legis_init
                .come_into_effect_info
                .as_ref()
                .map(|x| x.law_expires_on_date.clone())
                .flatten(),
            by_publication: legis_init
                .come_into_effect_info
                .as_ref()
                .map(|x| x.by_publication),
            has_reference: Some(
                legis_init
                    .references
                    .as_ref()
                    .map(|x| !x.is_empty())
                    .unwrap_or_default(),
            ),
            is_voteable_on: Some(legis_init.is_voteable_on),
            is_urgent: legis_init.is_urgent,
            is_emphasis_ai_generated: false,
            // law_come_into_effect_date: legis_init.come_into_effect_info.map(|x| x.law_come_into_effect_date.clone()),
        };

        let mut votes = legis_init
            .votes
            .iter()
            .map(|vote| DbVote {
                party: vote.party.name.clone(),
                code: vote.party.code.clone(),
                fraction: vote.party.fraction as i32,
                infavor: vote.infavor,
            })
            .collect::<Vec<_>>();

        let code_to_parties =
            gen_code_to_parties_with_date(pool, &db_legis_init.created_at, &db_legis_init.gp).await;

        if legis_init.votes.is_empty()
            // && legis_init.accepted == Some(somes_scraper::AcceptedState::PreDeclined)
            && legis_init.hidden_votes_info.pro_contra_parties.is_some()
        {
            match &legis_init.hidden_votes_info.pro_contra_parties {
                Some(pro_contra_parties) => {
                    votes =
                        translate_votes(&code_to_parties, pro_contra_parties, &legis_init.gp).await;
                }
                None => log::warn!(
                    "Pro Contra is empty despite hidden vote info for legis init: {}, {}, {}",
                    legis_init.gp,
                    legis_init.ityp,
                    legis_init.inr
                ),
            }
        }

        let named_votes = match &legis_init.hidden_votes_info.voted_by_name_data {
            Some(voted_by_name_data) => {
                let named_vote_info = DbNamedVoteInfo {
                    pro_count: voted_by_name_data.pro_count as i32,
                    contra_count: voted_by_name_data.contra_count as i32,
                    given_vote_sum: voted_by_name_data.vote_sum as i32,
                    invalid_count: voted_by_name_data.invalid_count as i32,
                };

                let named_votes = match &voted_by_name_data.delegates_pro_contra {
                    Some(pro_contra) => {
                        create_db_named_votes(pool, pro_contra, legis_init.date, &legis_init.gp)
                            .await
                            .ok()
                    }
                    None => None,
                };
                Some(DbNamedVotes {
                    named_vote_info,
                    named_votes: named_votes.unwrap_or_default(),
                })
            }
            _ => None,
        };

        let topics = legis_init
            .topics
            .into_iter()
            .map(|topic| Topic { topic })
            .collect::<Vec<_>>();
        let eurovoc_topics = legis_init
            .eurovoc_topics
            .into_iter()
            .map(|topic| Topic { topic })
            .collect::<Vec<_>>();
        let other_keyword_topics = legis_init
            .other_keyword_topics
            .into_iter()
            .map(|topic| Topic { topic })
            .collect::<Vec<_>>();
        let documents = legis_init
            .documents
            .into_iter()
            .map(|document| DbLegisDocumentOptional {
                title: Some(document.title),
                document_type: Some(document.ftype.to_string()),
                document_url: Some(document.link),
            })
            .collect::<Vec<_>>();

        let mut speeches = legis_init
            .speeches
            .into_iter()
            .map(|speech| {
                let opinion = match &speech.opinion {
                    Opinion::Infavor => Some("Pro".to_string()),
                    Opinion::Against => Some("Contra".to_string()),
                    Opinion::Other(text) => Some(text.clone()),
                    Opinion::None => None,
                };

                DbSpeechWithLink {
                    delegate_id: speech.delegate_id as i32,
                    infavor: match speech.opinion {
                        Opinion::Infavor => Some(true),
                        Opinion::Against => Some(false),
                        _ => None,
                    },
                    duration_in_seconds: None,
                    opinion,
                    document_url: speech
                        .spoken_text_html_urls
                        .first()
                        .map(|html_url| html_url.document_url.clone()),
                    about: None,
                }
            })
            .collect::<Vec<_>>();

        // IMPORTANT: others must not rely on this id (anymore)! (e.g. Meilisearch ID/primary key)
        let maybe_legis_init_id = sqlx::query!(
            "select id from legislative_initiatives where inr = $1 and ityp = $2 and gp = $3",
            db_legis_init.inr,
            db_legis_init.ityp,
            db_legis_init.gp
        )
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|x| x.id);

        let (absences, referenced_by_others_ids) = if let Some(maybe_legis_init_id) =
            maybe_legis_init_id
        {
            if let Ok(plenar_speeches) = sqlx::query_as!(
                DbSpeechWithLink,
                "
                    SELECT
                        delegate_id,
                        CASE
                            WHEN opinion = 'Pro' THEN ($2 NOT LIKE '%p%')
                            WHEN opinion = 'Contra' THEN ($2 LIKE '%p%')
                            ELSE NULL
                        END AS infavor,
                        duration_in_seconds,
                        opinion,
                        document_url,
                        about
                    FROM plenar_speeches
                    INNER JOIN plenar_speech_links psl ON plenar_speeches.id = psl.plenar_speech_id
                    INNER JOIN plenar_speech_legis_inits pl ON pl.speech_id = plenar_speeches.id
                    INNER JOIN debates ON debates.id = plenar_speeches.debate_id
                    WHERE legis_init_id = $1
                    ",
                maybe_legis_init_id,
                db_legis_init.pre_declined_type
            )
            .fetch_all(pool)
            .await
            {
                speeches = plenar_speeches
            }

            let absences = sqlx::query!(
                "
                   SELECT delegate_id FROM absences
        WHERE plenary_session_id = (
            SELECT plenary_session_id
            FROM legislative_initiatives
            WHERE id = $1
        ) 
            ",
                maybe_legis_init_id
            )
            .fetch_all(pool)
            .await
            .ok()
            .map(|x| x.into_iter().map(|x| x.delegate_id).collect::<Vec<_>>());

            let referenced_by_other_ids = sqlx::query!("select origin_legis_init_id from legis_inits_refs where ref_gp = $1 and ref_ityp = $2 and ref_inr = $3", legis_init.gp, legis_init.ityp, db_legis_init.inr).fetch_all(pool).await.ok()
                .map(|x| x.into_iter().map(|x| x.origin_legis_init_id).collect::<Vec<_>>());

            (absences, referenced_by_other_ids)
        } else {
            (None, None)
        };

        VoteResult {
            id: maybe_legis_init_id,
            legislative_initiative: db_legis_init,
            votes,
            speeches,
            named_votes,
            topics,
            eurovoc_topics,
            other_keyword_topics,
            documents,
            absences,
            issued_by_dels: legis_init.related_delegates,
            referenced_by_others_ids,
            references: legis_init.references,
            meilisearch_helper: MeiliesearchHelper::default(),
        }
    }
}

async fn retrieve_plenary_session_id(
    pool: &sqlx::Pool<sqlx::Postgres>,
    legis_init: &LegislativeInitiative,
) -> Option<i32> {
    match legis_init.plenary_session {
        Some(session) => sqlx::query!(
            "select id from plenar_infos where inr = $1 and legislative_period = $2",
            session as i32,
            legis_init.gp
        )
        .fetch_one(pool)
        .await
        .ok()
        .map(|id| id.id),
        None => None,
    }
}
