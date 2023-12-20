use crate::prompt_messages::{
    already_joined, game_already_started, game_is_not_started, game_logic_error_hint, hold_hint,
    join_after_play, joined, joined_hint, next_turn, next_turn_hint, not_enough_player,
    not_your_turn, player_list_hint, reset, reset_confirm, reset_confirm_hint, reset_hint,
    result_hint, started, started_hint, turn_lost, turn_lost_hint,
};

use super::message_action;
use super::telegram_types;
use super::text_messages;
use rand::seq::SliceRandom;
use std::collections::HashMap;

struct Player {
    user_id: telegram_types::UserId,
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

impl GameLogicError {
    fn get_reply_message(
        &self,
        reply_to_message_id: telegram_types::MessageId,
        audience_name: String,
        is_premium: bool,
    ) -> message_action::MessageAction {
        let text = match self {
            Self::JoinAfterPlay => join_after_play(),
            Self::AlreadyJoined => already_joined(),
            Self::AlreadyPlaying => game_already_started(),
            Self::NotEnoughPlayers => not_enough_player(),
            Self::IsNotPlaying => game_is_not_started(),
            Self::WrongTurn => not_your_turn(),
        }
        .to_string();
        message_action::MessageAction::Send(message_action::MessageInfo {
            text,
            reply_to_message_id: Some(reply_to_message_id),
            reply_markup: None,
            hint: Some(game_logic_error_hint(&audience_name)),
            is_premium,
        })
    }
}

enum AddDiceResult<'a> {
    Finished,
    TurnLost(&'a Player, u8),
    Continue(&'a Player, u8),
}

#[derive(Default)]
pub struct NewGame {
    players: HashMap<telegram_types::UserId, Player>,
    is_premium: bool,
}

impl NewGame {
    pub fn new() -> NewGame {
        NewGame {
            players: HashMap::new(),
            is_premium: false,
        }
    }

    fn send_players(&self) -> message_action::MessageAction {
        let text = if self.players.is_empty() {
            "No players!".to_string()
        } else {
            let players_text = self.players.values().fold("".to_string(), |res, player| {
                format!("{}\n- {}", res, player.show(false))
            });
            format!("Players:{}", players_text)
        };
        message_action::MessageAction::Send(message_action::MessageInfo {
            text,
            reply_to_message_id: None,
            reply_markup: None,
            hint: Some(player_list_hint().to_string()),
            is_premium: self.is_premium,
        })
    }
}

pub struct PlayingGame {
    players: Vec<Player>,
    turn: u8,
    current_score: u8,
    is_premium: bool,
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
            is_premium: new_game.is_premium,
        }
    }

    fn get_current_player_mut(&mut self) -> &mut Player {
        &mut self.players[self.turn as usize]
    }

    fn get_current_player(&self) -> &Player {
        &self.players[self.turn as usize]
    }

