use dashmap::DashMap;
use std::{collections::HashMap, env, sync::Arc};
use warp::Filter;

mod telegram_types;
mod text_messages;

async fn send_message(bot_token: &str, target_chat: i64, text: String, reply: Option<i64>) {
    let end_point: String = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
    let body = serde_json::json!({
        "chat_id": target_chat,
        "text": text,
        "reply_to_message_id": reply
    });
    let client = reqwest::Client::new();
    client.post(end_point).json(&body).send().await.unwrap();
}

mod game_model;

type GameStateStorage = Arc<DashMap<i64, game_model::GameState>>;

async fn handle_private_message(bot_token: &str, message: telegram_types::Message) {
    send_message(
        bot_token,
        message.chat.id,
        r"Add this bot to groups to enjoy the Pig (dice) game!".to_string(),
        None,
    )
    .await;
}

async fn handle_group_message(
    bot_token: &str,
    message: telegram_types::Message,
    storage: GameStateStorage,
) {
    let mut game = storage
        .entry(message.chat.id)
        .or_insert(game_model::GameState::New(game_model::NewGame::new()));

    match message.dice {
        None => {
            for command in message.get_commands() {
                game.handle_command(bot_token, &message, command.as_str())
                    .await;
            }
        }
        Some(ref dice) => {
            if matches!(dice.get_type(), telegram_types::DiceType::Dice) {
                game.handle_dice(bot_token, &message, dice.value as u8)
                    .await;
            }
        }
    }
}

async fn handle(bot_token: String, update: telegram_types::Update, storage: GameStateStorage) {
    if let Some(message) = update.message {
        match message.chat.chat_type.as_str() {
            "group" | "supergroup" => handle_group_message(&bot_token, message, storage).await,
            "private" => handle_private_message(&bot_token, message).await,
            _ => (),
        }
    }
}

#[tokio::main]
async fn main() {
    let bot_token: String = env::var("BOT_TOKEN").unwrap();
    let storage = GameStateStorage::new(DashMap::new());

    let route = warp::path::end()
        .and(warp::body::json())
        .and(warp::any().map(move || storage.clone()))
        .and(warp::any().map(move || bot_token.clone()))
        .and_then(
            |body: serde_json::Value, storage: GameStateStorage, bot_token: String| async {
                let update: telegram_types::Update = serde_json::value::from_value(body).unwrap();
                handle(bot_token, update, storage).await;
                Ok::<_, std::convert::Infallible>(warp::reply())
            },
        );

    warp::serve(route).run(([127, 0, 0, 1], 32926)).await;
}
