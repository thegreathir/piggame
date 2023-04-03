use dashmap::DashMap;
use serde::Serialize;
use std::{collections::HashMap, env, sync::Arc};
use warp::Filter;

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

struct MessageInfo {
    message_id: i64,
    chat_id: i64,
    sender_id: i64,
    sender_name: String,
    sender_username: Option<String>,
}

#[derive(Serialize)]
struct Player {
    #[serde(skip_serializing)]
    user_id: i64,
    name: String,
    username: Option<String>,
    score: u8,
}

impl Player {
    fn get_mention_string(&self) -> String {
        match &self.username {
            Some(username) => format!("@{}", username),
            None => self.name.clone(),
        }
    }
}

enum GameState {
    New,
    Playing,
}
struct Game {
    state: GameState,
    players: HashMap<i64, Player>,
    turn: u8,
    current_score: u8,
}

#[derive(Debug)]
enum GameLogicError {
    JoinAfterPlay,
    AlreadyPlaying,
    IsNotPlaying,
    WrongTurn,
    NotEnoughPlayers,
    AlreadyJoined,
}

impl Game {
    fn new() -> Game {
        Game {
            state: GameState::New,
            players: HashMap::new(),
            turn: 0,
            current_score: 0,
        }
    }

    fn join(
        &mut self,
        user_id: i64,
        username: Option<String>,
        name: String,
    ) -> Result<(), GameLogicError> {
        match self.state {
            GameState::New => {
                if let std::collections::hash_map::Entry::Vacant(e) = self.players.entry(user_id) {
                    e.insert(Player {
                        user_id,
                        score: 0,
                        name,
                        username,
                    });
                    Ok(())
                } else {
                    Err(GameLogicError::AlreadyJoined)
                }
            }
            GameState::Playing => Err(GameLogicError::JoinAfterPlay),
        }
    }

    fn play(&mut self) -> Result<(), GameLogicError> {
        match self.state {
            GameState::New => {
                if self.players.len() >= 2 {
                    self.state = GameState::Playing;
                    Ok(())
                } else {
                    Err(GameLogicError::NotEnoughPlayers)
                }
            }
            GameState::Playing => Err(GameLogicError::AlreadyPlaying),
        }
    }

    fn reset(&mut self) {
        self.players.clear();
        self.current_score = 0;
        self.turn = 0;
        self.state = GameState::New;
    }

    fn check_playing(&self) -> Result<(), GameLogicError> {
        match self.state {
            GameState::Playing => Ok(()),
            GameState::New => Err(GameLogicError::IsNotPlaying),
        }
    }

    fn get_current_player_mut(&mut self) -> &mut Player {
        let all_players: Vec<i64> = self.players.keys().cloned().collect();
        self.players
            .get_mut(&all_players[self.turn as usize])
            .unwrap()
    }

    fn get_current_player(&self) -> &Player {
        let all_players: Vec<&i64> = self.players.keys().collect();
        &self.players[all_players[self.turn as usize]]
    }

    fn check_turn(&self, user_id: i64) -> Result<(), GameLogicError> {
        if user_id != self.get_current_player().user_id {
            Err(GameLogicError::WrongTurn)
        } else {
            Ok(())
        }
    }

    fn advance_turn(&mut self) {
        self.current_score = 0;
        self.turn += 1;
        self.turn %= self.players.len() as u8;
    }

    fn add_dice(&mut self, user_id: i64, value: u8) -> Result<(bool, bool), GameLogicError> {
        self.check_playing()?;
        self.check_turn(user_id)?;
        if value == 1 {
            self.advance_turn();
            Ok((false, true))
        } else {
            self.current_score += value;
            if self.get_current_player().score + self.current_score >= 100 {
                Ok((true, false))
            } else {
                Ok((false, false))
            }
        }
    }

    fn hold(&mut self, user_id: i64) -> Result<u8, GameLogicError> {
        self.check_playing()?;
        self.check_turn(user_id)?;
        self.get_current_player_mut().score += self.current_score;
        let result = self.get_current_player().score;
        self.advance_turn();
        Ok(result)
    }

    async fn send_results(&self, bot_token: &str, chat_id: i64) {
        let players: Vec<&Player> = self.players.values().collect();
        send_message(
            bot_token,
            chat_id,
            format!("Scores:\n {}", serde_yaml::to_string(&players).unwrap()),
            None,
        )
        .await;
    }

