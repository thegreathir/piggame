use std::{pin::pin, sync::OnceLock};

use super::telegram_types;
use futures::StreamExt;
use serde::Serialize;

#[derive(Serialize)]
pub struct MessageInfo {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to_message_id: Option<telegram_types::MessageId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_markup: Option<telegram_types::ReplyMarkup>,
    #[serde(skip_serializing)]
    pub hint: Option<String>,
    #[serde(skip_serializing)]
    pub is_premium: bool,
}

#[derive(Serialize)]
pub struct EditMessageInfo {
    pub message_id: telegram_types::MessageId,
    #[serde(flatten)]
    pub message_info: MessageInfo,
}

pub enum MessageAction {
    Send(MessageInfo),
    Edit(EditMessageInfo),
}

#[derive(Serialize)]
struct ChatMessage<Info: Serialize> {
    chat_id: telegram_types::ChatId,
    #[serde(flatten)]
    info: Info,
}

async fn send_stream(
    client: &reqwest::Client,
    bot_token: &String,
    info: MessageInfo,
    chat_id: telegram_types::ChatId,
    mut message_id: Option<telegram_types::MessageId>,
) {
    if let Some(new_message_id) =
        append_chunk(message_id, client, bot_token, chat_id, "...", &info).await
    {
        message_id = Some(new_message_id);
    } else {
        return;
    }
    match crate::magic_messages::magic(&info.text, &info.hint).await {
        Ok(stream) => {
            let mut stream = pin!(stream);
            let mut text = String::new();
            let mut added_len = 0;
            while let Some(chunk) = stream.next().await {
                let Some(chunk) = chunk else {
                    continue;
                };
                text.push_str(&chunk);
                added_len += chunk.len();
                if added_len < 50 {
                    continue;
                }
                added_len = 0;
                if let Some(new_message_id) =
                    append_chunk(message_id, client, bot_token, chat_id, &text, &info).await
                {
                    message_id = Some(new_message_id);
                } else {
                    return;
                }
            }
            if added_len == 0 {
                return;
            }
            append_chunk(message_id, client, bot_token, chat_id, &text, &info).await;
        }
        Err(err) => {
            tracing::error!("Failed to call OpenAI API, error: {}", err);
        }
    }
}

async fn append_chunk(
    message_id: Option<telegram_types::MessageId>,
    client: &reqwest::Client,
    bot_token: &String,
    chat_id: telegram_types::ChatId,
    text: &str,
    info: &MessageInfo,
) -> Option<telegram_types::MessageId> {
    match message_id {
        Some(message_id) => {
            let result = get_result(
                client
                    .post(format!(
                        "https://api.telegram.org/bot{}/{}",
                        bot_token, "editMessageText"
                    ))
                    .json(&ChatMessage::<EditMessageInfo> {
                        chat_id,
                        info: EditMessageInfo {
                            message_id,
                            message_info: MessageInfo {
                                text: text.to_owned(),
                                reply_markup: info.reply_markup.clone(),
                                hint: Option::None,
                                ..*info
                            },
                        },
                    })
                    .send()
                    .await,
            )
            .await?;
            Some(result.result.message_id)
        }
        None => {
            let result = get_result(
                client
                    .post(format!(
                        "https://api.telegram.org/bot{}/{}",
                        bot_token, "sendMessage"
                    ))
                    .json(&ChatMessage::<MessageInfo> {
                        chat_id,
                        info: MessageInfo {
                            text: text.to_owned(),
                            reply_markup: info.reply_markup.clone(),
                            hint: Option::None,
                            ..*info
                        },
                    })
                    .send()
                    .await,
            )
            .await?;
            Some(result.result.message_id)
        }
    }
}

pub async fn send(chat_id: telegram_types::ChatId, action: MessageAction) {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    static TOKEN: OnceLock<String> = OnceLock::new();
    let bot_token = TOKEN.get_or_init(|| {
        std::env::var("BOT_TOKEN").expect("BOT_TOKEN environment variable is not set")
    });
    let client = CLIENT.get_or_init(reqwest::Client::new);
    let result = match action {
        MessageAction::Send(info) => {
            if info.is_premium {
                send_stream(client, bot_token, info, chat_id, None).await;
                return;
            }
            client
                .post(format!(
                    "https://api.telegram.org/bot{}/{}",
                    bot_token, "sendMessage"
                ))
                .json(&ChatMessage::<MessageInfo> { chat_id, info })
                .send()
                .await
        }
        MessageAction::Edit(info) => {
            if info.message_info.is_premium {
                send_stream(
                    client,
                    bot_token,
                    info.message_info,
                    chat_id,
                    Some(info.message_id),
                )
                .await;
                return;
            }
            client
                .post(format!(
                    "https://api.telegram.org/bot{}/{}",
                    bot_token, "editMessageText"
                ))
                .json(&ChatMessage::<EditMessageInfo> { chat_id, info })
                .send()
                .await
        }
    };

    handle_api_call(result).await;
}

async fn handle_api_call(
    result: Result<reqwest::Response, reqwest::Error>,
) -> Option<reqwest::Response> {
    match result {
        Ok(res) => {
            if !res.status().is_success() {
                match res.text().await {
                    Ok(body) => {
                        tracing::error!("Telegram API call was not success, {}", body);
                    }
                    Err(_) => {
                        tracing::error!("Telegram API call was not success, can not extract response text neither");
                    }
                }
                return None;
            }
            Some(res)
        }
        Err(err) => {
            tracing::error!("Can not send a request to Telegram, error: {}", err);
            None
        }
    }
}

async fn get_result(
    response_result: Result<reqwest::Response, reqwest::Error>,
) -> Option<telegram_types::ResultMessage> {
    let response = handle_api_call(response_result).await?;
    match response.json::<telegram_types::ResultMessage>().await {
        Ok(result) => Some(result),
        Err(err) => {
            tracing::error!("Can not parse Telegram response, error: {}", err);
            None
        }
    }
}
