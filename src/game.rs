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
    SetUsername(String),
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
            ("set_username", name) => UserAction::SetUsername(name.to_string()),
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
    Error(String),
    GameCreated(u32),
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
        {
            println!("[join] aquiring join lock");
            user.write().unwrap().game_id = Some(self.id);
            println!("[join] aquired join lock");
        }
        println!("[join] read user name lock");
        self.broadcast_message(ServerMessage::UserJoin(user.read().unwrap().name.clone()));
        self.players.iter().for_each(|player| {
            addr.do_send(ServerMessage::UserJoin(player.read().unwrap().name.clone()));
        });
        self.players.push(user.clone());
    }

    fn leave_game(&mut self, user: Arc<RwLock<User>>) {
        let user_id = user.read().unwrap().id;
        {
            println!("[leave] read user lock 2");
            let name = user.read().unwrap().name.clone();
            println!("[leave] read user lock 3");
            self.broadcast_message(ServerMessage::UserLeave(name));
            self.players
                .retain(|player| player.read().unwrap().id != user_id);
            println!("[leave] removed user {}", user_id);
        }
        drop(user_id);
        println!("[leave] before write user lock 4");
        user.write().unwrap().game_id = None;
        println!("[leave] after write user lock 4");
    }

    fn broadcast_message(&self, msg: ServerMessage) {
        self.players.iter().for_each(|user| {
            println!("broadcast lock");
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
    println!("read user lock for addr");
    let user_addr = user.read().unwrap().ws.as_ref().unwrap().clone();

    // print!("write user lock");
    // let w = user.write().unwrap().name.clone();
    // println!("write user lock");

    let ack = || user_addr.do_send(ServerMessage::ServerAck);

    match UserAction::from((action, conent)) {
        UserAction::SetUsername(name) => {
            user.write().unwrap().name = name.replace("\"", "");
            ack();
        }
        UserAction::NewGame => {
            let mut game = Game::new();
            let game_id = game.id;
            game.join_game(user.clone(), user_addr.clone());
            GAMES.write().unwrap().insert(game.id, game);
            println!("read user lock");
            user.read()
                .unwrap()
                .ws
                .as_ref()
                .unwrap()
                .do_send(ServerMessage::GameCreated(game_id));
        }
        UserAction::JoinGame(room_id) => {
            let mut games = GAMES.write().unwrap();
            println!("read user lock 1");
            let opt_room_id = user.read().unwrap().game_id;
            if let Some(room_id) = opt_room_id {
                if let Some(game) = games.get_mut(&room_id) {
                    game.leave_game(user.clone());
                }
            }
            match games.get_mut(&room_id) {
                Some(game) => game.join_game(user.clone(), user_addr),
                None => user_addr.do_send(ServerMessage::GameNotFound),
            };
            println!("joined")
        }
        UserAction::LeaveGame => {
            println!("read user lock");
            if let Some(game_id) = user.read().unwrap().game_id {
                let mut games = GAMES.write().unwrap();
                if let Some(game) = games.get_mut(&game_id) {
                    game.leave_game(user.clone());
                }
            }
        }
        _ => {}
    }
}
