use super::*;
use rand::seq::SliceRandom;

#[derive(Clone)]
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
            Some(username) => format!("{}({})", self.name, username),
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

enum AddDiceResult {
    Finished,
    TurnLost(Player),
    Continue(Player, u8),
}

pub struct NewGame {
    players: HashMap<i64, Player>,
}

impl NewGame {
    pub fn new() -> NewGame {
        NewGame {
            players: HashMap::new(),
        }
    }

    async fn send_players(&self, bot_token: &str, chat_id: i64) {
        let text = if self.players.is_empty() {
            "No players!".to_string()
        } else {
            let players_text = self.players.values().fold("".to_string(), |res, player| {
                format!("{}\n- {}", res, player.show(false))
            });
            format!("Players:{}", players_text)
        };
        send_message(bot_token, chat_id, text, None).await
    }
}

pub struct PlayingGame {
    players: Vec<Player>,
    turn: u8,
    current_score: u8,
}

impl PlayingGame {
    fn from(new_game: &NewGame) -> PlayingGame {
        let mut players: Vec<Player> = new_game.players.values().cloned().collect();
        let mut rng = rand::thread_rng();
        players.shuffle(&mut rng);
        PlayingGame {
            players,
            turn: 0,
            current_score: 0,
        }
    }

    fn get_current_player_mut(&mut self) -> &mut Player {
        self.players.get_mut(self.turn as usize).unwrap()
    }

