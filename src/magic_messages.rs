use std::sync::OnceLock;

use eventsource_stream::Eventsource;
use futures::{stream, Stream, StreamExt};
use serde::{Deserialize, Serialize};

use crate::prompt_messages::system_message;

#[derive(Deserialize, Serialize)]
struct Message {
    role: Option<String>,
    content: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct Choice {
    index: i32,
    delta: Message,
}

#[derive(Deserialize, Serialize)]
struct CompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Serialize)]
struct CompletionRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

async fn submit(
    request: CompletionRequest,
) -> Result<impl Stream<Item = CompletionResponse>, reqwest::Error> {
    static KEY: OnceLock<String> = OnceLock::new();
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    let client = CLIENT.get_or_init(reqwest::Client::new);
    let stream = client
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
        .bytes_stream()
        .eventsource();
    Ok(stream::unfold(stream, |mut stream| async {
        if let Some(Ok(event)) = stream.next().await {
            if event.data == "[DONE]" {
                return None;
            }
            match serde_json::from_str::<CompletionResponse>(&event.data) {
                Ok(response) => return Some((response, stream)),
                Err(err) => {
                    tracing::error!("Error while parsing OpenAI response: {}", err);
                }
            }
        };
        None
    }))
}

pub async fn magic(
    message: &str,
    hint: &Option<String>,
) -> Result<impl Stream<Item = Option<String>>, reqwest::Error> {
    let request = CompletionRequest {
        model: "gpt-4-1106-preview".into(),
        messages: vec![
            Message {
                role: Some("user".into()),
                content: Some(message.to_owned()),
            },
            Message {
                role: Some("system".into()),
                content: Some(system_message(hint)),
            },
        ],
        stream: true,
    };
    let stream_response = submit(request).await?;
    Ok(stream_response.then(|response| async move {
        let Some(choice) = response.choices.first() else {
            return None;
        };
        let Some(content) = &choice.delta.content else {
            return None;
        };
        Some(content.clone())
    }))
}
