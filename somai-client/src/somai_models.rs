use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LLMRequest {
    pub input: String,
    pub system: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub use_openai: bool,
    pub json_schema: Option<serde_json::Value>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct LLMResponse {
    pub result: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SummaryRequest {
    pub input: String,
    #[serde(default)]
    pub model: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct SummaryFromUrlRequest {
    pub url: String,
    #[serde(default)]
    pub model: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct SummaryResponse {
    pub summary: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopicsRequest {
    pub input: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TopicsFromUrlRequest {
    pub url: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TopicsResponse {
    pub topics: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QAItem {
    pub question: String,
    pub answer: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct QAResponse {
    pub qa: Vec<QAItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimilarityResponse {
    pub similarity_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpeechEmbeddingAddRequest {
    pub delegate_id: i32,
    pub speech: String,
    pub speech_id: Option<i32>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct SpeechEmbeddingUpdateRequest {
    pub delegate_id: i32,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct GenericStatusResponse {
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub input: String,
    pub model: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub embedding: Vec<f64>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct EmbedAndCosineRequest {
    pub lhs: String,
    pub rhs: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CosineResponse {
    pub cosine_similarity: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RerankingRequest {
    pub input: String,
    pub to_rerank: Vec<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub top_n: Option<usize>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct RerankedDocument {
    pub index: usize,
    pub text: String,
    pub score: f64,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct RerankingResponse {
    pub model: String,
    pub results: Vec<RerankedDocument>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PoliticalStanceRequest {
    pub delegate_id: i32,
    pub question_or_input: String,
    #[serde(default)]
    pub model: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct PoliticalStanceResponse {
    pub stance: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
}
