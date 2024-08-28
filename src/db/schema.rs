// @generated automatically by Diesel CLI.

diesel::table! {
    answers (id) {
        id -> Int4,
        question_id -> Int4,
        delegate_id -> Nullable<Int4>,
        via_mail -> Nullable<Bool>,
        received_at -> Timestamp,
        text_type -> Nullable<Varchar>,
        mail_subject -> Nullable<Varchar>,
        body -> Text,
    }
}

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
        gender -> Nullable<Varchar>,
        is_active -> Nullable<Bool>,
        birthdate -> Nullable<Date>,
    }
}

diesel::table! {
    delegates_divisions (id) {
        id -> Int4,
        delegate_id -> Int4,
        division -> Nullable<Varchar>,
    }
}

diesel::table! {
    eurovoc_topics_legis_init (id) {
        id -> Int4,
        topic -> Varchar,
        legislative_initiatives_id -> Int4,
    }
}

diesel::table! {
    eurovoc_topics_ministrial_proposals (id) {
        id -> Int4,
        topic -> Varchar,
        ministrial_proposal_id -> Int4,
    }
}

diesel::table! {
    eurovoc_topics_proposals (id) {
        id -> Int4,
        topic -> Varchar,
        proposals_id -> Varchar,
    }
}

diesel::table! {
    legislative_documents (id) {
        id -> Int4,
        legislative_initiatives_id -> Int4,
        title -> Nullable<Varchar>,
        document_type -> Varchar,
        document_url -> Varchar,
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
        pre_declined_type -> Nullable<Varchar>,
        voted_by_name -> Bool,
        description -> Text,
        accepted -> Nullable<Varchar>,
        created_at -> Date,
        appeared_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        requires_simple_majority -> Nullable<Bool>,
    }
}

diesel::table! {
    mandates (id) {
        id -> Int4,
        delegate_id -> Int4,
        is_nr -> Nullable<Bool>,
        is_gov_official -> Nullable<Bool>,
        name -> Varchar,
        party -> Nullable<Varchar>,
        start_date -> Date,
        end_date -> Nullable<Date>,
    }
}

diesel::table! {
    ministrial_proposals (id) {
        id -> Int4,
        ityp -> Varchar,
        gp -> Varchar,
        inr -> Int4,
        emphasis -> Nullable<Varchar>,
        title -> Text,
        description -> Text,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        due_to -> Date,
        ressort -> Nullable<Varchar>,
        ressort_shortform -> Nullable<Varchar>,
        legis_init_gp -> Nullable<Varchar>,
        legis_init_inr -> Nullable<Int4>,
        legis_init_ityp -> Nullable<Varchar>,
    }
}

diesel::table! {
    ministrial_proposals_documents (id) {
        id -> Int4,
        ministrial_proposal_id -> Int4,
        title -> Nullable<Varchar>,
        document_type -> Varchar,
        document_url -> Varchar,
    }
}

diesel::table! {
    named_vote_info (id) {
        id -> Int4,
        legis_init_id -> Int4,
        pro_count -> Int4,
        contra_count -> Int4,
        given_vote_sum -> Int4,
        invalid_count -> Int4,
    }
}

diesel::table! {
    named_votes (id) {
        id -> Int4,
        infavor -> Nullable<Bool>,
        was_absent -> Nullable<Bool>,
        lev -> Int8,
        similiarity_score -> Int8,
        searched_with -> Nullable<Varchar>,
        matched_with -> Varchar,
        delegate_id -> Int4,
        named_vote_info_id -> Int4,
        manually_matched -> Nullable<Bool>,
    }
}

diesel::table! {
    other_keyword_topics_legis_init (id) {
        id -> Int4,
        topic -> Varchar,
        legislative_initiatives_id -> Int4,
    }
}

diesel::table! {
    other_keyword_topics_ministrial_proposals (id) {
        id -> Int4,
        topic -> Varchar,
        ministrial_proposal_id -> Int4,
    }
}

diesel::table! {
    other_keyword_topics_proposals (id) {
        id -> Int4,
        topic -> Varchar,
        proposals_id -> Varchar,
    }
}

diesel::table! {
    parties (id) {
        id -> Int4,
        name -> Varchar,
        gp -> Varchar,
        code -> Varchar,
        fraction -> Int4,
        color -> Varchar,
    }
}

diesel::table! {
    plenar_infos (id) {
        id -> Int4,
        add_date -> Timestamp,
        inr -> Int4,
        title -> Varchar,
        description -> Varchar,
        legislative_period -> Varchar,
    }
}

diesel::table! {
    plenar_speech_links (id) {
        id -> Int4,
        plenar_speech_id -> Int4,
        document_url -> Nullable<Varchar>,
    }
}

