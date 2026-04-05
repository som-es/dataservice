mod delegate_context;
mod embed_speeches;
mod stance_system;
mod upload_questions;

use somai_client::SomAiClient;

use crate::Result;

pub(super) trait EmbeddingProvider: Send + Sync {
    /// Generates an embedding for a given chunk of text
    async fn embed_text(&self, text: &str) -> Result<Vec<f64>>;
    /// Returns the dimension of the embeddings produced
    async fn embedding_dim(&self) -> Result<u64>;
}

impl EmbeddingProvider for SomAiClient {
    async fn embed_text(&self, text: &str) -> Result<Vec<f64>> {
        Ok(self
            .embedding_embed(text.into(), None)
            .await
            .map(|out| out.embedding)?)
    }

    async fn embedding_dim(&self) -> Result<u64> {
        Ok(self.embed_text("This is a test").await?.len() as u64)
    }
}
