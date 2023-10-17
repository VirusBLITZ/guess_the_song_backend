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

static GAMES: Lazy<RwLock<HashMap<u16, Game>>> = Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Message)]
#[rtype(result = "()")]
pub enum UserAction {
    SetUsername(String),
    NewGame,
    JoinGame(u16),
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
            ("set_username", name) => UserAction::SetUsername(name.to_string()),
            ("new", _) => UserAction::NewGame,
            ("join", game_id) => UserAction::JoinGame(game_id.parse().unwrap_or(0)),
            ("ready_up", _) => UserAction::ReadyUp,
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
    Error(String),
    GameCreated(u16),
    GameNotFound,
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
    Lobby(Vec<(&'a User, bool)>),
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
    pub id: u16,
    pub players: Vec<Arc<RwLock<User>>>,
    pub state: GameStatus<'a>,
}

impl Game<'_> {
    fn new() -> Self {
        Self {
            id: rand::random(),
            players: Vec::new(),
            state: GameStatus::Lobby(Vec::new()),
        }
    }

    fn join_game(&mut self, user: Arc<RwLock<User>>, addr: Addr<UserSocket>) {
        user.write().unwrap().game_id = Some(self.id);

        self.broadcast_message(ServerMessage::UserJoin(user.read().unwrap().name.clone()));
        self.players.iter().for_each(|player| {
            addr.do_send(ServerMessage::UserJoin(player.read().unwrap().name.clone()));
        });
        self.players.push(user.clone());
    }

    fn leave_game(&mut self, user: Arc<RwLock<User>>) {
        let user_id = user.read().unwrap().id;

        let name = user.read().unwrap().name.clone();
        self.broadcast_message(ServerMessage::UserLeave(name));
        self.players
            .retain(|player| player.read().unwrap().id != user_id);

        println!("[leave] before write user lock 4");
        user.write().unwrap().game_id = None;
        println!("[leave] after write user lock 4");
    }

    fn broadcast_message(&self, msg: ServerMessage) {
        self.players.iter().for_each(|user| {
            if let Some(ws) = user.read().unwrap().ws.as_ref() {
                ws.do_send(msg.clone());
            };
        });
    }

    fn ready(&self, user: std::sync::RwLockReadGuard<'_, User>) -> ServerMessage {
        let name = user.name.clone();
        self.broadcast_message(ServerMessage::UserReady(name));
        match &self.state {
            GameStatus::Lobby(ready_states) => {
                if !ready_states.iter().all(|(_, ready)| *ready) {
                    return ServerMessage::ServerAck;
                }
                ServerMessage::GameStartAt(
                    (std::time::UNIX_EPOCH.elapsed().unwrap() + std::time::Duration::from_secs(5))
                        .as_millis(),
                )
            }
            _ => ServerMessage::Error("cannot ready up: game is not in lobby state".into()),
        }
        // ServerMessage::ServerAck
    }
}

#[derive(Default)]
pub struct GamesState {
    pub games: Mutex<HashMap<u32, String>>,
}

pub fn handle_user_msg(action: UserAction, user: Arc<RwLock<User>>) {
    let user_addr = user.read().unwrap().ws.as_ref().unwrap().clone();

    // print!("write user lock");
    // let w = user.write().unwrap().name.clone();
    // println!("write user lock");

    let ack = || user_addr.do_send(ServerMessage::ServerAck);
    let leave_current = || {
        let mut games = GAMES.write().unwrap();
        let opt_prev_room_id = user.read().unwrap().game_id;
        if let Some(room_id) = opt_prev_room_id {
            if let Some(game) = games.get_mut(&room_id) {
                game.leave_game(user.clone());
            }
        }
    };

    match action {
        UserAction::SetUsername(name) => {
            user.write().unwrap().name = name.replace("\"", "");
            ack();
        }
        UserAction::NewGame => {
            leave_current();
            let mut game = Game::new();
            let game_id = game.id;
            game.join_game(user.clone(), user_addr.clone());
            GAMES.write().unwrap().insert(game.id, game);
            user.read()
                .unwrap()
                .ws
                .as_ref()
                .unwrap()
                .do_send(ServerMessage::GameCreated(game_id));
        }
        UserAction::JoinGame(room_id) => {
            leave_current();
            let mut games = GAMES.write().unwrap();
            match games.get_mut(&room_id) {
                Some(game) => game.join_game(user.clone(), user_addr),
                None => user_addr.do_send(ServerMessage::GameNotFound),
            };
            println!("joined room")
        }
        UserAction::ReadyUp => {
            let user = user.read().unwrap();
            match user.game_id {
                Some(game_id) => {
                    if let Some(game) = GAMES.write().unwrap().get_mut(&game_id) {
                        user_addr.do_send(game.ready(user));
                    }
                }
                None => user.ws.as_ref().unwrap().do_send(ServerMessage::Error(
                    "cannot ready up: not in a game".into(),
                )),
            }
        }
        UserAction::LeaveGame => {
            leave_current();
            // if let Some(game_id) = user.read().unwrap().game_id {
            //     let mut games = GAMES.write().unwrap();
            //     if let Some(game) = games.get_mut(&game_id) {
            //         game.leave_game(user.clone());
            //     }
            // }
        }
        _ => user_addr.do_send(ServerMessage::Error("Invalid Action".to_string())),
    }
}
