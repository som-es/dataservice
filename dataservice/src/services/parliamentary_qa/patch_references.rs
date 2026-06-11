use serde_json::{json, Value};
use std::collections::HashSet;

pub fn patch_reference_possibility(schema: &mut Value, questions: &[combx::Question]) {
    let mut seen = HashSet::new();

    let allowed_question_refs = questions
        .iter()
        .filter_map(|question| {
            if question.nth_level.is_empty() {
                None
            } else if seen.insert(question.nth_level.clone()) {
                Some(json!(question.nth_level))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let patched_schema = if allowed_question_refs.is_empty() {
        json!({
            "type": "array",
            "minItems": 1,
            "maxItems": 0
        })
    } else {
        json!({
            "type": "array",
            "enum": allowed_question_refs
        })
    };

    for pointer in [
        "/properties/answers/items/properties/answering_questions_references/items/properties/affected_question",
        "/$defs/QuestionRef/properties/affected_question",
        "/definitions/QuestionRef/properties/affected_question",
    ] {
        if let Some(affected_question_schema) = schema.pointer_mut(pointer) {
            *affected_question_schema = patched_schema.clone();
        }
    }
}
