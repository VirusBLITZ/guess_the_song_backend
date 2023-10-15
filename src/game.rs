use std::{
    collections::HashMap,
    sync::{Mutex, RwLock},
};

use actix::Message;
use once_cell::sync::Lazy;

use crate::model::{song::Song, user::User};

static GAMES: Lazy<RwLock<HashMap<u32, Game>>> = Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Message)]
#[rtype(result = "()")]
pub enum UserAction {
    NewGame,
    JoinGame,
    ReadyUp,
    StartGame,
    AddSong(String),
    StartGuessing,
    GuessSong(u8),
    LeaveGame,
    InvalidAction,
}

impl From<(&str, &str)> for UserAction {
    fn from(value: (&str, &str)) -> Self {
        match value {
            ("new", _) => UserAction::NewGame,
            ("join", _) => UserAction::JoinGame,
            ("ready", _) => UserAction::ReadyUp,
            ("start", _) => UserAction::StartGame,
            ("add", id) => UserAction::AddSong(id.to_string()),
            ("start_guessing", _) => UserAction::StartGuessing,
            ("guess", idx) => UserAction::GuessSong(idx.parse().unwrap_or(0)),
            ("leave", _) => UserAction::LeaveGame,
            _ => UserAction::InvalidAction,
        }
    }
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub enum ServerMessage {
    GameCreated(u32),
    // lobby
    UserJoin(String),
    UserLeave(String),
    UserReady(String),
    UserUnready(String),
    GameStartAt(u128),
    // song selection
    GameStartSelect,
    Suggestion(Vec<invidious::hidden::SearchItem>),
}

#[derive(Clone)]
pub enum GameStatus<'a> {
    Lobby,
    Playing(Vec<Song>, PlayPhase<'a>),
    Ended,
}

#[derive(Clone)]
pub enum PlayPhase<'a> {
    SelectingSongs,
    GuessingSongs(Vec<&'a User>),
}

#[derive(Clone)]
pub struct Game<'a> {
    pub id: u32,
    pub players: Vec<User>,
    pub state: GameStatus<'a>,
}

impl Game<'_> {
    fn new() -> Self {
        Self {
            id: rand::random(),
            players: Vec::new(),
            state: GameStatus::Lobby,
        }
    }

    fn join_game(&mut self, user: User) {
        self.players.push(user);
        self.players.iter().for_each(|user| {
            if let Some(ws) = &user.ws {
                ws.do_send(ServerMessage::UserJoin(user.name.clone()));
            };
        });
    }
}

#[derive(Default)]
pub struct GamesState {
    pub games: Mutex<HashMap<u32, String>>,
}

pub fn handle_user_msg(action: &str, conent: &str, user: &User) {
    match UserAction::from((action, conent)) {
        UserAction::NewGame => {
            let game = Game::new();
            let game_id = game.id;
            GAMES.write().unwrap().insert(game.id, game);
            user.ws
                .as_ref()
                .unwrap()
                .do_send(ServerMessage::GameCreated(game_id));
        }
        // UserAction::JoinGame => {
        //     let game = unsafe { GAMES.get_mut(&0).unwrap() };
        //     game.join_game(user);
        // }
        _ => {}
    }
}
