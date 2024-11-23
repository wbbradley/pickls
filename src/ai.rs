use crate::prelude::*;
use allms::{llm_models::OpenAIModels, Completions};
use schemars::JsonSchema;

#[derive(Debug, Serialize)]
pub struct InlineAssistTemplateContext {
    pub language_id: String,
    pub text: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InlineAssistResponse {
    pub code: String,
}

pub async fn fetch_completion<T: JsonSchema + DeserializeOwned>(
    api_key: String,
    instructions: String,
) -> Result<T> {
    Completions::new(OpenAIModels::Gpt4o, &api_key, None, None)
        .debug()
        .get_answer::<T>(&instructions)
        .await
        .context("Failed to get answer")
    // Err(Error::new("Failed to get answer"))
}
