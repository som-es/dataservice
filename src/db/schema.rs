// @generated automatically by Diesel CLI.

diesel::table! {
    call_to_order (id) {
        id -> Varchar,
        plenar_id -> Int4,
        inr -> Int4,
        receiver_id -> Int4,
    }
}

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
    debates (id) {
        id -> Int4,
        plenar_id -> Int4,
        about -> Varchar,
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
        gender -> Nullable<Varchar>,
        is_active -> Nullable<Bool>,
    }
}

diesel::table! {
    legislative_initiatives (id) {
        id -> Int4,
        ityp -> Varchar,
        gp -> Varchar,
        inr -> Int4,
        emphasis -> Nullable<Varchar>,
        title -> Text,
        description -> Text,
        accepted -> Bool,
        created_at -> Date,
    }
}

diesel::table! {
    mandates (id) {
        id -> Int4,
        delegate_id -> Int4,
        name -> Varchar,
        party -> Nullable<Varchar>,
        start_date -> Date,
        end_date -> Nullable<Date>,
    }
}

diesel::table! {
    parties (id) {
        id -> Varchar,
        name -> Varchar,
    }
}

diesel::table! {
    plenar_infos (id) {
        id -> Int4,
        add_date -> Timestamptz,
        inr -> Int4,
        title -> Varchar,
        description -> Varchar,
        legislative_period -> Varchar,
    }
}

diesel::table! {
    plenar_speeches (id, delegate_id, debate_id) {
        id -> Int4,
        delegate_id -> Int4,
        debate_id -> Int4,
        opinion -> Nullable<Varchar>,
        spoken_text -> Nullable<Text>,
        duration_in_seconds -> Nullable<Int4>,
        ty -> Nullable<Varchar>,
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
        legislative_initiatives_id -> Int4,
    }
}

diesel::table! {
    votes (party, legislative_initiatives_id) {
        party -> Varchar,
        fraction -> Int4,
        infavor -> Bool,
        legislative_initiatives_id -> Int4,
    }
}

diesel::joinable!(call_to_order -> plenar_infos (plenar_id));
diesel::joinable!(debates -> plenar_infos (plenar_id));
diesel::joinable!(delegates -> contacts (contacts_id));
diesel::joinable!(mandates -> delegates (delegate_id));
diesel::joinable!(plenar_speeches -> debates (debate_id));
diesel::joinable!(plenar_speeches -> delegates (delegate_id));
diesel::joinable!(proposal_delegates -> delegates (delegate_id));
diesel::joinable!(proposal_delegates -> proposals (proposal_id));
diesel::joinable!(speeches -> delegates (delegate_id));
diesel::joinable!(speeches -> legislative_initiatives (legislative_initiatives_id));
diesel::joinable!(votes -> legislative_initiatives (legislative_initiatives_id));

diesel::allow_tables_to_appear_in_same_query!(
    call_to_order,
    contacts,
    debates,
    delegates,
    legislative_initiatives,
    mandates,
    parties,
    plenar_infos,
    plenar_speeches,
    proposal_delegates,
    proposals,
    speeches,
    votes,
);
