use chrono::{DateTime, TimeDelta, Utc};

pub fn update_days() -> i64 {
    std::env::var("POLITICAL_ANALYSIS_UPDATE_DAYS")
        .unwrap_or_else(|_| "90".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse POLITICAL_ANALYSIS_UPDATE_DAYS env var: {e}. Using default value of 90 days"
            );
            90
        })
}

pub fn update_delta() -> TimeDelta {
    TimeDelta::days(update_days())
}

pub fn is_fresh(updated_at: Option<DateTime<Utc>>, today: DateTime<Utc>) -> bool {
    updated_at.is_some_and(|updated_at| today - updated_at < update_delta())
}

pub fn vector_update_skip() -> bool {
    std::env::var("VECTOR_UPDATE_SKIP")
        .unwrap_or("false".into())
        .parse()
        .unwrap_or_default()
}

pub fn stance_update_skip() -> bool {
    std::env::var("STANCE_UPDATE_SKIP")
        .unwrap_or("false".into())
        .parse()
        .unwrap_or_default()
}

pub fn sleep_seconds() -> u64 {
    std::env::var("POLITICAL_ANALYSIS_SLEEP")
        .unwrap_or_else(|_| "800".to_string())
        .parse()
        .unwrap_or_else(|e| {
            log::warn!(
                "Could not parse POLITICAL_ANALYSIS_SLEEP env var: {e}. Using default value of 800 seconds"
            );
            800
        })
}
