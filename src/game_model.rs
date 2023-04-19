use super::message_action;
use super::telegram_types;
use super::text_messages;
use rand::seq::SliceRandom;
use std::collections::HashMap;

struct Player {
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

    fn show(&self, score: bool) -> String {
        let name = match &self.username {
            Some(username) => format!("{} ({})", self.name, username),
            None => self.name.clone(),
        };
        if score {
            format!("{}: {}", name, self.score)
        } else {
            name
        }
    }
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

enum AddDiceResult<'a> {
    Finished,
    TurnLost(&'a Player),
    Continue(&'a Player, u8),
}

#[derive(Default)]
pub struct NewGame {
    players: HashMap<i64, Player>,
}

impl NewGame {
    pub fn new() -> NewGame {
        NewGame {
            players: HashMap::new(),
        }
    }

    fn send_players(&self, chat_id: i64) -> message_action::MessageAction {
        let text = if self.players.is_empty() {
            "No players!".to_string()
        } else {
            let players_text = self.players.values().fold("".to_string(), |res, player| {
                format!("{}\n- {}", res, player.show(false))
            });
            format!("Players:{}", players_text)
        };
        message_action::MessageAction::Send(message_action::MessageInfo {
            chat_id,
            text,
            message_id: None,
            reply_to_message_id: None,
            reply_markup: None,
        })
    }
}

pub struct PlayingGame {
    players: Vec<Player>,
    turn: u8,
    current_score: u8,
}

impl PlayingGame {
    fn from(new_game: NewGame) -> PlayingGame {
        let mut players: Vec<Player> = new_game.players.into_values().collect();
        let mut rng = rand::thread_rng();
        players.shuffle(&mut rng);
        PlayingGame {
            players,
            turn: 0,
            current_score: 0,
        }
    }

    fn get_current_player_mut(&mut self) -> &mut Player {
        &mut self.players[self.turn as usize]
    }

    fn get_current_player(&self) -> &Player {
        &self.players[self.turn as usize]
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

    fn send_results(&self, chat_id: i64) -> message_action::MessageAction {
        let players_text =
            self.players
                .iter()
                .enumerate()
                .fold("".to_string(), |res, (i, player)| {
                    if player.score >= 100 {
                        format!(
                            "{}\n- {} {}",
                            res,
                            text_messages::KING_EMOJI,
                            player.show(true)
                        )
                    } else if self.turn as usize == i {
                        format!(
                            "{}\n- {} {}",
                            res,
                            text_messages::DICE_EMOJI,
                            player.show(true)
                        )
                    } else {
                        format!("{}\n- {}", res, player.show(true))
                    }
                });
        message_action::MessageAction::Send(message_action::MessageInfo {
            chat_id,
            text: format!("Scores:{}", players_text),
            message_id: None,
            reply_to_message_id: None,
            reply_markup: None,
        })
    }
}

// TODO: Move magic numbers and constant strings to configuration
pub enum GameState {
    New(NewGame),
    Playing(PlayingGame),
}

impl GameState {
    pub fn new() -> GameState {
        GameState::New(NewGame::new())
    }

    fn join(
        &mut self,
        user_id: i64,
        username: Option<String>,
        name: String,
    ) -> Result<(), GameLogicError> {
        match self {
            GameState::New(new_game) => {
                if let std::collections::hash_map::Entry::Vacant(e) =
                    new_game.players.entry(user_id)
                {
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
            GameState::Playing(_) => Err(GameLogicError::JoinAfterPlay),
        }
    }

    fn get_playing_game(&self) -> Option<&PlayingGame> {
        match self {
            GameState::Playing(playing_game) => Some(playing_game),
            _ => None,
        }
    }

    fn get_playing_game_mut(&mut self) -> Option<&mut PlayingGame> {
        match self {
            GameState::Playing(playing_game) => Some(playing_game),
            _ => None,
        }
    }

    fn play(&mut self) -> Result<&Player, GameLogicError> {
        match self {
            GameState::New(new_game) => {
                if new_game.players.len() >= 2 {
                    let playing_game = PlayingGame::from(std::mem::take(new_game));
                    *self = GameState::Playing(playing_game);
                    Ok(self.get_playing_game().unwrap().get_current_player())
                } else {
                    Err(GameLogicError::NotEnoughPlayers)
                }
            }
            GameState::Playing(_) => Err(GameLogicError::AlreadyPlaying),
        }
    }

    fn reset(&mut self) {
        *self = GameState::new();
    }

    fn add_dice(&mut self, user_id: i64, value: u8) -> Result<AddDiceResult<'_>, GameLogicError> {
        let playing_game = self
            .get_playing_game_mut()
            .ok_or(GameLogicError::IsNotPlaying)?;
        playing_game.check_turn(user_id)?;
        if value == 1 {
            playing_game.advance_turn();
            Ok(AddDiceResult::TurnLost(playing_game.get_current_player()))
        } else {
            playing_game.current_score += value;
            if playing_game.get_current_player().score + playing_game.current_score >= 100 {
                playing_game.get_current_player_mut().score += playing_game.current_score;
                playing_game.current_score = 0;
                Ok(AddDiceResult::Finished)
            } else {
                Ok(AddDiceResult::Continue(
                    playing_game.get_current_player(),
                    playing_game.current_score,
                ))
            }
        }
    }

    fn hold(&mut self, user_id: i64) -> Result<(u8, &Player), GameLogicError> {
        let playing_game = self
            .get_playing_game_mut()
            .ok_or(GameLogicError::IsNotPlaying)?;
        playing_game.check_turn(user_id)?;
        playing_game.get_current_player_mut().score += playing_game.current_score;
        let result = playing_game.get_current_player().score;
        playing_game.advance_turn();
        Ok((result, playing_game.get_current_player()))
    }

    fn send_results(&self, chat_id: i64) -> message_action::MessageAction {
        match self {
            GameState::New(new_game) => new_game.send_players(chat_id),
            GameState::Playing(playing_game) => playing_game.send_results(chat_id),
        }
    }

    pub fn handle_dice(
        &mut self,
        message: &telegram_types::Message,
        dice_value: u8,
    ) -> Vec<message_action::MessageAction> {
        if let Some(sender) = &message.from {
            match self.add_dice(sender.id, dice_value) {
                Ok(AddDiceResult::Finished) => {
                    let action = self.send_results(message.chat.id);
                    self.reset();
                    vec![action]
                }
                Ok(AddDiceResult::TurnLost(current_player)) => {
                    vec![
                        message_action::MessageAction::Send(message_action::MessageInfo {
                            chat_id: message.chat.id,
                            text: "Oops!".to_string(),
                            message_id: None,
                            reply_to_message_id: Some(message.message_id),
                            reply_markup: None,
                        }),
                        message_action::MessageAction::Send(message_action::MessageInfo {
                            chat_id: message.chat.id,
                            text: format!("Your turn: {}", current_player.get_mention_string()),
                            message_id: None,
                            reply_to_message_id: None,
                            reply_markup: None,
                        }),
                    ]
                }
                Ok(AddDiceResult::Continue(current_player, current_score)) => {
                    vec![message_action::MessageAction::Send(
                        message_action::MessageInfo {
                            chat_id: message.chat.id,
                            text: format!(
                                "{} + {} = {}",
                                current_player.score,
                                current_score,
                                current_player.score + current_score
                            ),
                            message_id: None,
                            reply_to_message_id: Some(message.message_id),
                            reply_markup: None,
                        },
                    )]
                }
                Err(_) => vec![],
            }
        } else {
            vec![]
        }
    }
    pub fn handle_command(
        &mut self,
        message: &telegram_types::Message,
        command: &str,
    ) -> Vec<message_action::MessageAction> {
        if let Some(sender) = &message.from {
            match command {
                "/join" | "/join@piiigdicegamebot" => {
                    match self.join(
                        sender.id,
                        sender.username.clone(),
                        sender.first_name.clone(),
                    ) {
                        Ok(_) => {
                            vec![message_action::MessageAction::Send(
                                message_action::MessageInfo {
                                    chat_id: message.chat.id,
                                    text: "Joined successfully ;)".to_string(),
                                    message_id: None,
                                    reply_to_message_id: Some(message.message_id),
                                    reply_markup: None,
                                },
                            )]
                        }
                        Err(GameLogicError::JoinAfterPlay) => {
                            vec![message_action::MessageAction::Send(
                                message_action::MessageInfo {
                                    chat_id: message.chat.id,
                                    text: "Game is already started :(".to_string(),
                                    message_id: None,
                                    reply_to_message_id: Some(message.message_id),
                                    reply_markup: None,
                                },
                            )]
                        }
                        Err(GameLogicError::AlreadyJoined) => {
                            vec![message_action::MessageAction::Send(
                                message_action::MessageInfo {
                                    chat_id: message.chat.id,
                                    text: "You have joined already :)".to_string(),
                                    message_id: None,
                                    reply_to_message_id: Some(message.message_id),
                                    reply_markup: None,
                                },
                            )]
                        }
                        Err(_) => vec![],
                    }
                }
                "/play" | "/play@piiigdicegamebot" => match self.play() {
                    Ok(current_player) => {
                        vec![message_action::MessageAction::Send(
                            message_action::MessageInfo {
                                chat_id: message.chat.id,
                                text: format!(
                                    "Started successfully. Turn: {}",
                                    current_player.get_mention_string()
                                ),
                                message_id: None,
                                reply_to_message_id: Some(message.message_id),
                                reply_markup: None,
                            },
                        )]
                    }
                    Err(GameLogicError::AlreadyPlaying) => {
                        vec![message_action::MessageAction::Send(
                            message_action::MessageInfo {
                                chat_id: message.chat.id,
                                text: "Game is already started :(".to_string(),
                                message_id: None,
                                reply_to_message_id: Some(message.message_id),
                                reply_markup: None,
                            },
                        )]
                    }
                    Err(GameLogicError::NotEnoughPlayers) => {
                        vec![message_action::MessageAction::Send(
                            message_action::MessageInfo {
                                chat_id: message.chat.id,
                                text: "Not enough players joined yet :(".to_string(),
                                message_id: None,
                                reply_to_message_id: Some(message.message_id),
                                reply_markup: None,
                            },
                        )]
                    }
                    Err(_) => vec![],
                },
                "/hold" | "/hold@piiigdicegamebot" => match self.hold(sender.id) {
                    Ok((score, current_player)) => {
                        vec![message_action::MessageAction::Send(
                            message_action::MessageInfo {
                                chat_id: message.chat.id,
                                text: format!(
                                    "Your score is {}. Turn: {}",
                                    score,
                                    current_player.get_mention_string()
                                ),
                                message_id: None,
                                reply_to_message_id: Some(message.message_id),
                                reply_markup: None,
                            },
                        )]
                    }
                    Err(GameLogicError::IsNotPlaying) => {
                        vec![message_action::MessageAction::Send(
                            message_action::MessageInfo {
                                chat_id: message.chat.id,
                                text: "Game is not started yet :(".to_string(),
                                message_id: None,
                                reply_to_message_id: Some(message.message_id),
                                reply_markup: None,
                            },
                        )]
                    }
                    Err(GameLogicError::WrongTurn) => {
                        vec![message_action::MessageAction::Send(
                            message_action::MessageInfo {
                                chat_id: message.chat.id,
                                text: "This is not your turn :(".to_string(),
                                message_id: None,
                                reply_to_message_id: Some(message.message_id),
                                reply_markup: None,
                            },
                        )]
                    }
                    Err(_) => vec![],
                },
                "/result" | "/result@piiigdicegamebot" => {
                    vec![self.send_results(message.chat.id)]
                }
                "/reset" | "/reset@piiigdicegamebot" => {
                    vec![message_action::MessageAction::Send(
                        message_action::MessageInfo {
                            chat_id: message.chat.id,
                            text: "Are you sure?".to_string(),
                            message_id: None,
                            reply_to_message_id: Some(message.message_id),
                            reply_markup: Some(telegram_types::ReplyMarkup {
                                inline_keyboard: Some(vec![vec![
                                    telegram_types::InlineKeyboardButton {
                                        text: "Yes".to_string(),
                                        callback_data: Some("reset".to_string()),
                                    },
                                ]]),
                            }),
                        },
                    )]
                }
                _ => vec![],
            }
        } else {
            vec![]
        }
    }

    pub fn handle_callback_query(
        &mut self,
        message: &telegram_types::Message,
        data: Option<String>,
    ) -> Vec<message_action::MessageAction> {
        if let Some(command) = data {
            if command.as_str() == "reset" {
                self.reset();

                vec![message_action::MessageAction::Edit(
                    message_action::MessageInfo {
                        chat_id: message.chat.id,
                        text: "Game is reset (players should join again).".to_string(),
                        message_id: Some(message.message_id),
                        reply_to_message_id: None,
                        reply_markup: Some(telegram_types::ReplyMarkup {
                            inline_keyboard: Some(vec![vec![]]),
                        }),
                    },
                )]
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }
}
