use std::collections::HashMap;

use combx::{DbVote, VoteResult};
use image::{codecs::png::PngEncoder, ImageEncoder, PixelWithColorType, Rgb, Rgba};
use imageproc::{
    drawing::{draw_filled_circle_mut, draw_filled_rect_mut},
    rect::Rect,
};
use somes_common_lib::Delegate;
use somes_scraper::{LegislativeInitiative, Party, Vote};
use sqlx::{query, PgPool};

use crate::{
    email::{send_mail, LEGIS_INIT_EMAIL_TEMPLATE, MAILER},
    LATEST_GP,
};
// could generify
pub async fn find_notifiers_legis_init<'a>(
    pool: &PgPool,
    legis_inits: &'a [(i32, VoteResult)],
) -> HashMap<String, Vec<&'a (i32, VoteResult)>> {
    let ids = query!("select id, email from somes_user where is_email_hashed is false and send_new_vote_results_mails")
        .fetch_all(pool)
        .await
        .unwrap_or_default();
    let mut to_notify_emails = HashMap::new();
    for id in ids {
        let user_topics = query!("select topic_name from user_topics inner join unique_eurovoc_topics ut on ut.id = topic_id where user_id = $1", id.id).fetch_all(pool).await.unwrap_or_default().into_iter().map(|x| x.topic_name).collect::<Vec<_>>();

        let relevant_legis_inits = legis_inits
            .iter()
            .filter(|legis_init| {
                for topic in &legis_init.1.eurovoc_topics {
                    if user_topics.contains(&topic.topic) {
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
    println!("vtpm: {votes_to_party_map:?}");

    for delegate in delegates {
        match (delegate.seat_row, delegate.seat_col, &delegate.party) {
            (Some(seat_row), Some(seat_col), Some(ref party)) => {
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
    legis_inits: &[&(i32, VoteResult)],
    seats: &[u64],
    delegates: &[Delegate],
    colors: HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // let content = EMAIL_TEMPLATE.replace("{*OTP*}", &white_splitted);
    // let content = content.replace("{*MINUTOS*}", &(*EMAIL_EXPIRATION_SECONDS / 60).to_string());

    let legis_inits = legis_inits.iter().map(|x| (*x).clone()).collect::<Vec<_>>();
    let delegates = delegates.to_vec();
    let seats = seats.to_vec();

    let mail_to = mail_to.to_string();
    std::thread::spawn(move || {
        let mut vote_results_html = String::new();
        for (idx, (_id, legis_init)) in legis_inits.iter().enumerate() {
            if idx >= 5 {
                if legis_inits.len() >= 6 {
                    vote_results_html.push_str(&"Und weitere ...");
                }
                break;
            }
            let parliament_img = generate_parliament_image_with_delegates(
                &legis_init.votes,
                &delegates,
                &seats,
                &colors,
            );

            let mut png_parliament_img_buf = Vec::new();
            let res = PngEncoder::new(&mut png_parliament_img_buf).write_image(
                &parliament_img,
                parliament_img.width(),
                parliament_img.height(),
                Rgba::<u8>::COLOR_TYPE,
            );

            if let Err(e) = res {
                log::error!("could not png encode image: {e:?}");
                continue;
            }

            #[allow(deprecated)]
            let base64_parliament_img = base64::encode(png_parliament_img_buf);

            let vote_result_html_entry = format!(
                r#"
                <a href="https://somes.at/alpha/vote_result/{gp}/{ityp}/{inr}">
                    <div style="display: flex; flex-direction: row; justify-content: space-between; width: 100%;">
                        <div style="border: 1px solid black; width: 50%; display: flex; justify-content: center; align-items: center;">
                            {desc}
                        </div>
                        <div style="border: 1px solid black; width: 60%;">
                            <img style="width: 70%;" src="data:image/png;base64,{base64_parliament_img}"/>
                        </div>
                    </div>
                </a>
            "#,
                desc = legis_init.legislative_initiative.description,
                gp = legis_init.legislative_initiative.gp,
                ityp = legis_init.legislative_initiative.ityp,
                inr = legis_init.legislative_initiative.inr,
            );

            vote_results_html.push_str(&vote_result_html_entry);
        }

        log::info!("Sending composed new vote result mail to {mail_to}");
        let content = LEGIS_INIT_EMAIL_TEMPLATE.replace("{*VOTE_RESULTS*}", &vote_results_html);
        println!("content: {content}");
        if let Err(e) = send_mail(&MAILER, &mail_to, "Neue Abstimmungen auf somes", content) {
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
) {
    for (mail_to, legis_inits) in user_legis_inits {
        if let Err(e) =
            send_new_legis_info_mail(mail_to, legis_inits, seats, delegates, colors.clone())
        {
            log::error!("could not send legis info mail: {e:?}");
        }
    }
    // for notifier_mail in find_notifiers(pool, legis_init_topics).await {}
}

#[cfg(test)]
mod tests {
    use combx::{PostgresEnhancer, VoteResult};
    use somes_scraper::{legis_init::LegislativeInitiatives, AustriaScraper};

    use crate::{
        connect_pg,
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

        let pg = connect_pg().await;
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

        let pg = connect_pg().await;
        let legis_init = VoteResult::from_legis_init(PostgresEnhancer(pg), legis_init, None).await;

        send_new_legis_info_mail(
            "",
            &[&(19106479, legis_init) /* &(10527230, legis_init1)*/],
            &seats,
            &delegates,
            colors,
        )
        .unwrap();

        loop {}
    }
}
