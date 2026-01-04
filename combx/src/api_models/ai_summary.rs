use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use somes_common_lib::ToCompositeType;
use somes_macro::CompositeType;
use sqlx::{FromRow, Type};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, Type)]
#[sqlx(type_name = "db_ai_summary")]
pub struct DbAiSummary {
    pub id: i32,
    pub full_summary: sqlx::types::Json<AiSummary>,
    pub short_title: String,
    pub short_summary: String,
    pub detailed_summary: String,
    pub complexity_scope_of_proposal: String,
    pub model_used: String,
    pub version: String,
    pub generated_at: DateTime<Utc>,
}

impl ToCompositeType for DbAiSummary {
    fn field_orders() -> Vec<&'static str> {
        vec![
            "id",
            "full_summary",
            "short_title",
            "short_summary",
            "detailed_summary",
            "complexity_scope_of_proposal",
            "model_used",
            "version",
            "generated_at",
        ]
    }
    fn type_name() -> &'static str {
        "db_ai_summary"
    }

    fn to_sql_create_composite_type() -> String {
        "CREATE TYPE db_ai_summary AS (
            id INTEGER,
            full_summary JSONB,
            short_title VARCHAR,
            short_summary VARCHAR,
            detailed_summary VARCHAR,
            complexity_scope_of_proposal VARCHAR,
            model_used VARCHAR,
            version VARCHAR,
            generated_at TIMESTAMPTZ
        );"
        .to_string()
    }
}

#[cfg(feature = "utoipa")]
impl utoipa::PartialSchema for DbAiSummary {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        use utoipa::openapi::schema::*;

        ObjectBuilder::new()
            .property(
                "id",
                Schema::Object(
                    ObjectBuilder::new()
                        .schema_type(SchemaType::Type(Type::Integer))
                        .format(Some(SchemaFormat::KnownFormat(KnownFormat::Int32)))
                        .build(),
                ),
            )
            .property("full_summary", AiSummary::schema())
            .property(
                "short_title",
                Schema::Object(
                    ObjectBuilder::new()
                        .schema_type(SchemaType::Type(Type::String))
                        .build(),
                ),
            )
            .property(
                "detailed_summary",
                Schema::Object(
                    ObjectBuilder::new()
                        .schema_type(SchemaType::Type(Type::String))
                        .build(),
                ),
            )
            .property(
                "complexity_scope_of_proposal",
                Schema::Object(
                    ObjectBuilder::new()
                        .schema_type(SchemaType::Type(Type::String))
                        .build(),
                ),
            )
            .property(
                "model_used",
                Schema::Object(
                    ObjectBuilder::new()
                        .schema_type(SchemaType::Type(Type::String))
                        .build(),
                ),
            )
            .property(
                "version",
                Schema::Object(
                    ObjectBuilder::new()
                        .schema_type(SchemaType::Type(Type::String))
                        .build(),
                ),
            )
            .property(
                "generated_at",
                Schema::Object(
                    ObjectBuilder::new()
                        .schema_type(SchemaType::Type(Type::String))
                        .format(Some(SchemaFormat::KnownFormat(KnownFormat::DateTime)))
                        .build(),
                ),
            )
            .required("id")
            .required("full_summary")
            .required("short_title")
            .required("detailed_summary")
            .required("complexity_scope_of_proposal")
            .required("model_used")
            .required("version")
            .required("generated_at")
            .build()
            .into()
    }
}

#[cfg(feature = "utoipa")]
impl utoipa::ToSchema for DbAiSummary {}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, Type, CompositeType,
)]
pub struct EnforcementDates {
    pub enforcement_start_date: String,
    pub start_notes: Option<String>,
    pub enforcement_end_date: Option<String>,
    pub end_notes: Option<String>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(JsonSchema, Debug, PartialEq, Deserialize, Serialize, Clone, FromRow, CompositeType)]
pub struct Keypoint {
    pub point: String,
    pub enforcement_start_end: Option<EnforcementDates>,
    pub paragraph_references: Vec<EuroLawReference>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(JsonSchema, Debug, PartialEq, Deserialize, Serialize, Clone, Type)]
pub enum ProposalComplexityScope {
    Highest,
    High,
    Medium,
    Low,
    Lowest,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(JsonSchema, Debug, Deserialize, PartialEq, Serialize, Clone, Type)]
pub enum Tone {
    Neutral,
    Urgent,
    Aggressive,
    Optimistic,
    Bureaucratic,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, Type, CompositeType,
)]
pub struct CriticalAnalysis {
    pub arguments_for: Vec<String>,
    pub arguments_against: Vec<String>,
    pub tone: Tone,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, Type, CompositeType,
)]
pub struct TermDefinition {
    pub term: String,
    pub simple_definition: String,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, Type, CompositeType,
)]
pub struct Glossary {
    pub difficult_terms: Vec<TermDefinition>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(
    JsonSchema, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, Type, CompositeType,
)]
pub struct FiscalAnalysis {
    pub estimated_cost_per_year_in_million: Option<f64>,
    pub estimated_cost_per_month_in_million: Option<f64>,
    pub economic_burden: EconomicImpactLevel,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(JsonSchema, Debug, Deserialize, PartialEq, Serialize, Clone, Type)]
pub enum EconomicImpactLevel {
    HighCost,
    ModerateCost,
    LowCost,
    Neutral,
    RevenueGenerating,
    Unclear,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(JsonSchema, Debug, PartialEq, Deserialize, Serialize, Clone, FromRow, CompositeType)]
pub struct AiSummary {
    pub short_title: String,
    pub short_summary: String,
    pub detailed_summary: String,
    pub very_detailed_summary: String,
    pub key_points: Vec<Keypoint>,
    pub general_enforcement_start_end: Option<EnforcementDates>,
    pub topics: Vec<String>,
    pub general_political_questions: Vec<String>,
    pub political_compass_questions: Vec<String>,
    pub complexity_scope_of_proposal: ProposalComplexityScope,
    pub critical_analysis: CriticalAnalysis,
    pub glossary: Glossary,
    pub fiscal_analysis: FiscalAnalysis,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(JsonSchema, Debug, Deserialize, PartialEq, Serialize, Clone)]
#[serde(tag = "kind", content = "value")]
pub enum LawPart {
    Article(u32),
    Paragraph(u32),
    Literal(char),
    Subsection(u32),
    Point(String),
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(JsonSchema, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, CompositeType)]
pub struct EuroLawReference {
    pub full_unparsed_reference: String,
    pub parsed_reference_parts: Vec<LawPart>,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(JsonSchema, Debug, Deserialize, PartialEq, Serialize, Clone, FromRow, CompositeType)]
pub struct SummarizeOutput {
    pub summary: Option<AiSummary>,
    pub abort_reason: AbortReason,
}

#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[derive(JsonSchema, Debug, Deserialize, PartialEq, Serialize, Clone, Type)]
pub enum AbortReason {
    NoDocumentsProvidedInContext,
    None,
}
