enum GameStatus {
    Lobby,
    Playing(Vec<Song>, Phase),
    Ended,
}

enum Phase {
    SelectingSongs,
    GuessingSongs (Vec<&user::User>),
}

struct Game {
    pub id: u32,
    pub players: Vec<user::User>,
}
