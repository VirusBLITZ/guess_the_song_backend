use std::{
    collections::HashMap,
    sync::{Arc, RwLock, RwLockReadGuard},
    thread,
    time::Duration,
};

use actix::{Addr, Message};
use once_cell::sync::Lazy;

use crate::{
    model::{song::Song, user::User},
    music_handler, UserSocket,
};

static GAMES: Lazy<RwLock<HashMap<u16, Game>>> = Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Message)]
#[rtype(result = "()")]
pub enum UserAction {
    SetUsername(String),
    NewGame,
    JoinGame(u16),
    ReadyUp,
    Unready,
    StartGame,
    GetSuggestions(String),
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
            ("unready", _) => UserAction::Unready,
            ("start", _) => UserAction::StartGame,
            ("suggest", query) => UserAction::GetSuggestions(query.to_string()),
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

#[derive(Clone, Debug)]
pub enum GameStatus<'a> {
    Lobby(u8), // ready count
    Playing(Vec<Song>, PlayPhase<'a>),
    Ended,
}

#[derive(Clone, Debug)]
pub enum PlayPhase<'a> {
    SelectingSongs,
    GuessingSongs(Vec<&'a User>),  // leaderboard
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
            state: GameStatus::Lobby(0),
        }
    }

    fn set_state(&mut self, state: GameStatus<'static>) {
        println!("[GAME {}] now in state {:?}", self.id, state);
        self.state = state;
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

    fn ready(&mut self, user: RwLockReadGuard<'_, User>) -> ServerMessage {
        let name = user.name.clone();
        self.broadcast_message(ServerMessage::UserReady(name));
        match &mut self.state {
            GameStatus::Lobby(ready_count) => {
                *ready_count += 1;
                if *ready_count as usize != self.players.len() {
                    return ServerMessage::ServerAck;
                }
                // announce game start
                #[cfg(debug_assertions)]
                static START_TIMEOUT: Duration = Duration::from_secs(2);
                #[cfg(not(debug_assertions))]
                static START_TIMEOUT: Duration = Duration::from_secs(12);
                self.broadcast_message(ServerMessage::GameStartAt(
                    (std::time::UNIX_EPOCH
                        .elapsed()
                        .expect("system to provide elapsed UNIX time")
                        + START_TIMEOUT)
                        .as_millis(),
                ));

                let game_id = self.id;
                thread::spawn(move || {
                    thread::sleep(START_TIMEOUT);
                    if let Some(game) = GAMES.write().unwrap().get_mut(&game_id) {
                        if let GameStatus::Lobby(ready_count) = &mut game.state {
                            if (*ready_count as usize) < game.players.len() {
                                return;
                            }
                        }
                        game.start_game();
                    }
                })
            }
            _ => return ServerMessage::Error("cannot ready up: game is not in lobby state".into()),
        };
        ServerMessage::ServerAck
    }

    fn unready(&mut self, user: RwLockReadGuard<'_, User>) -> ServerMessage {
        match &mut self.state {
            GameStatus::Lobby(ready_count) => {
                if *ready_count == 0 {
                    return ServerMessage::Error("cannot unready: no one is ready".into());
                }
                *ready_count -= 1;
                self.broadcast_message(ServerMessage::UserUnready(user.name.clone()));
                ServerMessage::ServerAck
            }
            _ => ServerMessage::Error("cannot unready: game is not in lobby state".into()),
        }
    }

    fn start_game(&mut self) {
        // self.state = GameStatus::Playing(Vec::new(), PlayPhase::SelectingSongs);
        self.set_state(GameStatus::Playing(Vec::new(), PlayPhase::SelectingSongs));
        self.broadcast_message(ServerMessage::GameStartSelect);
    }
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
            user.write().unwrap().name = name.trim_matches('"').to_string();
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
            let mut games: std::sync::RwLockWriteGuard<'_, HashMap<u16, Game<'_>>> =
                GAMES.write().unwrap();
            match games.get_mut(&room_id) {
                Some(game) => {
                    game.join_game(user.clone(), user_addr);
                    println!("joined room")
                }
                None => user_addr.do_send(ServerMessage::GameNotFound),
            };
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
        UserAction::Unready => {
            let user = user.read().unwrap();
            match user.game_id {
                Some(game_id) => {
                    if let Some(game) = GAMES.write().unwrap().get_mut(&game_id) {
                        user_addr.do_send(game.unready(user));
                    }
                }
                None => user
                    .ws
                    .as_ref()
                    .unwrap()
                    .do_send(ServerMessage::Error("cannot unready: not in a game".into())),
            }
        }
        UserAction::LeaveGame => {
            leave_current();
            // DEADLOCK:
            // if let Some(game_id) = user.read().unwrap().game_id {
            //     let mut games = GAMES.write().unwrap();
            //     if let Some(game) = games.get_mut(&game_id) {
            //         game.leave_game(user.clone());
            //     }
            // }
        }
        UserAction::GetSuggestions(query) => {
            user_addr.do_send(match music_handler::get_suggestions(&query) {
                Ok(songs) => ServerMessage::Suggestion(songs),
                Err(err) => ServerMessage::Error(err.to_string()),
            });
        }
        UserAction::AddSong(source_id) => {
            let user_addr = user_addr.clone();
            thread::spawn(move || {
                let songs = music_handler::songs_from_id(source_id.as_str()).unwrap();
                // if let Err(err) = songs {
                //     user_addr.do_send(ServerMessage::Error(format!("{:?}", err)));
                //     return;
                // }
                // let songs = songs.unwrap();
                let game_id = user.read().unwrap().game_id;
                {
                    let mut games = GAMES.write().unwrap();
                    match game_id {
                        Some(game_id) => {
                            if let Some(game) = games.get_mut(&game_id) {
                                match &mut game.state {
                                    GameStatus::Playing(game_songs, PlayPhase::SelectingSongs) => {
                                        game_songs.extend(dbg!(songs));
                                    }
                                    _ => user_addr.do_send(ServerMessage::Error(
                                        "cannot add song(s): game is not in song selection state"
                                            .into(),
                                    )),
                                }
                            }

                            // match games.get_mut(&game_id) {
                            // Some(game) => match game.state.as_ref() {
                            //     GameStatus::Playing(mut songs, PlayPhase::SelectingSongs) => {
                            //         songs.extend(songs);
                            //     }
                            //     _ => user_addr.do_send(ServerMessage::Error(
                            //         "cannot add song(s): game is not in song selection state".into(),
                            //     )),
                            // }
                            // None => user_addr.do_send(ServerMessage::Error(
                            //     "cannot add song(s): the game you're in doesn't exist".into(),
                            // )),
                        }

                        None => user_addr.do_send(ServerMessage::Error(
                            "cannot add song(s): not in a game".into(),
                        )),
                    }
                };
            });
            ack();
        }
        _ => user_addr.do_send(ServerMessage::Error("Invalid Action".to_string())),
    }
}
