use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(transparent)]
pub struct MessageId(i64);

#[derive(Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
#[serde(transparent)]
pub struct UserId(i64);

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
#[serde(transparent)]
pub struct ChatId(i64);

#[derive(Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct UpdateId(i64);

#[derive(Deserialize)]
pub struct User {
    pub id: UserId,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
}

#[derive(Deserialize)]
pub enum ChatType {
    #[serde(rename = "private")]
    Private,
    #[serde(rename = "group")]
    Group,
    #[serde(rename = "supergroup")]
    SuperGroup,
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
pub struct Chat {
    pub id: ChatId,
    #[serde(rename = "type")]
    pub chat_type: ChatType,
    pub username: Option<String>,
}

#[derive(Deserialize)]
pub struct MessageEntity {
    pub offset: usize,
    pub length: usize,
    #[serde(rename = "type")]
    pub entity_type: String,
}

#[derive(Deserialize)]
pub enum DiceType {
    #[serde(rename = "🎲")]
    Dice,
    #[serde(rename = "🎯")]
    Dart,
    #[serde(rename = "🎰")]
    Bowling,
    #[serde(rename = "🎳")]
    Basketball,
    #[serde(rename = "🏀")]
    Football,
    #[serde(rename = "⚽")]
    SlotMachine,
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
pub struct Dice {
    pub emoji: DiceType,
    pub value: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InlineKeyboardButton {
    pub text: String,
    pub callback_data: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ReplyMarkup {
    pub inline_keyboard: Option<Vec<Vec<InlineKeyboardButton>>>,
}

#[derive(Deserialize)]
pub struct Message {
    pub message_id: MessageId,
    pub from: Option<User>,
    pub chat: Chat,
    pub text: Option<String>,
    pub dice: Option<Dice>,
    pub entities: Option<Vec<MessageEntity>>,
    pub forward_date: Option<i64>,
}

impl Message {
    pub fn get_commands(&self) -> Vec<String> {
        match (&self.entities, &self.text) {
            (Some(entity), Some(text)) => entity
                .iter()
                .filter(|entity| entity.entity_type == "bot_command")
                .map(|entity| text[entity.offset..entity.offset + entity.length].to_string())
                .collect(),
            _ => Vec::new(),
        }
    }
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub id: String,
    pub from: User,
    pub message: Option<Message>,
    pub data: Option<String>,
}

#[derive(Deserialize)]
pub struct Update {
    pub update_id: UpdateId,
    pub message: Option<Message>,
    pub callback_query: Option<CallbackQuery>,
}

#[derive(Deserialize)]
pub struct ResultMessage {
    pub ok: bool,
    pub result: Message,
}
