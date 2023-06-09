// @generated automatically by Diesel CLI.

diesel::table! {
    contacts (id) {
        id -> Int4,
        mail -> Nullable<Varchar>,
        phone_number -> Nullable<Varchar>,
        twitter_url -> Nullable<Varchar>,
        facebook_url -> Nullable<Varchar>,
        website_url -> Nullable<Varchar>,
        tiktok_url -> Nullable<Varchar>,
        instagram_url -> Nullable<Varchar>,
        youtube_url -> Nullable<Varchar>,
    }
}

diesel::table! {
    delegates (id) {
        id -> Int4,
        name -> Varchar,
        party -> Nullable<Varchar>,
        image_url -> Nullable<Varchar>,
        constituency -> Nullable<Varchar>,
        council -> Nullable<Varchar>,
        seat_row -> Nullable<Int4>,
        seat_col -> Nullable<Int4>,
        contacts_id -> Nullable<Int4>,
    }
}

diesel::table! {
    legislative_initiatives (id) {
        id -> Varchar,
        ityp -> Varchar,
        gp -> Varchar,
        emphasis -> Nullable<Varchar>,
        title -> Text,
        description -> Text,
        accepted -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    parties (id) {
        id -> Varchar,
        name -> Varchar,
    }
}

diesel::table! {
    proposal_delegates (proposal_id, delegate_id) {
        proposal_id -> Varchar,
        delegate_id -> Int4,
        is_receiver -> Bool,
    }
}

diesel::table! {
    proposals (id) {
        id -> Varchar,
        ityp -> Varchar,
        gp -> Varchar,
        title -> Text,
        description -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    speeches (delegate_id, legislative_initiatives_id) {
        delegate_id -> Int4,
        infavor -> Nullable<Bool>,
        legislative_initiatives_id -> Varchar,
    }
}

diesel::table! {
    votes (party, legislative_initiatives_id) {
        party -> Varchar,
        fraction -> Int4,
        infavor -> Bool,
        legislative_initiatives_id -> Varchar,
    }
}

diesel::joinable!(delegates -> contacts (contacts_id));
diesel::joinable!(proposal_delegates -> delegates (delegate_id));
diesel::joinable!(proposal_delegates -> proposals (proposal_id));
diesel::joinable!(speeches -> delegates (delegate_id));
diesel::joinable!(speeches -> legislative_initiatives (legislative_initiatives_id));
diesel::joinable!(votes -> legislative_initiatives (legislative_initiatives_id));

diesel::allow_tables_to_appear_in_same_query!(
    contacts,
    delegates,
    legislative_initiatives,
    parties,
    proposal_delegates,
    proposals,
    speeches,
    votes,
);