    async fn handle_dice(&mut self, bot_token: &str, message_info: MessageInfo, dice_value: u8) {
        if let Ok((finished, turn_changed)) = self.add_dice(message_info.sender_id, dice_value) {
            if finished {
                self.hold(message_info.sender_id).unwrap();
                self.send_results(bot_token, message_info.chat_id).await;
                self.reset();
            } else if turn_changed {
                send_message(
                    bot_token,
                    message_info.chat_id,
                    format!(
                        "Oops! New turn: {}",
                        self.get_current_player().get_mention_string()
                    ),
                    Some(message_info.message_id),
                )
                .await
            } else {
                send_message(
                    bot_token,
                    message_info.chat_id,
                    format!(
                        "{} + {} = {}",
                        self.get_current_player().score,
                        self.current_score,
                        self.get_current_player().score + self.current_score
                    ),
                    Some(message_info.message_id),
                )
                .await
            }
        }
    }
    async fn handle_command(&mut self, bot_token: &str, message_info: &MessageInfo, command: &str) {
        match command {
            "/join" | "/join@piiigdicegamebot" => {
                match self.join(
                    message_info.sender_id,
                    message_info.sender_username.clone(),
                    message_info.sender_name.to_string(),
                ) {
                    Ok(_) => {
                        send_message(
                            bot_token,
                            message_info.chat_id,
                            "Joined successfully ;)".to_string(),
                            Some(message_info.message_id),
                        )
                        .await
                    }
                    Err(GameLogicError::JoinAfterPlay) => {
                        send_message(
                            bot_token,
                            message_info.chat_id,
                            "Game is already started :(".to_string(),
                            Some(message_info.message_id),
                        )
                        .await
                    }
                    Err(GameLogicError::AlreadyJoined) => {
                        send_message(
                            bot_token,
                            message_info.chat_id,
                            "You have joined already :)".to_string(),
                            Some(message_info.message_id),
                        )
                        .await
                    }
                    Err(_) => (),
                }
            }
            "/play" | "/play@piiigdicegamebot" => match self.play() {
                Ok(_) => {
                    send_message(
                        bot_token,
                        message_info.chat_id,
                        format!(
                            "Started successfully. Turn: {}",
                            self.get_current_player().get_mention_string()
                        ),
                        Some(message_info.message_id),
                    )
                    .await
                }
                Err(GameLogicError::AlreadyPlaying) => {
                    send_message(
                        bot_token,
                        message_info.chat_id,
                        "Game is already started :(".to_string(),
                        Some(message_info.message_id),
                    )
                    .await
                }
                Err(GameLogicError::NotEnoughPlayers) => {
                    send_message(
                        bot_token,
                        message_info.chat_id,
                        "Not enough players joined yet :(".to_string(),
                        Some(message_info.message_id),
                    )
                    .await
                }
                Err(_) => (),
            },
            "/hold" | "/hold@piiigdicegamebot" => match self.hold(message_info.sender_id) {
                Ok(score) => {
                    send_message(
                        bot_token,
                        message_info.chat_id,
                        format!(
                            "Your score is {}. Turn: {}",
                            score,
                            self.get_current_player().get_mention_string()
                        ),
                        Some(message_info.message_id),
                    )
                    .await
                }
                Err(GameLogicError::IsNotPlaying) => {
                    send_message(
                        bot_token,
                        message_info.chat_id,
                        "Game is not started yet :(".to_string(),
                        Some(message_info.message_id),
                    )
                    .await
                }
                Err(GameLogicError::WrongTurn) => {
                    send_message(
                        bot_token,
                        message_info.chat_id,
                        "This is not your turn :(".to_string(),
                        Some(message_info.message_id),
                    )
                    .await
                }
                Err(_) => (),
            },
            "/result" | "/result@piiigdicegamebot" => {
                self.send_results(bot_token, message_info.chat_id).await
            }
            "/reset" | "/reset@piiigdicegamebot" => {
                self.reset();
                send_message(
                    bot_token,
                    message_info.chat_id,
                    "Game is reset (players should join again).".to_string(),
                    Some(message_info.message_id),
                )
                .await
            }
            _ => (),
        }
    }
}

type GameStateStorage = Arc<DashMap<i64, Game>>;

async fn handle_private_message(bot_token: &str, message: &serde_json::Value) {
    send_message(
        bot_token,
        message
            .get("chat")
            .unwrap()
            .get("id")
            .unwrap()
            .as_i64()
            .unwrap(),
        r"Add this bot to groups to enjoy the Pig (dice) game!".to_string(),
        None,
    )
    .await;
}

async fn handle_group_message(
    bot_token: &str,
    message: &serde_json::Value,
    storage: GameStateStorage,
) {
    let chat_id = message
        .get("chat")
        .unwrap()
        .get("id")
        .unwrap()
        .as_i64()
        .unwrap();
    let sender_id = message
        .get("from")
        .unwrap()
        .get("id")
        .unwrap()
        .as_i64()
        .unwrap();
    let sender_username = message
        .get("from")
        .unwrap()
        .get("username")
        .unwrap()
        .as_str();
    let sender_name = message
        .get("from")
        .unwrap()
        .get("first_name")
        .unwrap()
        .as_str()
        .unwrap();
    let message_id = message.get("message_id").unwrap().as_i64().unwrap();
    let message_info = MessageInfo {
        chat_id,
        sender_id,
        sender_username: sender_username.map(String::from),
        sender_name: sender_name.to_string(),
        message_id,
    };
    let mut game = storage.entry(chat_id).or_insert(Game::new());

    match message.get("dice") {
        None => {
            if let Some(entities) = message.get("entities") {
                for entity in entities.as_array().unwrap() {
                    if entity.get("type").unwrap().as_str().unwrap() == "bot_command" {
                        let offset = entity.get("offset").unwrap().as_i64().unwrap() as usize;
                        let length = entity.get("length").unwrap().as_i64().unwrap() as usize;
                        game.handle_command(
                            bot_token,
                            &message_info,
                            &message.get("text").unwrap().as_str().unwrap()
                                [offset..offset + length],
                        )
                        .await;
                    }
                }
            }
        }
        Some(dice) => {
            if dice.get("emoji").unwrap().as_str().unwrap() == "🎲" {
                game.handle_dice(
                    bot_token,
                    message_info,
                    dice.get("value").unwrap().as_i64().unwrap() as u8,
                )
                .await
            }
        }
    }
}

async fn handle(bot_token: String, body: serde_json::Value, storage: GameStateStorage) {
    match body.get("message") {
        None => (),
        Some(message) => {
            match message
                .get("chat")
                .unwrap()
                .get("type")
                .unwrap()
                .as_str()
                .unwrap()
            {
                "group" | "supergroup" => handle_group_message(&bot_token, message, storage).await,
                "private" => handle_private_message(&bot_token, message).await,
                _ => (),
            }
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
                handle(bot_token, body, storage).await;
                Ok::<_, std::convert::Infallible>(warp::reply())
            },
        );

    warp::serve(route).run(([127, 0, 0, 1], 32926)).await;
}
