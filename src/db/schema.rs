// @generated automatically by Diesel CLI.

diesel::table! {
    absences (id) {
        id -> Int4,
        lev -> Int8,
        similiarity_score -> Int8,
        #[max_length = 255]
        searched_with -> Nullable<Varchar>,
        #[max_length = 255]
        matched_with -> Varchar,
        delegate_id -> Int4,
        manually_matched -> Nullable<Bool>,
        plenary_session_id -> Int4,
    }
}

diesel::table! {
    answers (id) {
        id -> Int4,
        question_id -> Int4,
        delegate_id -> Nullable<Int4>,
        via_mail -> Nullable<Bool>,
        received_at -> Timestamp,
        #[max_length = 10]
        text_type -> Nullable<Varchar>,
        #[max_length = 1024]
        mail_subject -> Nullable<Varchar>,
        body -> Text,
    }
}

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
        #[max_length = 1]
        gender -> Nullable<Varchar>,
        is_active -> Nullable<Bool>,
        birthdate -> Nullable<Date>,
    }
}

diesel::table! {
    delegates_divisions (id) {
        id -> Int4,
        delegate_id -> Int4,
        #[max_length = 255]
        division -> Varchar,
        insertion_date -> Nullable<Date>,
    }
}

diesel::table! {
    division_interest_score (id) {
        id -> Int4,
        timestamp -> Nullable<Timestamp>,
        score -> Float4,
        delegate_id -> Int4,
    }
}

diesel::table! {
    eurovoc_topics_legis_init (id) {
        id -> Int4,
        #[max_length = 255]
        topic -> Varchar,
        legislative_initiatives_id -> Int4,
    }
}

diesel::table! {
    eurovoc_topics_ministrial_proposals (id) {
        id -> Int4,
        #[max_length = 255]
        topic -> Varchar,
        ministrial_proposal_id -> Int4,
    }
}

diesel::table! {
    eurovoc_topics_proposals (id) {
        id -> Int4,
        #[max_length = 255]
        topic -> Varchar,
        #[max_length = 20]
        proposals_id -> Varchar,
    }
}

diesel::table! {
    favo_dels (id) {
        id -> Int4,
        user_id -> Int4,
        delegate_id -> Int4,
        favo_on -> Nullable<Timestamp>,
    }
}

diesel::table! {
    favo_legis_inits (id) {
        id -> Int4,
        user_id -> Int4,
        legis_init_id -> Int4,
        favo_on -> Nullable<Timestamp>,
    }
}

diesel::table! {
    introduction_transcriptions (id) {
        id -> Int4,
        delegate_id -> Nullable<Int4>,
    }
}

diesel::table! {
    legis_email_info (id) {
        id -> Int4,
        already_sent_id -> Int4,
        marked_on -> Nullable<Timestamp>,
    }
}

diesel::table! {
    legis_init_delegates (id) {
        id -> Int4,
        legis_init_id -> Int4,
        delegate_id -> Int4,
    }
}

diesel::table! {
    legis_inits_refs (id) {
        id -> Int4,
        origin_legis_init_id -> Int4,
        #[max_length = 255]
        ref_gp -> Varchar,
        #[max_length = 255]
        ref_ityp -> Varchar,
        ref_inr -> Int4,
    }
}

diesel::table! {
    legislative_documents (id) {
        id -> Int4,
        legislative_initiatives_id -> Int4,
        #[max_length = 1024]
        title -> Nullable<Varchar>,
        #[max_length = 100]
        document_type -> Varchar,
        #[max_length = 500]
        document_url -> Varchar,
    }
}

diesel::table! {
    legislative_initiatives (id) {
        id -> Int4,
        #[max_length = 7]
        ityp -> Varchar,
        #[max_length = 20]
        doktyp -> Nullable<Varchar>,
        #[max_length = 7]
        gp -> Varchar,
        inr -> Int4,
        #[max_length = 6000]
        emphasis -> Nullable<Varchar>,
        is_emphasis_ai_generated -> Nullable<Bool>,
        title -> Text,
        #[max_length = 10]
        pre_declined_type -> Nullable<Varchar>,
        voted_by_name -> Bool,
        description -> Text,
        #[max_length = 10]
        accepted -> Nullable<Varchar>,
        created_at -> Date,
        appeared_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        requires_simple_majority -> Nullable<Bool>,
        plenary_session_id -> Nullable<Int4>,
        notified_with_mail -> Nullable<Bool>,
        is_law -> Nullable<Bool>,
        law_accepted -> Nullable<Bool>,
        law_come_into_effect_date -> Nullable<Date>,
        law_expires_on_date -> Nullable<Date>,
        by_publication -> Nullable<Bool>,
        has_reference -> Nullable<Bool>,
    }
}