diesel::table! {
    plenar_speeches (id) {
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
    proposal_documents (id) {
        id -> Int4,
        proposal_id -> Nullable<Varchar>,
        title -> Nullable<Varchar>,
        document_type -> Varchar,
        document_url -> Varchar,
    }
}

diesel::table! {
    proposals (id) {
        id -> Varchar,
        ityp -> Varchar,
        gp -> Varchar,
        title -> Text,
        description -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    questions (id) {
        id -> Int4,
        issuer_id -> Int4,
        delegate_id -> Int4,
        title -> Varchar,
        body -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        is_sent -> Bool,
        is_sendable -> Bool,
        is_answerable -> Bool,
    }
}

diesel::table! {
    speech_complexity (id) {
        id -> Int4,
        speech_id -> Int4,
        flesch_kincaid -> Nullable<Float4>,
        smog -> Nullable<Float4>,
        gunning_fog -> Nullable<Float4>,
        coleman_liau -> Nullable<Float4>,
    }
}

diesel::table! {
    speeches (id) {
        id -> Int4,
        delegate_id -> Int4,
        infavor -> Nullable<Bool>,
        opinion -> Nullable<Varchar>,
        legislative_initiatives_id -> Int4,
    }
}

diesel::table! {
    speeches_html_urls (id) {
        id -> Int4,
        speech_id -> Int4,
        document_url -> Varchar,
    }
}

diesel::table! {
    topics_legis_init (id) {
        id -> Int4,
        topic -> Varchar,
        legislative_initiatives_id -> Int4,
    }
}

diesel::table! {
    topics_ministrial_proposals (id) {
        id -> Int4,
        topic -> Varchar,
        ministrial_proposal_id -> Int4,
    }
}

diesel::table! {
    topics_proposals (id) {
        id -> Int4,
        topic -> Varchar,
        proposals_id -> Varchar,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        username -> Varchar,
        email -> Varchar,
        password_hash -> Varchar,
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

diesel::joinable!(answers -> questions (question_id));
diesel::joinable!(call_to_order -> delegates (receiver_id));
diesel::joinable!(call_to_order -> plenar_infos (plenar_id));
diesel::joinable!(debates -> plenar_infos (plenar_id));
diesel::joinable!(delegates -> contacts (id));
diesel::joinable!(delegates_divisions -> delegates (delegate_id));
diesel::joinable!(eurovoc_topics_legis_init -> legislative_initiatives (legislative_initiatives_id));
diesel::joinable!(eurovoc_topics_ministrial_proposals -> ministrial_proposals (ministrial_proposal_id));
diesel::joinable!(eurovoc_topics_proposals -> proposals (proposals_id));
diesel::joinable!(legislative_documents -> legislative_initiatives (legislative_initiatives_id));
diesel::joinable!(mandates -> delegates (delegate_id));
diesel::joinable!(ministrial_proposals_documents -> ministrial_proposals (ministrial_proposal_id));
diesel::joinable!(named_vote_info -> legislative_initiatives (legis_init_id));
diesel::joinable!(named_votes -> delegates (delegate_id));
diesel::joinable!(named_votes -> named_vote_info (named_vote_info_id));
diesel::joinable!(other_keyword_topics_legis_init -> legislative_initiatives (legislative_initiatives_id));
diesel::joinable!(other_keyword_topics_ministrial_proposals -> ministrial_proposals (ministrial_proposal_id));
diesel::joinable!(other_keyword_topics_proposals -> proposals (proposals_id));
diesel::joinable!(plenar_speech_links -> plenar_speeches (plenar_speech_id));
diesel::joinable!(plenar_speeches -> debates (debate_id));
diesel::joinable!(plenar_speeches -> delegates (delegate_id));
diesel::joinable!(proposal_delegates -> delegates (delegate_id));
diesel::joinable!(proposal_delegates -> proposals (proposal_id));
diesel::joinable!(proposal_documents -> proposals (proposal_id));
diesel::joinable!(questions -> delegates (delegate_id));
diesel::joinable!(questions -> users (issuer_id));
diesel::joinable!(speech_complexity -> plenar_speeches (speech_id));
diesel::joinable!(speeches -> delegates (delegate_id));
diesel::joinable!(speeches -> legislative_initiatives (legislative_initiatives_id));
diesel::joinable!(speeches_html_urls -> speeches (speech_id));
diesel::joinable!(topics_legis_init -> legislative_initiatives (legislative_initiatives_id));
diesel::joinable!(topics_ministrial_proposals -> ministrial_proposals (ministrial_proposal_id));
diesel::joinable!(topics_proposals -> proposals (proposals_id));
diesel::joinable!(votes -> legislative_initiatives (legislative_initiatives_id));

diesel::allow_tables_to_appear_in_same_query!(
    answers,
    call_to_order,
    contacts,
    debates,
    delegates,
    delegates_divisions,
    eurovoc_topics_legis_init,
    eurovoc_topics_ministrial_proposals,
    eurovoc_topics_proposals,
    legislative_documents,
    legislative_initiatives,
    mandates,
    ministrial_proposals,
    ministrial_proposals_documents,
    named_vote_info,
    named_votes,
    other_keyword_topics_legis_init,
    other_keyword_topics_ministrial_proposals,
    other_keyword_topics_proposals,
    parties,
    plenar_infos,
    plenar_speech_links,
    plenar_speeches,
    proposal_delegates,
    proposal_documents,
    proposals,
    questions,
    speech_complexity,
    speeches,
    speeches_html_urls,
    topics_legis_init,
    topics_ministrial_proposals,
    topics_proposals,
    users,
    votes,
);
