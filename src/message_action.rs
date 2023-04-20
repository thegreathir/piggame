use super::telegram_types;
use serde::Serialize;

#[derive(Serialize)]
pub struct MessageInfo {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to_message_id: Option<telegram_types::MessageId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_markup: Option<telegram_types::ReplyMarkup>,
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

pub struct MessageSender {
    bot_token: String,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct ChatMessage<Info: Serialize> {
    chat_id: telegram_types::ChatId,
    #[serde(flatten)]
    info: Info,
}

impl MessageSender {
    pub fn new(bot_token: String) -> MessageSender {
        MessageSender {
            bot_token,
            client: reqwest::Client::new(),
        }
    }

    pub async fn send(&self, chat_id: telegram_types::ChatId, action: MessageAction) {
        let result = match action {
            MessageAction::Send(info) => {
                self.client
                    .post(format!(
                        "https://api.telegram.org/bot{}/{}",
                        self.bot_token, "sendMessage"
                    ))
                    .json(&ChatMessage::<MessageInfo> { chat_id, info })
                    .send()
                    .await
            }
            MessageAction::Edit(info) => {
                self.client
                    .post(format!(
                        "https://api.telegram.org/bot{}/{}",
                        self.bot_token, "editMessageText"
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
}
