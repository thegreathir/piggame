use std::sync::OnceLock;

use super::telegram_types;
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

impl MessageInfo {
    async fn apply_magic(&mut self) {
        if self.is_premium {
            self.text = crate::magic_messages::magic(self.text.clone(), self.hint.clone()).await;
        }
    }
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

pub async fn send(chat_id: telegram_types::ChatId, action: MessageAction) {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    static TOKEN: OnceLock<String> = OnceLock::new();
    let bot_token = TOKEN.get_or_init(|| {
        std::env::var("BOT_TOKEN").expect("BOT_TOKEN environment variable is not set")
    });
    let client = CLIENT.get_or_init(reqwest::Client::new);
    let result = match action {
        MessageAction::Send(mut info) => {
            info.apply_magic().await;
            client
                .post(format!(
                    "https://api.telegram.org/bot{}/{}",
                    bot_token, "sendMessage"
                ))
                .json(&ChatMessage::<MessageInfo> { chat_id, info })
                .send()
                .await
        }
        MessageAction::Edit(mut info) => {
            info.message_info.apply_magic().await;
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
            }
        }
        Err(err) => {
            tracing::error!("Can not send a request to Telegram, error: {}", err);
        }
    };
}
