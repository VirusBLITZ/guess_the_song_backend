use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};

use actix::{Addr, Message};
use once_cell::sync::Lazy;

use crate::{
    model::{song::Song, user::User},
    UserSocket,
};

static GAMES: Lazy<RwLock<HashMap<u32, Game>>> = Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Message)]
#[rtype(result = "()")]
pub enum UserAction {
    NewGame,
    JoinGame(u32),
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
            ("join", game_id) => UserAction::JoinGame(game_id.parse().unwrap_or(0)),
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

#[derive(Message, Debug, Clone)]
#[rtype(result = "()")]
pub enum ServerMessage {
    ServerAck,
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
    pub players: Vec<Arc<RwLock<User>>>,
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

    fn join_game(&mut self, user: Arc<RwLock<User>>, addr: Addr<UserSocket>) {
        self.broadcast_message(ServerMessage::UserJoin(user.read().unwrap().name.clone()));
        self.players.iter().for_each(|player| {
            addr.do_send(ServerMessage::UserJoin(player.read().unwrap().name.clone()));
        });
        self.players.push(user.clone());
    }

    fn broadcast_message(&self, msg: ServerMessage) {
        self.players.iter().for_each(|user| {
            if let Some(ws) = user.read().unwrap().ws.as_ref() {
                ws.do_send(msg.clone());
            };
        });
    }
}

#[derive(Default)]
pub struct GamesState {
    pub games: Mutex<HashMap<u32, String>>,
}

pub fn handle_user_msg(action: &str, conent: &str, user: Arc<RwLock<User>>) {
    let user_addr = user.read().unwrap().ws.as_ref().unwrap().clone();
    match UserAction::from((action, conent)) {
        UserAction::NewGame => {
            let mut game = Game::new();
            let game_id = game.id;
            game.players.push(user.clone());
            GAMES.write().unwrap().insert(game.id, game);
            user.read()
                .unwrap()
                .ws
                .as_ref()
                .unwrap()
                .do_send(ServerMessage::GameCreated(game_id));
        }
        UserAction::JoinGame(room_id) => {
            let mut games = GAMES.write().unwrap();
            let game = games.get_mut(&room_id).unwrap();
            game.join_game(user, user_addr);
        }
        _ => {}
    }
}
