use std::collections::HashMap;

use ::combx::Parliament;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{EXPO_PUSH_ACCESS_TOKEN, EXPO_PUSH_URL};

const EXPO_PUSH_RECEIPTS_URL: &str = "https://exp.host/--/api/v2/push/getReceipts";
const EXPO_BATCH_SIZE: usize = 100;
/// Expo caps a single message at 4 KiB; keep push bodies well under that.
const MAX_PUSH_BODY_LEN: usize = 200;

pub static PUSH_CLIENT: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);

pub fn parliament_country(parliament: Parliament) -> &'static str {
    match parliament {
        Parliament::At => "at",
        Parliament::Eu => "eu",
    }
}

pub fn truncate_for_push(text: &str) -> String {
    if text.chars().count() <= MAX_PUSH_BODY_LEN {
        return text.to_string();
    }

    let mut truncated = text.chars().take(MAX_PUSH_BODY_LEN).collect::<String>();
    truncated.push('…');
    truncated
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum PushTarget {
    #[serde(rename = "vote")]
    Vote {
        country: &'static str,
        gp: String,
        ityp: String,
        inr: i32,
        category: &'static str,
    },
    #[serde(rename = "ministerial_proposal")]
    MinisterialProposal {
        country: &'static str,
        gp: String,
        inr: i32,
        category: &'static str,
    },
    #[serde(rename = "decree")]
    Decree {
        country: &'static str,
        ris_id: String,
        category: &'static str,
    },
    #[serde(rename = "delegate")]
    Delegate {
        country: &'static str,
        id: i32,
        category: &'static str,
    },
}

#[derive(Serialize)]
pub struct ExpoPushMessage {
    pub to: String,
    pub title: String,
    pub body: String,
    #[serde(rename = "channelId")]
    pub channel_id: &'static str,
    // always None -> serializes to `null`, matching the app's foreground handler
    pub sound: Option<()>,
    pub data: PushTarget,
}

#[derive(Deserialize, Debug, Default)]
struct ExpoTicketDetails {
    error: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ExpoTicket {
    status: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    details: Option<ExpoTicketDetails>,
}

#[derive(Deserialize, Debug)]
struct ExpoTicketResponse {
    data: Vec<ExpoTicket>,
}

#[derive(Deserialize, Debug)]
struct ExpoReceipt {
    status: String,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    details: Option<ExpoTicketDetails>,
}

#[derive(Deserialize, Debug)]
struct ExpoReceiptResponse {
    data: HashMap<String, ExpoReceipt>,
}

#[derive(Serialize)]
struct GetReceiptsRequest<'a> {
    ids: &'a [String],
}

/// Sends Expo push messages in batches of up to 100, then (in the background)
/// polls read receipts once to self-clean tokens Expo reports as unregistered.
pub async fn send_expo_push_notifications(
    client: &reqwest::Client,
    pool: &PgPool,
    messages: Vec<ExpoPushMessage>,
) {
    if messages.is_empty() {
        return;
    }

    for chunk in messages.chunks(EXPO_BATCH_SIZE) {
        let mut request = client.post(EXPO_PUSH_URL.as_str()).json(chunk);
        if let Some(token) = EXPO_PUSH_ACCESS_TOKEN.as_ref() {
            request = request.bearer_auth(token);
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(e) => {
                log::error!("could not send expo push notification batch: {e:?}");
                continue;
            }
        };

        let ticket_response = match response.json::<ExpoTicketResponse>().await {
            Ok(body) => body,
            Err(e) => {
                log::error!("could not parse expo push ticket response: {e:?}");
                continue;
            }
        };

        let mut ticket_ids = Vec::new();
        for (message, ticket) in chunk.iter().zip(ticket_response.data.iter()) {
            match ticket.status.as_str() {
                "ok" => {
                    if let Some(id) = &ticket.id {
                        ticket_ids.push((id.clone(), message.to.clone()));
                    }
                }
                _ => {
                    log::error!(
                        "expo push ticket error for {}: {:?} ({:?})",
                        message.to,
                        ticket.message,
                        ticket.details.as_ref().and_then(|d| d.error.as_ref())
                    );
                }
            }
        }

        if !ticket_ids.is_empty() {
            let pool = pool.clone();
            let client = client.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                check_expo_receipts(&client, &pool, ticket_ids).await;
            });
        }
    }
}

async fn check_expo_receipts(
    client: &reqwest::Client,
    pool: &PgPool,
    ticket_ids: Vec<(String, String)>,
) {
    let ids = ticket_ids
        .iter()
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();

    let mut request = client
        .post(EXPO_PUSH_RECEIPTS_URL)
        .json(&GetReceiptsRequest { ids: &ids });
    if let Some(token) = EXPO_PUSH_ACCESS_TOKEN.as_ref() {
        request = request.bearer_auth(token);
    }

    let response = match request.send().await {
        Ok(response) => response,
        Err(e) => {
            log::error!("could not fetch expo push receipts: {e:?}");
            return;
        }
    };

    let receipts = match response.json::<ExpoReceiptResponse>().await {
        Ok(body) => body,
        Err(e) => {
            log::error!("could not parse expo push receipt response: {e:?}");
            return;
        }
    };

    for (ticket_id, push_token) in ticket_ids {
        let Some(receipt) = receipts.data.get(&ticket_id) else {
            continue;
        };
        if receipt.status == "ok" {
            continue;
        }

        let error = receipt.details.as_ref().and_then(|d| d.error.as_deref());
        match error {
            Some("DeviceNotRegistered") => {
                if let Err(e) = sqlx::query!(
                    "delete from expo_push_tokens where push_token = $1",
                    push_token
                )
                .execute(pool)
                .await
                {
                    log::error!("could not delete stale expo push token: {e:?}");
                }
            }
            Some("MessageRateExceeded") => {
                log::warn!(
                    "expo push rate exceeded for {push_token}: {:?}",
                    receipt.message
                );
            }
            _ => {
                log::error!(
                    "expo push receipt error for {push_token}: {:?} ({error:?})",
                    receipt.message
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AT_DATABASE_URL, connect_pg2};

    // Replace with a real device token from the app's notification settings
    // screen before running manually; this test sends a real push.
    const TEST_EXPO_PUSH_TOKEN: &str = "ExponentPushToken[REPLACE_WITH_REAL_DEVICE_TOKEN]";

    #[tokio::test]
    async fn test_send_vote_push_preview() {
        let pg = connect_pg2(AT_DATABASE_URL).await;

        let message = ExpoPushMessage {
            to: TEST_EXPO_PUSH_TOKEN.to_string(),
            title: "Neues Abstimmungsergebnis".to_string(),
            body: "Antrag auf Änderung des Klimaschutzgesetzes wurde angenommen.".to_string(),
            channel_id: "default",
            sound: None,
            data: PushTarget::Vote {
                country: "at",
                gp: "XXVIII".to_string(),
                ityp: "A".to_string(),
                inr: 123,
                category: "send_new_vote_result_by_favo",
            },
        };

        send_expo_push_notifications(&PUSH_CLIENT, &pg, vec![message]).await;

        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
    }
}
