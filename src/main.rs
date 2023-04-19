use dashmap::{mapref::entry::Entry, DashMap};
use std::{env, sync::Arc};
use warp::Filter;

mod game_model;
mod message_action;
mod telegram_types;
mod text_messages;

type GameStateStorage = Arc<DashMap<i64, game_model::GameState>>;

async fn handle_private_message(
    message_sender: message_action::MessageSender,
    message: telegram_types::Message,
) {
    message_sender
        .send(message_action::MessageAction::Send(
            message_action::MessageInfo {
                chat_id: message.chat.id,
                text: "Add this bot to groups to enjoy the Pig (dice) game!".to_string(),
                message_id: None,
                reply_to_message_id: None,
                reply_markup: None,
            },
        ))
        .await;
}

async fn handle_group_message(
    message_sender: message_action::MessageSender,
    message: telegram_types::Message,
    storage: GameStateStorage,
) {
    let mut game = storage
        .entry(message.chat.id)
        .or_insert(game_model::GameState::New(game_model::NewGame::new()));

    let mut actions = vec![];
    match message.dice {
        None => {
            for command in message.get_commands() {
                actions.extend(game.handle_command(&message, command.as_str()));
            }
        }
        Some(ref dice) => {
            if matches!(dice.get_type(), telegram_types::DiceType::Dice)
                && message.forward_date.is_none()
            {
                actions.extend(game.handle_dice(&message, dice.value as u8));
            };
        }
    };

    for action in actions {
        message_sender.send(action).await;
    }
}

async fn handle(
    message_sender: message_action::MessageSender,
    update: telegram_types::Update,
    storage: GameStateStorage,
) {
    if let Some(message) = update.message {
        match message.chat.chat_type.as_str() {
            "group" | "supergroup" => handle_group_message(message_sender, message, storage).await,
            "private" => handle_private_message(message_sender, message).await,
            _ => (),
        }
    } else if let Some(callback_query) = update.callback_query {
        if let Some(message) = callback_query.message {
            match storage.entry(message.chat.id) {
                Entry::Occupied(mut occupied) => {
                    let game = occupied.get_mut();
                    for action in game.handle_callback_query(&message, callback_query.data) {
                        message_sender.send(action).await;
                    }
                }
                Entry::Vacant(_) => (),
            };
        };
    };
}

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    let bot_token: String = env::var("BOT_TOKEN").unwrap();
    let storage = GameStateStorage::new(DashMap::new());

    let route = warp::path::end()
        .and(warp::body::json())
        .and(warp::any().map(move || storage.clone()))
        .and(warp::any().map(move || bot_token.clone()))
        .and_then(
            |body: serde_json::Value, storage: GameStateStorage, bot_token: String| async {
                match serde_json::value::from_value::<telegram_types::Update>(body) {
                    Ok(update) => {
                        let message_sender = message_action::MessageSender::new(bot_token);
                        handle(message_sender, update, storage).await;
                    },
                    Err(err) => {
                        tracing::error!("Can not parse Telegram request body, error: {}", err);
                    }
                };
                Ok::<_, std::convert::Infallible>(warp::reply())
            },
        );

    warp::serve(route).run(([127, 0, 0, 1], 32926)).await;
}
