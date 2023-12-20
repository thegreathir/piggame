use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::prompt_messages::system_message;

#[derive(Deserialize, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize, Serialize)]
struct Choice {
    index: i32,
    message: Message,
}

#[derive(Deserialize, Serialize)]
struct CompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Serialize)]
struct CompletionRequest {
    model: String,
    messages: Vec<Message>,
}

async fn submit(request: CompletionRequest) -> Result<CompletionResponse, reqwest::Error> {
    static KEY: OnceLock<String> = OnceLock::new();
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    let client = CLIENT.get_or_init(reqwest::Client::new);
    client
        .post("https://api.openai.com/v1/chat/completions")
        .header(
            "Authorization",
            format!(
                "Bearer {}",
                KEY.get_or_init(|| { std::env::var("OPENAI_API_KEY").unwrap().as_str().into() })
            ),
        )
        .json(&request)
        .send()
        .await?
        .json::<CompletionResponse>()
        .await
}

pub async fn magic(message: String, hint: Option<String>) -> String {
    let request = CompletionRequest {
        model: "gpt-4-1106-preview".into(),
        messages: vec![
            Message {
                role: "user".into(),
                content: message.clone(),
            },
            Message {
                role: "system".into(),
                content: system_message(&hint),
            },
        ],
    };
    let Ok(response) = submit(request).await else {
        return message;
    };
    let Some(choice) = response.choices.get(0) else {
        return message;
    };
    choice.message.content.clone()
}
