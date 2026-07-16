use std::collections::HashMap;

use combx::{DbVote, Delegate, VoteResult};
use image::Rgba;
use imageproc::{
    drawing::{draw_filled_circle_mut, draw_filled_rect_mut},
    rect::Rect,
};
use somes_scraper::Party;
use sqlx::{PgPool, query, query_as, query_scalar};

use crate::{
    LATEST_GP,
    email::{LEGIS_INIT_EMAIL_TEMPLATE, MAILER, VOTE_RESULT_ENTRY_TEMPLATE, send_mail},
};

struct ToNotify {
    id: i32,
    email: String,
    by_topics: bool,
    by_person_favo: bool,
}

pub async fn find_notifiers_legis_init<'a>(
    pool: &PgPool,
    legis_inits: &'a [(i32, VoteResult)],
    is_finished: bool,
) -> HashMap<String, Vec<&'a (i32, VoteResult)>> {
    let ids = if is_finished {
        query_as!(ToNotify, "select id, email, send_new_vote_results_mails as by_topics, send_new_vote_result_by_favo_mails as by_person_favo from somes_user where is_email_hashed is false and (send_new_vote_results_mails or send_new_vote_result_by_favo_mails)")
            .fetch_all(pool)
            .await
            .unwrap_or_default()
    } else {
        query_as!(ToNotify, "
            select
                id, email,
                send_new_proposal_mails as by_topics,
                send_new_proposal_by_favo_mails as by_person_favo
            from somes_user where is_email_hashed is false and (send_new_proposal_mails or send_new_proposal_by_favo_mails)")
            .fetch_all(pool)
            .await
            .unwrap_or_default()
    };
    let mut to_notify_emails = HashMap::new();
    for id in ids {
        let user_topics = query!("select topic_name from user_topics inner join unique_eurovoc_topics ut on ut.id = topic_id where user_id = $1", id.id).fetch_all(pool).await.unwrap_or_default().into_iter().map(|x| x.topic_name).collect::<Vec<_>>();

        let favo_dels = query_scalar!(
            "select delegate_id from favo_dels where user_id = $1",
            id.id
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        let relevant_legis_inits = legis_inits
            .iter()
            .filter(|legis_init| {
                if id.by_person_favo {
                    if legis_init
                        .1
                        .issued_by_dels
                        .iter()
                        .any(|issued_by_del| favo_dels.contains(&issued_by_del.delegate_id))
                    {
                        return true;
                    }
                }
                if id.by_topics {
                    for topic in &legis_init.1.eurovoc_topics {
                        if user_topics.contains(&topic.topic) {
                            return true;
                        }
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

pub fn gen_half_circle(n: usize, r: f32, w: u32, h: u32) -> Option<Vec<(f32, f32)>> {
    if n <= 2 {
        return None;
    }

    let n = n - 2;

    // reduce data points when possible by dividing by 2
    let scaled_max_angle = 18000;

    let modulo = scaled_max_angle / n;

    let count_to = scaled_max_angle + modulo + 1;

    // reduce data points when possible by dividing by 2
    let normalize = 100. * count_to as f32 / (scaled_max_angle as f32);

    Some(
        (0..count_to)
            .step_by(modulo)
            .map(|angle_deg| {
                //const x = 2.0 * r * Math.cos(angle_rad) + w / 2;
                //const y = 2.0 * r * Math.sin(angle_rad) + h / 2;

                let angle_rad = -(angle_deg as f32 / normalize) * std::f32::consts::PI / 180.;
                (
                    angle_rad.cos() * r * 2. + (w / 2) as f32,
                    angle_rad.sin() * r * 2. + (h / 2) as f32,
                )
            })
            .collect::<Vec<_>>(),
    )
}

const IMG_WIDTH: u32 = 830;
const IMG_HEIGHT: u32 = 900;
const CIRCLE_RADIUS: i32 = 6;

fn setup_parliament(seats: &[u64]) -> Vec<Vec<(f32, f32)>> {
    // 70 + (useOffset ? idx * (idx == 1 ? 30 : 20) + (idx >= 2 ? 30 : 0) : idx * 19),
    seats
        .iter()
        .enumerate()
        .flat_map(|(idx, seat_count)| {
            let mut r = 70;
            r += idx * if idx == 1 { 30 } else { 20 };

            if idx >= 2 {
                r += 30;
            }

            gen_half_circle(*seat_count as usize, r as f32, IMG_WIDTH, IMG_HEIGHT)
        })
        .collect::<Vec<_>>()
}

fn convert_votes_to_party_color_map(
    votes: &[DbVote],
    colors: &HashMap<String, String>,
) -> HashMap<String, Rgba<u8>> {
    votes
        .iter()
        .map(|x| {
            let grey_color = "#999999".to_string();
            let color = colors.get(&x.party).unwrap_or(&grey_color);
            let mut rgba = Rgba(
                u32::from_str_radix(&format!("{}FF", &color.as_str()[1..]), 16)
                    .unwrap()
                    .to_be_bytes(),
            );
            if x.infavor {
                rgba.0[3] = 255;
            } else {
                rgba.0[3] = 50;
            }

            (x.party.clone(), rgba)
        })
        .collect::<HashMap<String, Rgba<u8>>>()
}

fn enrich_parliament_with_colors(
    parliament: Vec<Vec<(f32, f32)>>,
    votes: &[DbVote],
    delegates: &[Delegate],
    colors: &HashMap<String, String>,
) -> Vec<Vec<(f32, f32, Rgba<u8>)>> {
    let white = Rgba([255u8, 255u8, 255u8, 255u8]);

    let mut parliament = parliament
        .into_iter()
        .map(|x| {
            x.into_iter()
                .map(|(x, y)| (x, y, white))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let votes_to_party_map = convert_votes_to_party_color_map(votes, colors);

    for delegate in delegates {
        match (delegate.seat_row, delegate.seat_col, &delegate.party) {
            (Some(seat_row), Some(seat_col), Some(party)) => {
                let seat_row = seat_row as usize;
                let seat_col = seat_col as usize;
                let color = *votes_to_party_map
                    .get(party)
                    .unwrap_or(&Rgba([125, 125, 125, 255u8]));
                let len = parliament[seat_row - 1].len();
                parliament[seat_row - 1][len - seat_col].2 = color;
            }

            _ => {}
        }
    }

    parliament
}

fn blend_with_background(background: Rgba<u8>, foreground: Rgba<u8>) -> Rgba<u8> {
    let alpha = foreground[3] as f32 / 255.0;
    let inv_alpha = 1.0 - alpha;

    let r = (foreground[0] as f32 * alpha + background[0] as f32 * inv_alpha) as u8;
    let g = (foreground[1] as f32 * alpha + background[1] as f32 * inv_alpha) as u8;
    let b = (foreground[2] as f32 * alpha + background[2] as f32 * inv_alpha) as u8;

    Rgba([r, g, b, 255])
}

pub fn generate_parliament_image_with_delegates(
    votes: &[DbVote],
    delegates: &[Delegate],
    seats: &[u64],
    colors: &HashMap<String, String>,
) -> image::ImageBuffer<Rgba<u8>, Vec<u8>> {
    let parliament = setup_parliament(seats);
    let parliament = enrich_parliament_with_colors(parliament, votes, delegates, colors);

    let mut image: image::ImageBuffer<Rgba<u8>, Vec<u8>> =
        image::RgbaImage::new(IMG_WIDTH, IMG_HEIGHT / 2 + 60);

    let white = Rgba([255u8, 255u8, 255u8, 255u8]);

    draw_filled_rect_mut(
        &mut image,
        Rect::at(0, 0).of_size(IMG_WIDTH, IMG_HEIGHT / 2 + 60),
        white,
    );

    for half_circle in parliament {
        for (x, y, color) in half_circle {
            // println!("x: {x}, y: {y}");
            let blended_color = blend_with_background(white, color);
            draw_filled_circle_mut(
                &mut image,
                (x as i32, y as i32),
                CIRCLE_RADIUS,
                blended_color,
            );
            // draw_filled_circle_mut(&mut image, (IMG_WIDTH as i32 / 2, IMG_HEIGHT as i32 / 2), 10, red);
        }
    }

    image
    // image.
}

pub async fn get_current_delegates() -> Option<Vec<Delegate>> {
    reqwest::get("https://somes.at/api/delegates")
        .await
        .ok()?
        .json()
        .await
        .ok()
}

pub async fn get_current_parties() -> Option<Vec<Party>> {
    reqwest::get("https://somes.at/api/parties")
        .await
        .ok()?
        .json()
        .await
        .ok()
}

pub fn create_color_map(parties: Vec<Party>) -> HashMap<String, String> {
    parties
        .into_iter()
        .map(|party| (party.name, party.color))
        .collect::<HashMap<_, _>>()
}

pub async fn generate_parliament_image(votes: &[DbVote], seats: &[u64]) -> Option<()> {
    let delegates: Vec<Delegate> = get_current_delegates().await?;
    let colors = create_color_map(get_current_parties().await?);

    let image = generate_parliament_image_with_delegates(votes, &delegates, seats, &colors);
    image.save("test_parliament.png").unwrap();

    Some(())
}

pub async fn get_current_seats() -> Option<Vec<u64>> {
    let seats: serde_json::Value = reqwest::get("https://somes.at/api/seats")
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let seats = seats[LATEST_GP]
        .as_array()?
        .iter()
        .flat_map(|x: &serde_json::Value| x.as_u64())
        .collect::<Vec<_>>();
    Some(seats)
}

pub fn send_new_legis_info_mail(
    mail_to: &str,
    vote_results: &[&(i32, VoteResult)],
    _seats: &[u64],
    _delegates: &[Delegate],
    _colors: HashMap<String, String>,
    is_finished: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // let content = EMAIL_TEMPLATE.replace("{*OTP*}", &white_splitted);
    // let content = content.replace("{*MINUTOS*}", &(*EMAIL_EXPIRATION_SECONDS / 60).to_string());

    let vote_results = vote_results
        .iter()
        .map(|x| (*x).clone())
        .collect::<Vec<_>>();
    // let delegates = delegates.to_vec();
    // let seats = seats.to_vec();

    let mail_to = mail_to.to_string();
    std::thread::spawn(move || {
        let mut vote_results_html = String::new();
        for (idx, (_id, vote_result)) in vote_results.iter().enumerate() {
            if idx >= 5 {
                if vote_results.len() >= 8 {
                    vote_results_html.push_str(&"Und weitere ...");
                }
                break;
            }
            // let parliament_img = generate_parliament_image_with_delegates(
            //     &vote_result.votes,
            //     &delegates,
            //     &seats,
            //     &colors,
            // );

            // let mut png_parliament_img_buf = Vec::new();
            // let res = PngEncoder::new(&mut png_parliament_img_buf).write_image(
            //     &parliament_img,
            //     parliament_img.width(),
            //     parliament_img.height(),
            //     Rgba::<u8>::COLOR_TYPE,
            // );

            // if let Err(e) = res {
            //     log::error!("could not png encode image: {e:?}");
            //     continue;
            // }

            // #[allow(deprecated)]
            // let base64_parliament_img = base64::encode(png_parliament_img_buf);

            let vote_outcome = match &vote_result.legislative_initiative.accepted {
                Some(status) if status == "a" => "Angenommen",
                Some(status) if status == "d" || status == "p" => "Abgelehnt",
                _ => "In Bearbeitung",
            };

            let vote_type = match vote_result
                .legislative_initiative
                .voting
                .clone()
                .unwrap_or("_".into())
                .as_str()
            {
                "Law" => "Gesetz",
                "Report" => "Bericht",
                "Amendment" => "Abänderung",
                "Resolution" => "Entschließung",
                _ => &vote_result.legislative_initiative.ityp,
            };

            let vote_description = vote_result
                .ai_summary
                .as_ref()
                .map(|summary| &summary.short_title)
                .unwrap_or(&vote_result.legislative_initiative.description);

            let party_votes_html = vote_result
                .votes
                .iter()
                .map(|vote| {
                    let symbol = if vote.infavor {
                        "<span style=\"color: #22c55e;\">✓</span>"
                    } else {
                        "<span style=\"color: #ef4444;\">✗</span>"
                    };
                    format!("{} {}", vote.party, symbol)
                })
                .collect::<Vec<_>>()
                .join("  ");

            let vote_date_badge = vote_result
                .legislative_initiative
                .vote_date
                .map(|d| {
                    format!(
                        r#"<span class="dm-badge" style="display: inline-block; background-color: #fae9da; color: #000000; font-family: Arial, sans-serif; font-size: 12px; line-height: 16px; font-weight: 600; border-radius: 6px; padding: 4px 8px; margin: 0 6px 6px 0; white-space: nowrap;">{}</span>"#,
                        d.format("%d.%m.%Y")
                    )
                })
                .unwrap_or_default();

            let vote_result_html_entry = VOTE_RESULT_ENTRY_TEMPLATE
                .replace("{*VOTE_RESULT_DESCRIPTION*}", &vote_description)
                .replace("{*VOTE_RESULT_GP*}", &vote_result.legislative_initiative.gp)
                .replace("{*VOTE_RESULT_TYPE*}", vote_type)
                .replace("{*VOTE_RESULT_OUTCOME*}", vote_outcome)
                .replace("{*VOTE_RESULT_DATE_BADGE*}", &vote_date_badge)
                .replace("{*VOTE_RESULT_PARTY_VOTES*}", &party_votes_html)
                .replace(
                    "{*VOTE_RESULT_SOMES_URL*}",
                    &format!(
                        "https://somes.at/vote_result/{gp}/{ityp}/{inr}",
                        gp = vote_result.legislative_initiative.gp,
                        ityp = vote_result.legislative_initiative.ityp,
                        inr = vote_result.legislative_initiative.inr
                    ),
                );

            vote_results_html.push_str(&vote_result_html_entry);
        }

        let title = if is_finished {
            "Neue Abstimmungsergebnisse"
        } else {
            "Neue Anträge"
        };

        let description = if is_finished {
            "Der Nationalrat hat kürzlich verschiedene Abstimmungen gehalten, die zu Ihrer Interessenswahl passen."
        } else {
            "Kürzlich wurden neue Anträge eingebracht, die zu Ihrere Interessenswahl passen."
        };

        log::info!("Sending composed new vote result mail to {mail_to}");
        let content = LEGIS_INIT_EMAIL_TEMPLATE
            .replace("{*TITLE*}", title)
            .replace("{*DESCRIPTION*}", description)
            .replace("{*VOTE_RESULTS*}", &vote_results_html);

        let subject = if is_finished {
            "Neue Abstimmungen auf somes"
        } else {
            "Neue Anträge auf somes"
        };

        if let Err(e) = send_mail(&MAILER, &mail_to, subject, content) {
            log::error!("could not send legis info mail: {e:?}");
        }
    });

    Ok(())
}

pub fn notify_users(
    seats: &[u64],
    delegates: &[Delegate],
    user_legis_inits: &HashMap<String, Vec<&(i32, VoteResult)>>,
    colors: HashMap<String, String>,
    is_finished: bool,
) {
    for (mail_to, legis_inits) in user_legis_inits {
        if let Err(e) = send_new_legis_info_mail(
            mail_to,
            legis_inits,
            seats,
            delegates,
            colors.clone(),
            is_finished,
        ) {
            log::error!("could not send legis info mail: {e:?}");
        }
    }
    // for notifier_mail in find_notifiers(pool, legis_init_topics).await {}
}

#[cfg(test)]
mod tests {
    use combx::{PostgresEnhancer, VoteResult};
    use somes_scraper::{AustriaScraper, legis_init::LegislativeInitiatives};

    use crate::{
        connect_pg2,
        legislative_initiatives::notify::{
            create_color_map, generate_parliament_image, get_current_delegates,
            get_current_parties, get_current_seats, send_new_legis_info_mail,
        },
    };

    #[tokio::test]
    async fn test_get_current_seats() {
        let seats = get_current_seats().await.unwrap();
        println!("seats: {seats:?}");
        assert_eq!(seats, [20, 28, 37, 43, 48, 54])
    }

    #[tokio::test]
    async fn test_generate_parliament_image() {
        let scraper = AustriaScraper::default();
        let legis_init = scraper
            // .get_legis_init("/gegenstand/XXVIII/I/14")
            .get_legis_init("/gegenstand/XXVIII/A/6")
            .await
            .unwrap();

        let pg = connect_pg2().await;
        let legis_init = VoteResult::from_legis_init(PostgresEnhancer(pg), legis_init, None).await;
        generate_parliament_image(&legis_init.votes, &[20, 28, 37, 43, 48, 54]).await;
    }

    #[tokio::test]
    async fn test_send_new_votes_info_mail() {
        let scraper = AustriaScraper::default();
        let legis_init = scraper
            .get_legis_init("/gegenstand/XXVIII/A/164")
            .await
            .unwrap();

        /*let legis_init = scraper
        .get_legis_init("/gegenstand/XXVII/I/2710")
        .await
        .unwrap();*/
        // gegenstand/XXVIII/A/6

        let seats = get_current_seats().await.unwrap_or_default();
        let delegates = get_current_delegates().await.unwrap_or_default();
        let colors = create_color_map(get_current_parties().await.unwrap_or_default());

        let pg = connect_pg2().await;
        let legis_init = VoteResult::from_legis_init(PostgresEnhancer(pg), legis_init, None).await;

        send_new_legis_info_mail(
            crate::email::TEST_MAIL_RECEIVER,
            &[&(19106479, legis_init)],
            &seats,
            &delegates,
            colors,
            true,
        )
        .unwrap();

        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}
