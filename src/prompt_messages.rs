const DEFAULT_SYSTEM_MESSAGE: &str = "\
    You are a Persian translator. \
    Write up to 3 sentences but keep it short. \
    Use informal language. \
    Free to use emojis.";

pub fn system_message(extra_info: &Option<String>) -> String {
    match extra_info {
        Some(extra_info) => format!("{}\n(Extra info: {})", DEFAULT_SYSTEM_MESSAGE, extra_info),
        None => DEFAULT_SYSTEM_MESSAGE.into(),
    }
}

pub const fn greeting() -> &'static str {
    "Add this bot to groups to enjoy the Pig (dice) game!"
}

pub fn greeting_hint(name: &String) -> String {
    format!("Audience name is {}.", name)
}

pub const fn join_after_play() -> &'static str {
    "You can't join when game is started! Wait for next round ;)"
}
pub const fn already_joined() -> &'static str {
    "You have joined already, you can't do it again :)"
}
pub const fn game_already_started() -> &'static str {
    "Game is already started :("
}
pub const fn not_enough_player() -> &'static str {
    "Not enough players joined yet :("
}
pub const fn game_is_not_started() -> &'static str {
    "Game is not started yet :("
}
pub const fn not_your_turn() -> &'static str {
    "This is not your turn :("
}

pub fn game_logic_error_hint(name: &String) -> String {
    format!("Audience name is {}.", name)
}

pub const fn player_list_hint() -> &'static str {
    "\
    List of the players who joined the game provided.\
    Each row contains the name and username in the parenthesis."
}

pub const fn result_hint() -> &'static str {
    "\
    List of the players in the game and their achieved points provided. \
    The game is a Pig dice game. \
    The player with king emoji (if exists) is the winner, \
    say congratulations to the winner (if exists). \
    The one with dice emoji (if exists) is the current player who possesses \
    the turn to roll the dice. \
    Say your opinion about the current state of the game."
}

pub const fn turn_lost() -> &'static str {
    "Oops! You lost your turn :("
}

pub fn turn_lost_hint(name: &String, last_score: u8) -> String {
    format!(
        "\
        {} lost the turn after rolling a \"one\" by the dice. \
        The game is a Pig dice game and the player lost the turn after adding {} by \
        the previous rolled dice results during the turn. \
        Say your opinion about the player's performance during the last turn and \
        how lucky the player was.",
        name, last_score
    )
}

pub fn next_turn(mention_string: &String) -> String {
    format!("It's {} turn to roll the dice.", mention_string)
}

pub fn next_turn_hint(name: &String) -> String {
    format!(
        "\
        The game is a Pig dice game and \
        now it's {} turn to roll the dice. \
        Don't forget to mention the player's username.",
        name
    )
}

pub const fn joined() -> &'static str {
    "You joined the game successfully!"
}

pub fn joined_hint(name: &String) -> String {
    format!(
        "\
        The game is a Pig dice game and \
        {} joined the game.",
        name
    )
}

pub fn started(mention_string: &String) -> String {
    format!("The game has just started. Turn: {}.", mention_string)
}

pub fn started_hint(name: &String) -> String {
    format!(
        "\
        The game is a Pig dice game. \
        Game has just started. \
        {} is the first player to roll the dice.",
        name
    )
}

pub fn hold(score: u8, mention_string: &String) -> String {
    format!(
        "Your total score is {}. Next turn: {}",
        score, mention_string
    )
}

pub fn hold_hint(name: &String, score: u8) -> String {
    format!(
        "\
        The game is a Pig dice game. \
        {} decided to hold their achieved points and pass the dice \
        to the next player. \
        The player achieved {} points during the turn. \
        Tell your opinion about this decision. \
        Also, don't forget to mention the next player username.",
        name, score
    )
}

pub const fn reset_confirm() -> &'static str {
    "Are you sure?"
}

pub fn reset_confirm_hint(name: &String) -> String {
    format!("{} wants to reset the game.", name)
}

pub const fn reset() -> &'static str {
    "Game is reset (players should join again)."
}

pub const fn reset_hint() -> &'static str {
    "The game is a Pig dice game and it's reset."
}
