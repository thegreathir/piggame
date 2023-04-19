use super::telegram_types;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct MessageInfo {
    pub chat_id: i64,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to_message_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_markup: Option<telegram_types::ReplyMarkup>,
}

pub enum MessageAction {
    Send(MessageInfo),
    Edit(MessageInfo),
}

pub struct MessageSender {
    bot_token: String,
    client: reqwest::Client,
}

impl MessageSender {
    pub fn new(bot_token: String) -> MessageSender {
        MessageSender {
            bot_token,
            client: reqwest::Client::new(),
        }
    }

    pub async fn send(&self, action: MessageAction) {
        let result = match action {
            MessageAction::Send(info) => {
                self.client
                    .post(format!(
                        "https://api.telegram.org/bot{}/{}",
                        self.bot_token, "sendMessage"
                    ))
                    .json(&info)
                    .send()
                    .await
            }
            MessageAction::Edit(info) => {
                self.client
                    .post(format!(
                        "https://api.telegram.org/bot{}/{}",
                        self.bot_token, "editMessageText"
                    ))
                    .json(&info)
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
