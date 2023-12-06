use rand::Rng;

use crate::{
    message_action::MessageAction,
    telegram_types::{MessageId, User, UserId},
};

use super::GameState;

fn get_random_id() -> i64 {
    let mut rng = rand::thread_rng();
    rng.gen()
}

fn get_user(user_number: i64) -> User {
    User {
        id: UserId(user_number),
        first_name: format!("Name{}", user_number),
        last_name: None,
        username: None,
    }
}

#[test]
fn player_join() {
    let message_id = MessageId(get_random_id());
    let mut game_state = GameState::new();
    let actions = game_state.handle_command(message_id, &get_user(0), "/join");

    assert_eq!(1, actions.len());
    if let MessageAction::Send(info) = &actions[0] {
        assert_eq!(message_id, info.reply_to_message_id.unwrap());
        assert!(info.text.contains("Joined"));
    } else {
        panic!("Message action is not Send");
    }
}