diesel::table! {
    mandates (id) {
        id -> Int4,
        delegate_id -> Int4,
        is_nr -> Nullable<Bool>,
        is_gov_official -> Nullable<Bool>,
        #[max_length = 255]
        function -> Nullable<Varchar>,
        is_ministry -> Nullable<Bool>,
        is_chancellor -> Nullable<Bool>,
        #[max_length = 312]
        ministry -> Nullable<Varchar>,
        #[max_length = 312]
        name -> Varchar,
        #[max_length = 100]
        party -> Nullable<Varchar>,
        start_date -> Date,
        end_date -> Nullable<Date>,
    }
}

diesel::table! {
    ministrial_email_info (id) {
        id -> Int4,
        already_sent_id -> Int4,
        marked_on -> Nullable<Timestamp>,
    }
}

diesel::table! {
    ministrial_issuer (id) {
        id -> Int4,
        delegate_id -> Int4,
        ministrial_proposal_id -> Int4,
    }
}

diesel::table! {
    ministrial_proposals (id) {
        id -> Int4,
        #[max_length = 7]
        ityp -> Varchar,
        #[max_length = 14]
        gp -> Varchar,
        inr -> Int4,
        #[max_length = 6000]
        emphasis -> Nullable<Varchar>,
        title -> Text,
        description -> Text,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
        due_to -> Date,
        #[max_length = 255]
        ressort -> Nullable<Varchar>,
        #[max_length = 12]
        ressort_shortform -> Nullable<Varchar>,
        #[max_length = 14]
        legis_init_gp -> Nullable<Varchar>,
        legis_init_inr -> Nullable<Int4>,
        #[max_length = 7]
        legis_init_ityp -> Nullable<Varchar>,
        has_vote_result -> Nullable<Bool>,
    }
}

diesel::table! {
    ministrial_proposals_documents (id) {
        id -> Int4,
        ministrial_proposal_id -> Int4,
        #[max_length = 1024]
        title -> Nullable<Varchar>,
        #[max_length = 100]
        document_type -> Varchar,
        #[max_length = 500]
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
        #[max_length = 255]
        searched_with -> Nullable<Varchar>,
        #[max_length = 255]
        matched_with -> Varchar,
        delegate_id -> Int4,
        named_vote_info_id -> Int4,
        manually_matched -> Nullable<Bool>,
    }
}

diesel::table! {
    other_keyword_topics_legis_init (id) {
        id -> Int4,
        #[max_length = 255]
        topic -> Varchar,
        legislative_initiatives_id -> Int4,
    }
}

diesel::table! {
    other_keyword_topics_ministrial_proposals (id) {
        id -> Int4,
        #[max_length = 255]
        topic -> Varchar,
        ministrial_proposal_id -> Int4,
    }
}

diesel::table! {
    other_keyword_topics_proposals (id) {
        id -> Int4,
        #[max_length = 255]
        topic -> Varchar,
        #[max_length = 20]
        proposals_id -> Varchar,
    }
}

diesel::table! {
    parties (id) {
        id -> Int4,
        #[max_length = 8]
        name -> Varchar,
        #[max_length = 14]
        gp -> Varchar,
        #[max_length = 8]
        code -> Varchar,
        fraction -> Int4,
        #[max_length = 7]
        color -> Varchar,
    }
}

