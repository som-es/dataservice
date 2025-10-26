use std::time::{Duration, Instant};

use reqwest::Client;
use tokio::sync::RwLock;

use crate::somai_models::*;

pub struct SomAiClient {
    base_url: String,
    username: String,
    password: String,
    token: RwLock<String>,
    token_created: RwLock<Instant>,
    client: Client,
}

impl SomAiClient {
    pub async fn new(
        base_url: &str,
        username: &str,
        password: &str,
    ) -> Result<Self, reqwest::Error> {
        let client = Client::new();
        let somai = Self {
            base_url: base_url.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            token: RwLock::new(String::new()),
            token_created: RwLock::new(Instant::now() - Duration::from_secs(3600)),
            client,
        };
        somai.relogin().await?;
        Ok(somai)
    }

    pub async fn new_local(base_url: &str) -> Result<Self, reqwest::Error> {
        let client = Client::new();
        let somai = Self {
            base_url: base_url.to_string(),
            username: "".into(),
            password: "".into(),
            token: RwLock::new(String::new()),
            token_created: RwLock::new(Instant::now() - Duration::from_secs(3600)),
            client,
        };
        somai.local_login().await?;
        Ok(somai)
    }

    pub async fn local_login(&self) -> Result<(), reqwest::Error> {
        let mut token_guard = self.token.write().await;
        let mut token_created_guard = self.token_created.write().await;
        if token_guard.is_empty() || token_created_guard.elapsed() >= Duration::from_secs(3600) {
            let resp = self
                .client
                .post(format!("{}/api/local_login", self.base_url))
                .send()
                .await?;
            let login_resp: LoginResponse = resp.json().await?;
            *token_guard = login_resp.access_token;
            *token_created_guard = Instant::now();
        }
        Ok(())
    }

    pub async fn relogin(&self) -> Result<(), reqwest::Error> {
        let mut token_guard = self.token.write().await;
        let mut token_created_guard = self.token_created.write().await;
        if token_guard.is_empty() || token_created_guard.elapsed() >= Duration::from_secs(3600) {
            let login_req = LoginRequest {
                username: self.username.clone(),
                password: self.password.clone(),
            };
            let resp = self
                .client
                .post(format!("{}/api/login", self.base_url))
                .json(&login_req)
                .send()
                .await?;
            let login_resp: LoginResponse = resp.json().await?;
            *token_guard = login_resp.access_token;
            *token_created_guard = Instant::now();
        }
        Ok(())
    }

    async fn auth_header(&self) -> String {
        self.relogin().await.unwrap();
        let token = self.token.read().await;
        format!("Bearer {}", token.as_str())
    }

    // ===================== ENDPOINTS =====================