    fn check_turn(&self, user_id: telegram_types::UserId) -> Result<(), GameLogicError> {
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

    fn send_results(&self) -> message_action::MessageAction {
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
            text: format!("Scores:{}", players_text),
            reply_to_message_id: None,
            reply_markup: None,
            hint: Some(result_hint().to_string()),
            is_premium: self.is_premium,
        })
    }
}

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
        user_id: telegram_types::UserId,
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
                        username: username.clone(),
                    });
                    if crate::premium::is_premium(username.unwrap_or_default()) {
                        new_game.is_premium = true;
                    }
                    Ok(())
                } else {
                    Err(GameLogicError::AlreadyJoined)
                }
            }
            GameState::Playing(_) => Err(GameLogicError::JoinAfterPlay),
        }
    }

    fn get_playing_game(&self) -> Result<&PlayingGame, GameLogicError> {
        match self {
            GameState::Playing(playing_game) => Ok(playing_game),
            _ => Err(GameLogicError::IsNotPlaying),
        }
    }

    fn get_playing_game_mut(&mut self) -> Result<&mut PlayingGame, GameLogicError> {
        match self {
            GameState::Playing(playing_game) => Ok(playing_game),
            _ => Err(GameLogicError::IsNotPlaying),
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

    fn add_dice(
        &mut self,
        user_id: telegram_types::UserId,
        value: u8,
    ) -> Result<AddDiceResult<'_>, GameLogicError> {
        let playing_game = self.get_playing_game_mut()?;
        playing_game.check_turn(user_id)?;
        if value == 1 {
            let last_score = playing_game.current_score;
            playing_game.advance_turn();
            Ok(AddDiceResult::TurnLost(
                playing_game.get_current_player(),
                last_score,
            ))
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

    fn hold(
        &mut self,
        user_id: telegram_types::UserId,
    ) -> Result<(u8, u8, &Player), GameLogicError> {
        let playing_game = self.get_playing_game_mut()?;
        playing_game.check_turn(user_id)?;
        playing_game.get_current_player_mut().score += playing_game.current_score;
        let result = playing_game.get_current_player().score;
        let turn_score = playing_game.current_score;
        playing_game.advance_turn();
        Ok((result, turn_score, playing_game.get_current_player()))
    }

    fn send_results(&self) -> message_action::MessageAction {
        match self {
            GameState::New(new_game) => new_game.send_players(),
            GameState::Playing(playing_game) => playing_game.send_results(),
        }
    }

    fn is_premium(&self) -> bool {
        match self {
            GameState::New(new_game) => new_game.is_premium,
            GameState::Playing(playing_game) => playing_game.is_premium,
        }
    }

    pub fn handle_dice(
        &mut self,
        message: &telegram_types::Message,
        dice_value: u8,
    ) -> Vec<message_action::MessageAction> {
        let is_premium = self.is_premium();
        if let Some(sender) = &message.from {
            match self.add_dice(sender.id, dice_value) {
                Ok(AddDiceResult::Finished) => {
                    let action = self.send_results();
                    self.reset();
                    vec![action]
                }
                Ok(AddDiceResult::TurnLost(current_player, last_score)) => {
                    vec![
                        message_action::MessageAction::Send(message_action::MessageInfo {
                            text: turn_lost().to_string(),
                            reply_to_message_id: Some(message.message_id),
                            reply_markup: None,
                            hint: Some(turn_lost_hint(&sender.first_name, last_score)),
                            is_premium,
                        }),
                        message_action::MessageAction::Send(message_action::MessageInfo {
                            text: next_turn(&current_player.get_mention_string()),
                            reply_to_message_id: None,
                            reply_markup: None,
                            hint: Some(next_turn_hint(&current_player.name)),
                            is_premium,
                        }),
                    ]
                }
                Ok(AddDiceResult::Continue(current_player, current_score)) => {
                    vec![message_action::MessageAction::Send(
                        message_action::MessageInfo {
                            text: format!(
                                "{} + {} = {}",
                                current_player.score,
                                current_score,
                                current_player.score + current_score,
                            ),
                            reply_to_message_id: Some(message.message_id),
                            reply_markup: None,
                            hint: None,
                            is_premium: false,
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
                                    text: joined().to_string(),
                                    reply_to_message_id: Some(message.message_id),
                                    reply_markup: None,
                                    hint: joined_hint(&sender.first_name).into(),
                                    is_premium: self.is_premium(),
                                },
                            )]
                        }
                        Err(err) => {
                            vec![err.get_reply_message(
                                message.message_id,
                                sender.first_name.clone(),
                                self.is_premium(),
                            )]
                        }
                    }
                }
                "/play" | "/play@piiigdicegamebot" => match self.play() {
                    Ok(current_player) => {
                        vec![message_action::MessageAction::Send(
                            message_action::MessageInfo {
                                text: started(&current_player.get_mention_string()),
                                reply_to_message_id: Some(message.message_id),
                                reply_markup: None,
                                hint: Some(started_hint(&current_player.name)),
                                is_premium: self.is_premium(),
                            },
                        )]
                    }
                    Err(err) => {
                        vec![err.get_reply_message(
                            message.message_id,
                            sender.first_name.clone(),
                            self.is_premium(),
                        )]
                    }
                },
                "/hold" | "/hold@piiigdicegamebot" => match self.hold(sender.id) {
                    Ok((total_score, turn_score, current_player)) => {
                        vec![message_action::MessageAction::Send(
                            message_action::MessageInfo {
                                text: crate::prompt_messages::hold(
                                    total_score,
                                    &current_player.get_mention_string(),
                                ),
                                reply_to_message_id: Some(message.message_id),
                                reply_markup: None,
                                hint: Some(hold_hint(&sender.first_name, turn_score, total_score)),
                                is_premium: self.is_premium(),
                            },
                        )]
                    }
                    Err(err) => {
                        vec![err.get_reply_message(
                            message.message_id,
                            sender.first_name.clone(),
                            self.is_premium(),
                        )]
                    }
                },
                "/result" | "/result@piiigdicegamebot" => {
                    vec![self.send_results()]
                }
                "/reset" | "/reset@piiigdicegamebot" => {
                    vec![message_action::MessageAction::Send(
                        message_action::MessageInfo {
                            text: reset_confirm().to_string(),
                            reply_to_message_id: Some(message.message_id),
                            reply_markup: Some(telegram_types::ReplyMarkup {
                                inline_keyboard: Some(vec![vec![
                                    telegram_types::InlineKeyboardButton {
                                        text: "Yes".to_string(),
                                        callback_data: Some("reset".to_string()),
                                    },
                                ]]),
                            }),
                            hint: Some(reset_confirm_hint(&sender.first_name)),
                            is_premium: self.is_premium(),
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
                let is_premium = self.is_premium();
                self.reset();

                vec![message_action::MessageAction::Edit(
                    message_action::EditMessageInfo {
                        message_id: message.message_id,
                        message_info: message_action::MessageInfo {
                            text: reset().to_string(),
                            reply_to_message_id: None,
                            reply_markup: Some(telegram_types::ReplyMarkup {
                                inline_keyboard: Some(vec![vec![]]),
                            }),
                            hint: Some(reset_hint().to_string()),
                            is_premium,
                        },
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
