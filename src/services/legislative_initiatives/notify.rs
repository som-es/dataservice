use std::collections::HashMap;

use image::{Rgb, Rgba};
use imageproc::{
    drawing::{draw_filled_circle_mut, draw_filled_rect_mut},
    rect::Rect,
};
use somes_common_lib::Delegate;
use somes_scraper::Vote;
use sqlx::{query, PgPool};

use crate::{GPS, LATEST_GP};

pub async fn find_notifiers(pool: &PgPool, legis_init_topics: &[String]) -> Vec<String> {
    let ids = query!("select id, email from somes_user where is_email_hashed is false")
        .fetch_all(pool)
        .await
        .unwrap_or_default();
    let mut to_notify_emails = vec![];
    for id in ids {
        let user_topics = query!("select topic_name from user_topics inner join unique_topics ut on ut.id = topic_id where user_id = $1", id.id).fetch_all(pool).await.unwrap_or_default().into_iter().map(|x| x.topic_name).collect::<Vec<_>>();
        for legis_init_topic in legis_init_topics {
            if user_topics.contains(legis_init_topic) {
                to_notify_emails.push(id.email);
                break;
            }
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

fn convert_votes_to_party_color_map(votes: &[Vote]) -> HashMap<String, Rgba<u8>> {
    votes.iter()
        .map(|x| {
            let mut rgba = Rgba(u32::from_str_radix(&x.party.color.as_str()[1..], 16).unwrap().to_be_bytes());
            if x.infavor {
                rgba.0[3] = 255;
            } else {
                rgba.0[3] = 125;
            }
            (x.party.name.clone(), rgba)
        }).collect::<HashMap<String, Rgba<u8>>>()
}

fn enrich_parliament_with_colors(
    parliament: Vec<Vec<(f32, f32)>>,
    votes: &[Vote],
    delegates: &[Delegate]
) -> Vec<Vec<(f32, f32, Rgba<u8>)>> {
    let white = Rgba([255u8, 255u8, 255u8, 0u8]);

    let mut parliament = parliament
        .into_iter()
        .map(|x| {
            x.into_iter()
                .map(|(x, y)| (x, y, white))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let votes_to_party_map = convert_votes_to_party_color_map(votes);

    for delegate in delegates {
        match (delegate.seat_row, delegate.seat_col, &delegate.party) {
            (Some(seat_row), Some(seat_col), Some(ref party)) => {
                let seat_row = seat_row as usize;
                let seat_col = seat_col as usize;
                let color = *votes_to_party_map.get(party).unwrap_or(&Rgba([125, 125, 125, 255u8]));
                parliament[seat_row - 1][seat_col - 1].2 = color;
            }
            
            _ => {}
        }
    }

    parliament
}

pub fn generate_parliament_image(votes: &[Vote], delegates: &[Delegate], seats: &[u64]) {
    let parliament = setup_parliament(seats);
    let parliament = enrich_parliament_with_colors(parliament, votes, delegates);

    let mut image = image::RgbaImage::new(IMG_WIDTH, IMG_HEIGHT / 2 + 60);

    let white = Rgba([255u8, 255u8, 255u8, 255u8]);
    let red = Rgba([255u8, 0, 0, 255u8]);

    draw_filled_rect_mut(
        &mut image,
        Rect::at(0, 0).of_size(IMG_WIDTH, IMG_HEIGHT / 2 + 60),
        white,
    );

    for half_circle in parliament {
        for (x, y, color) in half_circle {
            println!("x: {x}, y: {y}");
            draw_filled_circle_mut(&mut image, (x as i32, y as i32), CIRCLE_RADIUS, color);
            // draw_filled_circle_mut(&mut image, (IMG_WIDTH as i32 / 2, IMG_HEIGHT as i32 / 2), 10, red);
        }
    }

    image.save("test_parliament.png").unwrap();
    // image.
}

pub async fn get_current_seats() -> Option<Vec<u64>> {
    let seats: serde_json::Value = reqwest::get("https://somes.at/seats")
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

pub async fn notify_delegates(pool: &PgPool, legis_init_topics: &[String]) {
    for notifier_mail in find_notifiers(pool, legis_init_topics).await {}
}

#[cfg(test)]
mod tests {
    use crate::legislative_initiatives::notify::{generate_parliament_image, get_current_seats};

    #[tokio::test]
    async fn test_get_current_seats() {
        let seats = get_current_seats().await.unwrap();
        println!("seats: {seats:?}");
        assert_eq!(seats, [20, 28, 37, 43, 48, 54])
    }

    #[tokio::test]
    async fn test_generate_parliament_image() {
        generate_parliament_image(&[], &[20, 28, 37, 43, 48, 54]);
    }
}
