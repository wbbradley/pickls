use crate::prelude::*;
use schemars::JsonSchema;

#[derive(Debug, Serialize)]
pub struct InlineAssistTemplateContext {
    pub language_id: String,
    pub text: String,
    pub include_workspace_files: bool,
    pub files: HashMap<String, String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InlineAssistResponse {
    pub provider: String,
    pub model: String,
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

/// The completion object returned by the Ollama API.
/// {
///   "model": "llama3.2",
///   "created_at": "2023-08-04T19:22:45.499127Z",
///   "response": "The sky is blue because it is the color of the sky.",
///   "done": true,
///   "context": [1, 2, 3],
///   "total_duration": 5043500667,
///   "load_duration": 5025959,
///   "prompt_eval_count": 26,
///   "prompt_eval_duration": 325953000,
///   "eval_count": 290,
///   "eval_duration": 4709213000
/// }
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct OllamaChatCompletion {
    pub model: String,
    pub created_at: String,
    pub response: String,
    pub done: bool,
    // ignore: pub usage: serde_json::Value,
}

pub async fn fetch_openai_completion(
    //<T: JsonSchema + DeserializeOwned>(
    api_key: String,
    model: String,
    system_prompt: String,
    instructions: String,
) -> Result<OpenAIChatCompletion> {
    log::info!(
        "fetching openai completion with {} of {}",
        &api_key[0..4],
        &instructions[0..10]
    );
    let client = reqwest::Client::new();
    let body = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .bearer_auth(api_key.trim())
        .json(&json!({
        "model": model,
        "messages": [
            {
                "role": "system",
                "content": system_prompt,
            },
            {
                "role": "user",
                "content": instructions
            }
        ]}))
        .send()
        .await
        .context("openai post failed")?;
    let json = body.text().await.context("failed to read body")?;
    serde_json::from_str(&json).context("failed to parse json")
}

pub async fn fetch_ollama_completion(
    //<T: JsonSchema + DeserializeOwned>(
    api_address: String,
    model: String,
    system_prompt: String,
    prompt: String,
) -> Result<OllamaChatCompletion> {
    log::info!(
        "fetching ollama completion with {} of {}",
        &api_address,
        &prompt,
    );
    let client = reqwest::Client::new();
    let body = client
        .post(api_address)
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": model,
            "system": system_prompt,
            "prompt": prompt,
            "stream": false
        }))
        .send()
        .await
        .context("ollama post failed")?;
    let json = body.text().await.context("failed to read body")?;
    serde_json::from_str(&json).context("failed to parse json")
}
