use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub id: i64,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Chat {
    pub id: i64,
    #[serde(rename = "type")]
    pub chat_type: String,
    pub username: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MessageEntity {
    pub offset: usize,
    pub length: usize,
    #[serde(rename = "type")]
    pub entity_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Dice {
    pub emoji: String,
    pub value: i64,
}

pub enum DiceType {
    Unknown,
    Dice,
    Dart,
    Bowling,
    Basketball,
    Football,
    SlotMachine,
}

impl Dice {
    pub fn get_type(&self) -> DiceType {
        match self.emoji.as_str() {
            "🎲" => DiceType::Dice,
            "🎯" => DiceType::Dart,
            "🎰" => DiceType::SlotMachine,
            "🎳" => DiceType::Bowling,
            "🏀" => DiceType::Basketball,
            "⚽" => DiceType::Football,
            _ => DiceType::Unknown,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InlineKeyboardButton {
    pub text: String,
    pub callback_data: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    pub message_id: i64,
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

#[derive(Debug, Deserialize, Serialize)]
pub struct CallbackQuery {
    pub id: String,
    pub from: User,
    pub message: Option<Message>,
    pub data: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Update {
    pub update_id: i64,
    pub message: Option<Message>,
    pub callback_query: Option<CallbackQuery>,
}
