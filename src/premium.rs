use std::{collections::HashSet, fs::File, sync::OnceLock};

use serde::Deserialize;

#[derive(Deserialize)]
struct Usernames {
    usernames: Vec<String>,
}

pub fn is_premium(username: String) -> bool {
    static USERNAMES: OnceLock<HashSet<String>> = OnceLock::new();
    USERNAMES
        .get_or_init(|| {
            let file_path = std::env::args()
                .nth(1)
                .expect("Premium users YAML file is not provided!");
            let file = File::open(file_path).unwrap();
            serde_yaml::from_reader::<_, Usernames>(file)
                .unwrap()
                .usernames
                .into_iter()
                .collect()
        })
        .contains(&username)
}
