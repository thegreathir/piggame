use dashmap::{mapref::entry::Entry, DashMap};
use prompt_messages::{greeting, greeting_hint};
use std::sync::Arc;
use warp::Filter;

mod game_model;
mod magic_messages;
mod message_action;
mod premium;
mod prompt_messages;
mod telegram_types;
mod text_messages;

type GameStateStorage = Arc<DashMap<telegram_types::ChatId, game_model::GameState>>;

async fn handle_private_message(message: telegram_types::Message) {
    let (hint, is_premium) = match message.from {
        Some(sender) => (
            Some(greeting_hint(&sender.first_name)),
            premium::is_premium(sender.username.unwrap_or_default()),
        ),
        None => (None, false),
    };
    message_action::send(
        message.chat.id,
        message_action::MessageAction::Send(message_action::MessageInfo {
            text: greeting().to_owned(),
            reply_to_message_id: None,
            reply_markup: None,
            hint,
            is_premium,
        }),
    )
    .await;
}

async fn handle_group_message(message: telegram_types::Message, storage: GameStateStorage) {
    let mut actions = vec![];
    {
        let mut game = storage
            .entry(message.chat.id)
            .or_insert(game_model::GameState::New(game_model::NewGame::new()));

        match message.dice {
            None => {
                for command in message.get_commands() {
                    actions.extend(game.handle_command(&message, command.as_str()));
                }
            }
            Some(ref dice) => {
                if matches!(dice.emoji, telegram_types::DiceType::Dice)
                    && message.forward_date.is_none()
                {
                    actions.extend(game.handle_dice(&message, dice.value as u8));
                };
            }
        };
    }

    for action in actions {
        message_action::send(message.chat.id, action).await;
    }
}

async fn handle(update: telegram_types::Update, storage: GameStateStorage) {
    if let Some(message) = update.message {
        match message.chat.chat_type {
            telegram_types::ChatType::Group | telegram_types::ChatType::SuperGroup => {
                handle_group_message(message, storage).await
            }
            telegram_types::ChatType::Private => handle_private_message(message).await,
            _ => (),
        }
    } else if let Some(callback_query) = update.callback_query {
        if let Some(message) = callback_query.message {
            let actions = match storage.entry(message.chat.id) {
                Entry::Occupied(mut occupied) => {
                    let game = occupied.get_mut();
                    game.handle_callback_query(&message, callback_query.data)
                }
                Entry::Vacant(_) => vec![],
            };
            for action in actions {
                message_action::send(message.chat.id, action).await;
            }
        };
    };
}

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    let storage = GameStateStorage::new(DashMap::new());

    let route = warp::path::end()
        .and(warp::body::json())
        .and(warp::any().map(move || storage.clone()))
        .and_then(|body: serde_json::Value, storage: GameStateStorage| async {
            match serde_json::value::from_value::<telegram_types::Update>(body) {
                Ok(update) => {
                    handle(update, storage).await;
                }
                Err(err) => {
                    tracing::error!("Can not parse Telegram request body, error: {}", err);
                }
            };
            Ok::<_, std::convert::Infallible>(warp::reply())
        });

    warp::serve(route).run(([127, 0, 0, 1], 32926)).await;
}
