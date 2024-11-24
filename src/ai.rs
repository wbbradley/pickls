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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenAIRole {
    System,
    User,
    Assistant,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct OpenAIChatCompletionChoiceMessage {
    pub content: String,
    pub role: OpenAIRole,
    // ignore: refusal
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct OpenAIChatCompletionChoice {
    pub finish_reason: String,
    pub index: u64,
    // ignore: logprobs: serde_json::Value,
    pub message: OpenAIChatCompletionChoiceMessage,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct OpenAIChatCompletion {
    pub choices: Vec<OpenAIChatCompletionChoice>,
    pub created: u64,
    pub id: String,
    pub model: String,
    pub object: String,
    pub system_fingerprint: String,
    // ignore: pub usage: serde_json::Value,
}
const INLINE_ASSIST_SYSTEM_PROMPT: &str =
    "You are a helpful code assistant. Reply with only code and/or comments that would be correct in the context. NEVER include markdown (like ```) annotating your response.";

pub async fn fetch_completion(
    //<T: JsonSchema + DeserializeOwned>(
    api_key: String,
    model: String,
    instructions: String,
) -> Result<OpenAIChatCompletion> {
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
        .body(serde_json::to_string(&json!({
        "model": model,
        "messages": [
            {
                "role": "system",
                "content": INLINE_ASSIST_SYSTEM_PROMPT
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