    pub async fn make_llm_request(
        &self,
        input: String,
        prompt: String,
        model: String,
        use_openai: bool,
    ) -> Result<LLMResponse, reqwest::Error> {
        let req = LLMRequest {
            input,
            prompt,
            model,
            use_openai,
        };
        let resp = self
            .client
            .post(format!("{}/api/make_llm_request", self.base_url))
            .header("Authorization", self.auth_header().await)
            .json(&req)
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    pub async fn summarize_input(
        &self,
        input: String,
        model: String,
    ) -> Result<SummaryResponse, reqwest::Error> {
        let req = SummaryRequest { input, model };
        let resp = self
            .client
            .post(format!("{}/api/summarize_input", self.base_url))
            .header("Authorization", self.auth_header().await)
            .json(&req)
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    pub async fn summarize_input_from_url(
        &self,
        url: String,
        model: String,
    ) -> Result<SummaryResponse, reqwest::Error> {
        let req = SummaryFromUrlRequest { url, model };
        let resp = self
            .client
            .post(format!("{}/api/summarize_input_from_url", self.base_url))
            .header("Authorization", self.auth_header().await)
            .json(&req)
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    pub async fn make_topics_predictions(
        &self,
        input: String,
    ) -> Result<TopicsResponse, reqwest::Error> {
        let req = TopicsRequest { input };
        let resp = self
            .client
            .post(format!("{}/api/make_topics_predictions", self.base_url))
            .header("Authorization", self.auth_header().await)
            .json(&req)
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    pub async fn make_topics_predictions_from_url(
        &self,
        url: String,
    ) -> Result<TopicsResponse, reqwest::Error> {
        let req = TopicsFromUrlRequest { url };
        let resp = self
            .client
            .post(format!(
                "{}/api/make_topics_predictions_from_url",
                self.base_url
            ))
            .header("Authorization", self.auth_header().await)
            .json(&req)
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    pub async fn extract_delegate_preview_video_qa(
        &self,
        delegate_id: i32,
    ) -> Result<QAResponse, reqwest::Error> {
        let resp = self
            .client
            .get(format!(
                "{}/api/extract_delegate_preview_video_qa?delegate_id={}",
                self.base_url, delegate_id
            ))
            .header("Authorization", self.auth_header().await)
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    pub async fn compute_delegate_interests_division_similarity(
        &self,
        delegate_id: i32,
    ) -> Result<SimilarityResponse, reqwest::Error> {
        let resp = self
            .client
            .get(format!(
                "{}/api/compute_delegate_interests_division_similarity?delegate_id={}",
                self.base_url, delegate_id
            ))
            .header("Authorization", self.auth_header().await)
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    pub async fn speech_embedding_add(
        &self,
        delegate_id: i32,
        speech: String,
        speech_id: Option<i32>,
    ) -> Result<GenericStatusResponse, reqwest::Error> {
        let req = SpeechEmbeddingAddRequest {
            delegate_id,
            speech,
            speech_id,
        };
        let resp = self
            .client
            .post(format!("{}/api/speech_embedding/add", self.base_url))
            .header("Authorization", self.auth_header().await)
            .json(&req)
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    pub async fn speech_embedding_update_all(
        &self,
        delegate_id: i32,
    ) -> Result<GenericStatusResponse, reqwest::Error> {
        let req = SpeechEmbeddingUpdateRequest { delegate_id };
        let resp = self
            .client
            .post(format!("{}/api/speech_embedding/update_all", self.base_url))
            .header("Authorization", self.auth_header().await)
            .json(&req)
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    pub async fn embedding_embed(
        &self,
        input: String,
    ) -> Result<EmbeddingResponse, reqwest::Error> {
        let req = EmbeddingRequest { input };
        let resp = self
            .client
            .post(format!("{}/api/embedding/embed", self.base_url))
            .header("Authorization", self.auth_header().await)
            .json(&req)
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    pub async fn embedding_embed_and_cosine(
        &self,
        lhs: String,
        rhs: String,
    ) -> Result<CosineResponse, reqwest::Error> {
        let req = EmbedAndCosineRequest { lhs, rhs };
        let resp = self
            .client
            .post(format!("{}/api/embedding/embed_and_cosine", self.base_url))
            .header("Authorization", self.auth_header().await)
            .json(&req)
            .send()
            .await?;
        Ok(resp.json().await?)
    }

    pub async fn political_stance_generate(
        &self,
        delegate_id: i32,
        question_or_input: String,
        model: String,
    ) -> Result<PoliticalStanceResponse, reqwest::Error> {
        let req = PoliticalStanceRequest {
            delegate_id,
            question_or_input,
            model,
        };
        let resp = self
            .client
            .post(format!("{}/api/political_stance/generate", self.base_url))
            .header("Authorization", self.auth_header().await)
            .json(&req)
            .send()
            .await?;
        Ok(resp.json().await?)
    }
}

#[cfg(test)]
mod tests {
    use crate::SomAiClient;

    #[tokio::test]
    async fn test_somai_client() {
        let client = SomAiClient::new_local("http://127.0.0.1:8000")
            .await
            .unwrap();
        let topics = client
            .make_topics_predictions("Einen langen Antragstext, über das Thema Naturschutz:".into())
            .await
            .unwrap();
        dbg!(topics);
    }
}