diesel::table! {
    plenar_infos (id) {
        id -> Int4,
        add_date -> Timestamp,
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
    plenar_speech_legis_inits (id) {
        id -> Int4,
        #[max_length = 255]
        gp -> Varchar,
        #[max_length = 255]
        ityp -> Varchar,
        inr -> Int4,
        speech_id -> Int4,
        legis_init_id -> Nullable<Int4>,
    }
}

diesel::table! {
    plenar_speech_links (id) {
        id -> Int4,
        plenar_speech_id -> Int4,
        site_start -> Nullable<Int4>,
        site_end -> Nullable<Int4>,
        #[max_length = 500]
        document_url -> Nullable<Varchar>,
    }
}

diesel::table! {
    plenar_speeches (id) {
        id -> Int4,
        delegate_id -> Int4,
        debate_id -> Int4,
        #[max_length = 255]
        opinion -> Nullable<Varchar>,
        spoken_text -> Nullable<Text>,
        summarized_spoken_text -> Nullable<Text>,
        duration_in_seconds -> Int4,
        #[max_length = 40]
        ty -> Varchar,
        #[max_length = 255]
        delegate_display_name -> Varchar,
    }
}

diesel::table! {
    political_answers (id) {
        id -> Int4,
        question_id -> Int4,
        delegate_id -> Nullable<Int4>,
        answer -> Text,
        #[max_length = 255]
        stance_llm -> Varchar,
        is_strong_reference -> Nullable<Bool>,
        #[max_length = 255]
        model_used -> Nullable<Varchar>,
    }
}

diesel::table! {
    political_opinions (id) {
        id -> Int4,
        delegate_id -> Int4,
        question_id -> Int4,
        answer_id -> Int4,
        #[max_length = 255]
        stance -> Varchar,
        stance_acc -> Float8,
        pro_strong_ref_score -> Float8,
        contra_strong_ref_score -> Float8,
        ref_score -> Float8,
    }
}

diesel::table! {
    political_positions (id) {
        id -> Int4,
        delegate_id -> Int4,
        is_left -> Float8,
        is_not_left -> Float8,
        is_liberal -> Float8,
        is_not_liberal -> Float8,
        neutral_count -> Int4,
    }
}

diesel::table! {
    political_questions (id) {
        id -> Int4,
        question -> Text,
        is_left -> Nullable<Bool>,
        is_liberal -> Nullable<Bool>,
    }
}

diesel::table! {
    political_questions_detailed_topics (id) {
        id -> Int4,
        question_id -> Int4,
        #[max_length = 255]
        topic -> Varchar,
    }
}

diesel::table! {
    political_questions_detailed_topics_influence (id) {
        id -> Int4,
        question_id -> Int4,
        #[max_length = 255]
        topic -> Varchar,
        influence -> Float8,
    }
}

diesel::table! {
    political_questions_topics (id) {
        id -> Int4,
        question_id -> Int4,
        #[max_length = 255]
        topic -> Varchar,
    }
}

diesel::table! {
    political_questions_topics_influence (id) {
        id -> Int4,
        question_id -> Int4,
        #[max_length = 255]
        topic -> Varchar,
        influence -> Float8,
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
    proposal_documents (id) {
        id -> Int4,
        #[max_length = 20]
        proposal_id -> Varchar,
        #[max_length = 1024]
        title -> Nullable<Varchar>,
        #[max_length = 100]
        document_type -> Varchar,
        #[max_length = 500]
        document_url -> Varchar,
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
        #[max_length = 1024]
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
    quiz (id) {
        id -> Int4,
        #[max_length = 255]
        title -> Varchar,
        #[max_length = 2048]
        description -> Nullable<Varchar>,
        stars -> Nullable<Int4>,
    }
}

diesel::table! {
    quiz_questions (id) {
        id -> Int4,
        quiz_id -> Nullable<Int4>,
        #[max_length = 2048]
        question -> Varchar,
        #[max_length = 255]
        answer1 -> Varchar,
        #[max_length = 255]
        answer2 -> Varchar,
        #[max_length = 255]
        answer3 -> Varchar,
        #[max_length = 255]
        answer4 -> Varchar,
        correct_answer -> Int4,
    }
}

diesel::table! {
    seat_history (id) {
        id -> Int4,
        delegate_id -> Nullable<Int4>,
        seat_col -> Int4,
        seat_row -> Int4,
        #[max_length = 255]
        council -> Varchar,
        #[max_length = 255]
        gp -> Varchar,
        insertion_date -> Nullable<Date>,
    }
}

diesel::table! {
    somes_user (id) {
        id -> Int4,
        #[max_length = 356]
        email -> Varchar,
        is_email_hashed -> Bool,
        is_admin -> Bool,
        send_new_vote_results_mails -> Bool,
        send_new_delegate_activity_mails -> Bool,
        send_new_ministrial_prop_mails -> Bool,
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
        #[max_length = 255]
        opinion -> Nullable<Varchar>,
        legislative_initiatives_id -> Int4,
    }
}

diesel::table! {
    speeches_html_urls (id) {
        id -> Int4,
        speech_id -> Int4,
        #[max_length = 500]
        document_url -> Varchar,
    }
}

diesel::table! {
    topics_legis_init (id) {
        id -> Int4,
        #[max_length = 100]
        topic -> Varchar,
        legislative_initiatives_id -> Int4,
    }
}

diesel::table! {
    topics_ministrial_proposals (id) {
        id -> Int4,
        #[max_length = 255]
        topic -> Varchar,
        ministrial_proposal_id -> Int4,
    }
}

diesel::table! {
    topics_proposals (id) {
        id -> Int4,
        #[max_length = 255]
        topic -> Varchar,
        #[max_length = 20]
        proposals_id -> Varchar,
    }
}

diesel::table! {
    transcriptionsqa (id) {
        id -> Int4,
        question -> Text,
        answer -> Text,
        transcription_id -> Int4,
    }
}

diesel::table! {
    unique_topics (id) {
        id -> Int4,
        #[max_length = 255]
        topic_name -> Varchar,
    }
}

diesel::table! {
    user_topics (id) {
        id -> Int4,
        user_id -> Int4,
        topic_id -> Int4,
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

diesel::table! {
    walo (id) {
        id -> Int4,
        question_statement -> Nullable<Text>,
        new_keywords_topics -> Nullable<Text>,
        spoe_justification -> Nullable<Text>,
        gruene_justification -> Nullable<Text>,
        neos_justification -> Nullable<Text>,
        fpoe_justification -> Nullable<Text>,
        oevp_justification -> Nullable<Text>,
        somes_link -> Nullable<Text>,
        law_link -> Nullable<Text>,
        erklaerbaer -> Nullable<Text>,
    }
}

diesel::joinable!(absences -> delegates (delegate_id));
diesel::joinable!(absences -> plenar_infos (plenary_session_id));
diesel::joinable!(answers -> questions (question_id));
diesel::joinable!(call_to_order -> delegates (receiver_id));
diesel::joinable!(call_to_order -> plenar_infos (plenar_id));
diesel::joinable!(debates -> plenar_infos (plenar_id));
diesel::joinable!(delegates -> contacts (id));
diesel::joinable!(delegates_divisions -> delegates (delegate_id));
diesel::joinable!(division_interest_score -> delegates (delegate_id));
diesel::joinable!(eurovoc_topics_legis_init -> legislative_initiatives (legislative_initiatives_id));
diesel::joinable!(eurovoc_topics_ministrial_proposals -> ministrial_proposals (ministrial_proposal_id));
diesel::joinable!(eurovoc_topics_proposals -> proposals (proposals_id));
diesel::joinable!(favo_dels -> delegates (delegate_id));
diesel::joinable!(favo_dels -> somes_user (user_id));
diesel::joinable!(favo_legis_inits -> legislative_initiatives (legis_init_id));
diesel::joinable!(favo_legis_inits -> somes_user (user_id));
diesel::joinable!(introduction_transcriptions -> delegates (delegate_id));
diesel::joinable!(legis_email_info -> legislative_initiatives (already_sent_id));
diesel::joinable!(legis_init_delegates -> delegates (delegate_id));
diesel::joinable!(legis_init_delegates -> legislative_initiatives (legis_init_id));
diesel::joinable!(legis_inits_refs -> legislative_initiatives (origin_legis_init_id));
diesel::joinable!(legislative_documents -> legislative_initiatives (legislative_initiatives_id));
diesel::joinable!(legislative_initiatives -> plenar_infos (plenary_session_id));
diesel::joinable!(mandates -> delegates (delegate_id));
diesel::joinable!(ministrial_email_info -> ministrial_proposals (already_sent_id));
diesel::joinable!(ministrial_issuer -> delegates (delegate_id));
diesel::joinable!(ministrial_issuer -> ministrial_proposals (ministrial_proposal_id));
diesel::joinable!(ministrial_proposals_documents -> ministrial_proposals (ministrial_proposal_id));
diesel::joinable!(named_vote_info -> legislative_initiatives (legis_init_id));
diesel::joinable!(named_votes -> delegates (delegate_id));
diesel::joinable!(named_votes -> named_vote_info (named_vote_info_id));
diesel::joinable!(other_keyword_topics_legis_init -> legislative_initiatives (legislative_initiatives_id));
diesel::joinable!(other_keyword_topics_ministrial_proposals -> ministrial_proposals (ministrial_proposal_id));
diesel::joinable!(other_keyword_topics_proposals -> proposals (proposals_id));
diesel::joinable!(plenar_speech_legis_inits -> legislative_initiatives (legis_init_id));
diesel::joinable!(plenar_speech_legis_inits -> plenar_speeches (speech_id));
diesel::joinable!(plenar_speech_links -> plenar_speeches (plenar_speech_id));
diesel::joinable!(plenar_speeches -> debates (debate_id));
diesel::joinable!(plenar_speeches -> delegates (delegate_id));
diesel::joinable!(political_answers -> delegates (delegate_id));
diesel::joinable!(political_answers -> political_questions (question_id));
diesel::joinable!(political_opinions -> delegates (delegate_id));
diesel::joinable!(political_opinions -> political_answers (answer_id));
diesel::joinable!(political_opinions -> political_questions (question_id));
diesel::joinable!(political_positions -> delegates (delegate_id));
diesel::joinable!(political_questions_detailed_topics -> political_questions (question_id));
diesel::joinable!(political_questions_detailed_topics_influence -> political_questions (question_id));
diesel::joinable!(political_questions_topics -> political_questions (question_id));
diesel::joinable!(political_questions_topics_influence -> political_questions (question_id));
diesel::joinable!(proposal_delegates -> delegates (delegate_id));
diesel::joinable!(proposal_delegates -> proposals (proposal_id));
diesel::joinable!(proposal_documents -> proposals (proposal_id));
diesel::joinable!(questions -> delegates (delegate_id));
diesel::joinable!(questions -> users (issuer_id));
diesel::joinable!(quiz_questions -> quiz (quiz_id));
diesel::joinable!(seat_history -> delegates (delegate_id));
diesel::joinable!(speech_complexity -> plenar_speeches (speech_id));
diesel::joinable!(speeches -> delegates (delegate_id));
diesel::joinable!(speeches -> legislative_initiatives (legislative_initiatives_id));
diesel::joinable!(speeches_html_urls -> speeches (speech_id));
diesel::joinable!(topics_legis_init -> legislative_initiatives (legislative_initiatives_id));
diesel::joinable!(topics_ministrial_proposals -> ministrial_proposals (ministrial_proposal_id));
diesel::joinable!(topics_proposals -> proposals (proposals_id));
diesel::joinable!(transcriptionsqa -> introduction_transcriptions (transcription_id));
diesel::joinable!(user_topics -> somes_user (user_id));
diesel::joinable!(user_topics -> unique_topics (topic_id));
diesel::joinable!(votes -> legislative_initiatives (legislative_initiatives_id));

diesel::allow_tables_to_appear_in_same_query!(
    absences,
    answers,
    call_to_order,
    contacts,
    debates,
    delegates,
    delegates_divisions,
    division_interest_score,
    eurovoc_topics_legis_init,
    eurovoc_topics_ministrial_proposals,
    eurovoc_topics_proposals,
    favo_dels,
    favo_legis_inits,
    introduction_transcriptions,
    legis_email_info,
    legis_init_delegates,
    legis_inits_refs,
    legislative_documents,
    legislative_initiatives,
    mandates,
    ministrial_email_info,
    ministrial_issuer,
    ministrial_proposals,
    ministrial_proposals_documents,
    named_vote_info,
    named_votes,
    other_keyword_topics_legis_init,
    other_keyword_topics_ministrial_proposals,
    other_keyword_topics_proposals,
    parties,
    plenar_infos,
    plenar_speech_legis_inits,
    plenar_speech_links,
    plenar_speeches,
    political_answers,
    political_opinions,
    political_positions,
    political_questions,
    political_questions_detailed_topics,
    political_questions_detailed_topics_influence,
    political_questions_topics,
    political_questions_topics_influence,
    proposal_delegates,
    proposal_documents,
    proposals,
    questions,
    quiz,
    quiz_questions,
    seat_history,
    somes_user,
    speech_complexity,
    speeches,
    speeches_html_urls,
    topics_legis_init,
    topics_ministrial_proposals,
    topics_proposals,
    transcriptionsqa,
    unique_topics,
    user_topics,
    users,
    votes,
    walo,
);
