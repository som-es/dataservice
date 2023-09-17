// @generated automatically by Diesel CLI.

diesel::table! {
    call_to_order (id) {
        #[max_length = 20]
        id -> Varchar,
        plenar_id -> Int4,
        inr -> Int4,
        receiver_id -> Int4,
    }
}

diesel::table! {
    contacts (id) {
        id -> Int4,
        #[max_length = 255]
        mail -> Nullable<Varchar>,
        #[max_length = 255]
        phone_number -> Nullable<Varchar>,
        #[max_length = 255]
        twitter_url -> Nullable<Varchar>,
        #[max_length = 255]
        facebook_url -> Nullable<Varchar>,
        #[max_length = 255]
        website_url -> Nullable<Varchar>,
        #[max_length = 255]
        tiktok_url -> Nullable<Varchar>,
        #[max_length = 255]
        instagram_url -> Nullable<Varchar>,
        #[max_length = 255]
        youtube_url -> Nullable<Varchar>,
    }
}

diesel::table! {
    debates (id) {
        id -> Int4,
        plenar_id -> Int4,
        #[max_length = 512]
        about -> Varchar,
    }
}

diesel::table! {
    delegates (id) {
        id -> Int4,
        #[max_length = 255]
        name -> Varchar,
        #[max_length = 100]
        party -> Nullable<Varchar>,
        #[max_length = 300]
        image_url -> Nullable<Varchar>,
        #[max_length = 200]
        constituency -> Nullable<Varchar>,
        #[max_length = 200]
        council -> Nullable<Varchar>,
        seat_row -> Nullable<Int4>,
        seat_col -> Nullable<Int4>,
        contacts_id -> Nullable<Int4>,
        #[max_length = 1]
        gender -> Nullable<Varchar>,
        is_active -> Nullable<Bool>,
        birthdate -> Nullable<Date>,
    }
}

diesel::table! {
    legislative_initiatives (id) {
        id -> Int4,
        #[max_length = 7]
        ityp -> Varchar,
        #[max_length = 7]
        gp -> Varchar,
        inr -> Int4,
        #[max_length = 4000]
        emphasis -> Nullable<Varchar>,
        title -> Text,
        description -> Text,
        accepted -> Nullable<Bool>,
        created_at -> Date,
    }
}

diesel::table! {
    mandates (id) {
        id -> Int4,
        delegate_id -> Int4,
        #[max_length = 312]
        name -> Varchar,
        #[max_length = 100]
        party -> Nullable<Varchar>,
        start_date -> Date,
        end_date -> Nullable<Date>,
    }
}

diesel::table! {
    parties (name) {
        #[max_length = 8]
        name -> Varchar,
        fraction -> Int4,
        #[max_length = 7]
        color -> Varchar,
    }
}

diesel::table! {
    plenar_infos (id) {
        id -> Int4,
        add_date -> Timestamptz,
        inr -> Int4,
        #[max_length = 255]
        title -> Varchar,
        #[max_length = 255]
        description -> Varchar,
        #[max_length = 9]
        legislative_period -> Varchar,
    }
}

diesel::table! {
    plenar_speeches (id, delegate_id, debate_id) {
        id -> Int4,
        delegate_id -> Int4,
        debate_id -> Int4,
        #[max_length = 40]
        opinion -> Nullable<Varchar>,
        spoken_text -> Nullable<Text>,
        duration_in_seconds -> Nullable<Int4>,
        #[max_length = 40]
        ty -> Nullable<Varchar>,
    }
}

diesel::table! {
    proposal_delegates (proposal_id, delegate_id) {
        #[max_length = 20]
        proposal_id -> Varchar,
        delegate_id -> Int4,
        is_receiver -> Bool,
    }
}

diesel::table! {
    proposals (id) {
        #[max_length = 20]
        id -> Varchar,
        #[max_length = 7]
        ityp -> Varchar,
        #[max_length = 7]
        gp -> Varchar,
        title -> Text,
        description -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    questions (id) {
        id -> Int4,
        issuer_id -> Int4,
        delegate_id -> Int4,
        #[max_length = 255]
        title -> Varchar,
        body -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
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
    users (id) {
        id -> Int4,
        #[max_length = 255]
        username -> Varchar,
        #[max_length = 300]
        email -> Varchar,
        #[max_length = 356]
        password_hash -> Varchar,
    }
}

diesel::table! {
    votes (party, legislative_initiatives_id) {
        #[max_length = 30]
        party -> Varchar,
        fraction -> Int4,
        infavor -> Bool,
        legislative_initiatives_id -> Int4,
    }
}

diesel::joinable!(call_to_order -> delegates (receiver_id));
diesel::joinable!(call_to_order -> plenar_infos (plenar_id));
diesel::joinable!(debates -> plenar_infos (plenar_id));
diesel::joinable!(delegates -> contacts (contacts_id));
diesel::joinable!(mandates -> delegates (delegate_id));
diesel::joinable!(plenar_speeches -> debates (debate_id));
diesel::joinable!(plenar_speeches -> delegates (delegate_id));
diesel::joinable!(proposal_delegates -> delegates (delegate_id));
diesel::joinable!(proposal_delegates -> proposals (proposal_id));
diesel::joinable!(questions -> delegates (delegate_id));
diesel::joinable!(questions -> users (issuer_id));
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
    questions,
    speeches,
    users,
    votes,
);