    fn get_current_player(&self) -> &Player {
        self.players.get(self.turn as usize).unwrap()
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

    async fn send_results(&self, bot_token: &str, chat_id: i64) {
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
        send_message(bot_token, chat_id, format!("Scores:{}", players_text), None).await
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

    fn play(&mut self) -> Result<Player, GameLogicError> {
        match self {
            GameState::New(new_game) => {
                if new_game.players.len() >= 2 {
                    let playing_game = PlayingGame::from(new_game);
                    let current_player = playing_game.get_current_player().clone();
                    *self = GameState::Playing(playing_game);
                    Ok(current_player)
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

    fn add_dice(&mut self, user_id: i64, value: u8) -> Result<AddDiceResult, GameLogicError> {
        match self {
            GameState::Playing(playing_game) => {
                playing_game.check_turn(user_id)?;
                if value == 1 {
                    playing_game.advance_turn();
                    Ok(AddDiceResult::TurnLost(
                        playing_game.get_current_player().clone(),
                    ))
                } else {
                    playing_game.current_score += value;
                    if playing_game.get_current_player().score + playing_game.current_score >= 100 {
                        playing_game.get_current_player_mut().score += playing_game.current_score;
                        playing_game.current_score = 0;
                        Ok(AddDiceResult::Finished)
                    } else {
                        Ok(AddDiceResult::Continue(
                            playing_game.get_current_player().clone(),
                            playing_game.current_score,
                        ))
                    }
                }
            }
            GameState::New(_) => Err(GameLogicError::IsNotPlaying),
        }
    }

    fn hold(&mut self, user_id: i64) -> Result<(u8, Player), GameLogicError> {
        match self {
            GameState::Playing(playing_game) => {
                playing_game.check_turn(user_id)?;
                playing_game.get_current_player_mut().score += playing_game.current_score;
                let result = playing_game.get_current_player().score;
                playing_game.advance_turn();
                Ok((result, playing_game.get_current_player().clone()))
            }
            GameState::New(_) => Err(GameLogicError::IsNotPlaying),
        }
    }

    async fn send_results(&self, bot_token: &str, chat_id: i64) {
        match self {
            GameState::New(new_game) => new_game.send_players(bot_token, chat_id).await,
            GameState::Playing(playing_game) => playing_game.send_results(bot_token, chat_id).await,
        }
    }

    pub async fn handle_dice(
        &mut self,
        bot_token: &str,
        message: &telegram_types::Message,
        dice_value: u8,
    ) {
        if let Some(sender) = &message.from {
            match self.add_dice(sender.id, dice_value) {
                Ok(AddDiceResult::Finished) => {
                    self.send_results(bot_token, message.chat.id).await;
                    self.reset();
                }
                Ok(AddDiceResult::TurnLost(current_player)) => {
                    send_message(
                        bot_token,
                        message.chat.id,
                        "Oops!".to_string(),
                        Some(message.message_id),
                    )
                    .await;
                    send_message(
                        bot_token,
                        message.chat.id,
                        format!("Your turn: {}", current_player.get_mention_string()),
                        None,
                    )
                    .await;
                }
                Ok(AddDiceResult::Continue(current_player, current_score)) => {
                    send_message(
                        bot_token,
                        message.chat.id,
                        format!(
                            "{} + {} = {}",
                            current_player.score,
                            current_score,
                            current_player.score + current_score
                        ),
                        Some(message.message_id),
                    )
                    .await
                }
                Err(_) => (),
            }
        }
    }
    pub async fn handle_command(
        &mut self,
        bot_token: &str,
        message: &telegram_types::Message,
        command: &str,
    ) {
        if let Some(sender) = &message.from {
            match command {
                "/join" | "/join@piiigdicegamebot" => {
                    match self.join(
                        sender.id,
                        sender.username.clone(),
                        sender.first_name.clone(),
                    ) {
                        Ok(_) => {
                            send_message(
                                bot_token,
                                message.chat.id,
                                "Joined successfully ;)".to_string(),
                                Some(message.message_id),
                            )
                            .await
                        }
                        Err(GameLogicError::JoinAfterPlay) => {
                            send_message(
                                bot_token,
                                message.chat.id,
                                "Game is already started :(".to_string(),
                                Some(message.message_id),
                            )
                            .await
                        }
                        Err(GameLogicError::AlreadyJoined) => {
                            send_message(
                                bot_token,
                                message.chat.id,
                                "You have joined already :)".to_string(),
                                Some(message.message_id),
                            )
                            .await
                        }
                        Err(_) => (),
                    }
                }
                "/play" | "/play@piiigdicegamebot" => match self.play() {
                    Ok(current_player) => {
                        send_message(
                            bot_token,
                            message.chat.id,
                            format!(
                                "Started successfully. Turn: {}",
                                current_player.get_mention_string()
                            ),
                            Some(message.message_id),
                        )
                        .await
                    }
                    Err(GameLogicError::AlreadyPlaying) => {
                        send_message(
                            bot_token,
                            message.chat.id,
                            "Game is already started :(".to_string(),
                            Some(message.message_id),
                        )
                        .await
                    }
                    Err(GameLogicError::NotEnoughPlayers) => {
                        send_message(
                            bot_token,
                            message.chat.id,
                            "Not enough players joined yet :(".to_string(),
                            Some(message.message_id),
                        )
                        .await
                    }
                    Err(_) => (),
                },
                "/hold" | "/hold@piiigdicegamebot" => match self.hold(sender.id) {
                    Ok((score, current_player)) => {
                        send_message(
                            bot_token,
                            message.chat.id,
                            format!(
                                "Your score is {}. Turn: {}",
                                score,
                                current_player.get_mention_string()
                            ),
                            Some(message.message_id),
                        )
                        .await
                    }
                    Err(GameLogicError::IsNotPlaying) => {
                        send_message(
                            bot_token,
                            message.chat.id,
                            "Game is not started yet :(".to_string(),
                            Some(message.message_id),
                        )
                        .await
                    }
                    Err(GameLogicError::WrongTurn) => {
                        send_message(
                            bot_token,
                            message.chat.id,
                            "This is not your turn :(".to_string(),
                            Some(message.message_id),
                        )
                        .await
                    }
                    Err(_) => (),
                },
                "/result" | "/result@piiigdicegamebot" => {
                    self.send_results(bot_token, message.chat.id).await
                }
                "/reset" | "/reset@piiigdicegamebot" => {
                    self.reset();
                    send_message(
                        bot_token,
                        message.chat.id,
                        "Game is reset (players should join again).".to_string(),
                        Some(message.message_id),
                    )
                    .await
                }
                _ => (),
            }
        }
    }
}
