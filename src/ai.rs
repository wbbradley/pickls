use crate::prelude::*;
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

pub async fn fetch_completion(
    //<T: JsonSchema + DeserializeOwned>(
    api_key: String,
    model: String,
    instructions: String,
) -> Result<serde_json::Value> {
    log::info!(
        "fetching completion with {} of {}",
        &api_key[0..4],
        &instructions[0..10]
    );
    let client = reqwest::Client::new();
    let body = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .bearer_auth(api_key.trim())
        .body(serde_json::to_string(&json!({"model": model,
            "messages": [
            {
                "role": "system",
                "content": "You are a helpful code assistant. Only reply with code that would be correct in the context."
            },
            {
                "role": "user",
                "content": instructions
            }
            ]}))?)
        .send()
        .await
        .context("openai post failed")?;
    let json = body.text().await.context("failed to read body")?;
    serde_json::from_str(&json).context("failed to parse json")
}
