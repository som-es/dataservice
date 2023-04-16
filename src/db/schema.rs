// @generated automatically by Diesel CLI.

diesel::table! {
    delegates (id) {
        id -> Int4,
        name -> Varchar,
        party -> Nullable<Varchar>,
        constituency -> Nullable<Varchar>,
        council -> Nullable<Varchar>,
        seat_row -> Nullable<Int4>,
        seat_col -> Nullable<Int4>,
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

diesel::joinable!(proposal_delegates -> delegates (delegate_id));
diesel::joinable!(proposal_delegates -> proposals (proposal_id));

diesel::allow_tables_to_appear_in_same_query!(
    delegates,
    proposal_delegates,
    proposals,
);
