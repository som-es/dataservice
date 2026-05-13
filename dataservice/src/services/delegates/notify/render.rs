use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Europe::Vienna;

use super::activity::RecentActivity;
use crate::email::{
    DECREE_ENTRY_TEMPLATE, DELEGATE_ENTRY_TEMPLATE, MINISTRIAL_PROP_ENTRY_TEMPLATE,
    VOTE_RESULT_ENTRY_TEMPLATE,
};

const FALLBACK_DELEGATE_IMG_URL: &str = "https://somes.at/images/somes_logo.png";

fn escape_html(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#39;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

fn format_datetime_date(date: &DateTime<Utc>) -> String {
    Vienna
        .from_utc_datetime(&date.naive_utc())
        .date_naive()
        .format("%d.%m.%Y")
        .to_string()
}

fn render_section(title: &str, content: String) -> String {
    if content.is_empty() {
        return String::new();
    }

    format!(
        r#"<h3 style="font-size: 1rem; margin: 18px 0 8px 0; color: #333;">{}</h3>{}"#,
        escape_html(title),
        content
    )
}

fn render_text_entry(title: &str, description: &str, url: Option<&str>) -> String {
    let title = escape_html(title);
    let description = escape_html(description);
    let link = url.map(|url| {
        format!(
            r#"<a href="{}" style="color: #d8806f; text-decoration: none;">Auf somes ansehen</a>"#,
            escape_html(url)
        )
    });

    format!(
        r#"<div style="border: 1px solid #eee; border-radius: 8px; padding: 12px; margin: 8px 0; background: #fff;">
            <div style="font-weight: 700; margin-bottom: 6px;">{title}</div>
            <div style="color: #555; line-height: 1.35;">{description}</div>
            {}
        </div>"#,
        link.unwrap_or_default()
    )
}

fn render_gov_proposals(activity: &RecentActivity) -> String {
    let mut html = String::new();

    for gov_proposal in activity.gov_poposals.iter().take(5) {
        let Some(proposal) = &gov_proposal.ministrial_proposal else {
            continue;
        };

        let description = gov_proposal
            .ai_summary
            .as_ref()
            .map(|summary| summary.short_title.as_str())
            .unwrap_or(&proposal.description);

        let entry = MINISTRIAL_PROP_ENTRY_TEMPLATE
            .replace("{*PROPOSAL_DESCRIPTION*}", &escape_html(description))
            .replace(
                "{*PROPOSAL_MINISTRY*}",
                &escape_html(&proposal.ressort_shortform),
            )
            .replace(
                "{*PROPOSAL_DATE*}",
                &format_datetime_date(&proposal.raw_data_created_at),
            )
            .replace("{*PROPOSAL_GP*}", &escape_html(&proposal.gp))
            .replace(
                "{*PROPOSAL_GOV_OFFICIAL_NAME*}",
                &escape_html(&activity.delegate_name),
            )
            .replace(
                "{*PROPOSAL_DELEGATE_IMG_URL*}",
                activity
                    .delegate_image_url
                    .as_deref()
                    .unwrap_or(FALLBACK_DELEGATE_IMG_URL),
            )
            .replace(
                "{*PROPOSAL_SOMES_URL*}",
                &format!(
                    "https://somes.at/gov_proposal/{}/ME/{}",
                    proposal.gp, proposal.inr
                ),
            );

        html.push_str(&entry);
    }

    if activity.gov_poposals.len() > 5 {
        html.push_str("Und weitere ...");
    }

    render_section("Ministerialentwürfe", html)
}

fn render_decrees(activity: &RecentActivity) -> String {
    let mut html = String::new();

    for decree in activity.decrees.iter().take(5) {
        let Some(ris_id) = decree.ris_id.as_deref() else {
            continue;
        };

        let description = decree
            .ai_summary
            .as_ref()
            .map(|summary| summary.short_title.as_str())
            .or(decree.short_title.as_deref())
            .or(decree.title.as_deref())
            .unwrap_or("Neue Verordnung");

        let entry = DECREE_ENTRY_TEMPLATE
            .replace("{*DECREE_DESCRIPTION*}", &escape_html(description))
            .replace(
                "{*DECREE_MINISTRY*}",
                &escape_html(decree.ministrial_issuer.as_deref().unwrap_or("")),
            )
            .replace(
                "{*DECREE_DATE*}",
                &decree
                    .publication_date
                    .map(|date| date.format("%d.%m.%Y").to_string())
                    .unwrap_or_default(),
            )
            .replace(
                "{*DECREE_GP*}",
                &escape_html(decree.gp.as_deref().unwrap_or("")),
            )
            .replace(
                "{*DECREE_GOV_OFFICIAL_NAME*}",
                &escape_html(&activity.delegate_name),
            )
            .replace(
                "{*DECREE_DELEGATE_IMG_URL*}",
                activity
                    .delegate_image_url
                    .as_deref()
                    .unwrap_or(FALLBACK_DELEGATE_IMG_URL),
            )
            .replace(
                "{*DECREE_SOMES_URL*}",
                &format!("https://somes.at/decree/{ris_id}"),
            );

        html.push_str(&entry);
    }

    if activity.decrees.len() > 5 {
        html.push_str("Und weitere ...");
    }

    render_section("Verordnungen", html)
}

fn render_issued_proposals(activity: &RecentActivity) -> String {
    let mut html = String::new();

    for vote_result in activity.issued_proposals.iter().take(5) {
        let Some(legis_init) = &vote_result.legislative_initiative else {
            continue;
        };

        let vote_outcome = match legis_init.accepted.as_deref() {
            Some("a") => "Angenommen",
            Some("d") | Some("p") => "Abgelehnt",
            _ => "In Bearbeitung",
        };

        let vote_type = match legis_init.voting.as_deref().unwrap_or("_") {
            "Law" => "Gesetz",
            "Report" => "Bericht",
            "Amendment" => "Abänderung",
            "Resolution" => "Entschließung",
            _ => &legis_init.ityp,
        };

        let description = vote_result
            .ai_summary
            .as_ref()
            .map(|summary| summary.short_title.as_str())
            .unwrap_or(&legis_init.description);

        let party_votes_html = vote_result
            .votes
            .as_ref()
            .map(|votes| {
                votes
                    .iter()
                    .map(|vote| {
                        let symbol = if vote.infavor {
                            "<span style=\"color: #22c55e;\">✓</span>"
                        } else {
                            "<span style=\"color: #ef4444;\">✗</span>"
                        };
                        format!("{} {}", escape_html(&vote.party), symbol)
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();

        let entry = VOTE_RESULT_ENTRY_TEMPLATE
            .replace("{*VOTE_RESULT_DESCRIPTION*}", &escape_html(description))
            .replace("{*VOTE_RESULT_GP*}", &escape_html(&legis_init.gp))
            .replace("{*VOTE_RESULT_TYPE*}", &escape_html(vote_type))
            .replace("{*VOTE_RESULT_OUTCOME*}", vote_outcome)
            .replace("{*VOTE_RESULT_PARTY_VOTES*}", &party_votes_html)
            .replace(
                "{*VOTE_RESULT_SOMES_URL*}",
                &format!(
                    "https://somes.at/vote_result/{}/{}/{}",
                    legis_init.gp, legis_init.ityp, legis_init.inr
                ),
            );

        html.push_str(&entry);
    }

    if activity.issued_proposals.len() > 5 {
        html.push_str("Und weitere ...");
    }

    render_section("Eingebrachte Anträge", html)
}

fn render_named_votes(activity: &RecentActivity) -> String {
    let mut html = String::new();

    for named_vote in activity.named_votes.iter().take(5) {
        let vote = if named_vote.was_absent.unwrap_or(false) {
            "Abwesend"
        } else {
            match named_vote.infavor {
                Some(true) => "Dafür",
                Some(false) => "Dagegen",
                None => "Keine Stimme",
            }
        };

        let entry = VOTE_RESULT_ENTRY_TEMPLATE
            .replace(
                "{*VOTE_RESULT_DESCRIPTION*}",
                &escape_html(
                    named_vote
                        .title
                        .as_deref()
                        .unwrap_or("Namentliche Abstimmung"),
                ),
            )
            .replace("{*VOTE_RESULT_GP*}", &escape_html(&named_vote.gp))
            .replace("{*VOTE_RESULT_TYPE*}", "Namentliche Abstimmung")
            .replace(
                "{*VOTE_RESULT_OUTCOME*}",
                &named_vote.vote_date.format("%d.%m.%Y").to_string(),
            )
            .replace("{*VOTE_RESULT_PARTY_VOTES*}", &escape_html(vote))
            .replace(
                "{*VOTE_RESULT_SOMES_URL*}",
                &format!(
                    "https://somes.at/vote_result/{}/{}/{}",
                    named_vote.gp, named_vote.ityp, named_vote.inr
                ),
            );

        html.push_str(&entry);
    }

    if activity.named_votes.len() > 5 {
        html.push_str("Und weitere ...");
    }

    render_section("Namentliche Abstimmungen", html)
}

fn render_speeches(activity: &RecentActivity) -> String {
    let mut html = String::new();

    for speech in activity.speeches.iter().take(5) {
        let opinion = speech.opinion.as_deref().unwrap_or("Rede");
        let duration = speech
            .duration_in_seconds
            .map(|seconds| format!("{seconds} Sekunden"))
            .unwrap_or_else(|| "Dauer unbekannt".to_string());
        let description = format!("{opinion} · {duration}");
        html.push_str(&render_text_entry(
            speech.about.as_deref().unwrap_or("Rede im Plenum"),
            &description,
            speech.document_url.as_deref(),
        ));
    }

    if activity.speeches.len() > 5 {
        html.push_str("Und weitere ...");
    }

    render_section("Reden", html)
}

fn render_call_to_orders(activity: &RecentActivity) -> String {
    let mut html = String::new();

    for call_to_order in activity.call_to_orders.iter().take(5) {
        let title = format!(
            "Ordnungsruf in der {}. Sitzung",
            call_to_order.plenary_session_id
        );
        let description = format!(
            "{} · GP {}",
            call_to_order.date.date_naive().format("%d.%m.%Y"),
            call_to_order.gp
        );
        let url = format!(
            "https://somes.at/plenar/{}/{}",
            call_to_order.gp, call_to_order.inr
        );

        html.push_str(&render_text_entry(&title, &description, Some(&url)));
    }

    if activity.call_to_orders.len() > 5 {
        html.push_str("Und weitere ...");
    }

    render_section("Ordnungsrufe", html)
}

pub(super) async fn populate_delegate_profile_html(activity: &RecentActivity) -> String {
    DELEGATE_ENTRY_TEMPLATE
        .replace(
            "{*DELEGATE_IMG_URL*}",
            activity
                .delegate_image_url
                .as_deref()
                .unwrap_or(FALLBACK_DELEGATE_IMG_URL),
        )
        .replace("{*DELEGATE_NAME*}", &escape_html(&activity.delegate_name))
        .replace(
            "{*DELEGATE_PARTY*}",
            &escape_html(activity.delegate_party.as_deref().unwrap_or("")),
        )
        .replace("{*DECREES*}", &render_decrees(activity))
        .replace("{*GOV_PROPOSALS*}", &render_gov_proposals(activity))
        .replace("{*ISSUED_PROPOSALS*}", &render_issued_proposals(activity))
        .replace("{*NAMED_VOTES*}", &render_named_votes(activity))
        .replace("{*SPEECHES*}", &render_speeches(activity))
        .replace("{*CALL_TO_ORDERS*}", &render_call_to_orders(activity))
}
