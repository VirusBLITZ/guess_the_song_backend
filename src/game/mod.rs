mod guessing_songs;

use std::{
    collections::HashMap,
    sync::{mpsc::SyncSender, Arc, RwLock, RwLockReadGuard},
    thread,
    time::Duration,
};

use actix::{Addr, Message};
use once_cell::sync::Lazy;

use crate::{
    model::{song::Song, user::User},
    music_handler, UserSocket,
};

use self::guessing_songs::{handle_game_end, handle_guessing};

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
    RemoveSong(u32),
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
            ("remove", idx) => UserAction::RemoveSong(idx.parse().unwrap_or(0)),
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
    AddedSong(Song),
    RemovedSong(u32),
    // guessing
    GameStartGuessing,
    GamePlayAudio(String),
    GameGuessOptions(Vec<(String, String)>),

    LeaderBoard(Vec<(String, usize)>),
    Correct(u8),

    // restart => GameEnded (go back to lobby)
    GameEnded,
}

#[derive(Clone, Debug)]
pub enum GameStatus {
    Lobby(u8), // ready count
    Playing(PlayPhase),
}

#[derive(Clone, Debug)]
pub enum PlayPhase {
    SelectingSongs(HashMap<usize, Vec<Song>>),
    GuessingSongs(SyncSender<(Arc<RwLock<User>>, u8)>), // game thread sender
}

#[derive(Clone)]
pub struct Game {
    pub id: u16,
    pub players: Vec<Arc<RwLock<User>>>,
    pub state: GameStatus,
}

impl Game {
    fn new() -> Self {
        Self {
            id: rand::random(),
            players: Vec::new(),
            state: GameStatus::Lobby(0),
        }
    }

    fn set_state(&mut self, state: GameStatus) {
        println!("[GAME {}] now in state {:?}", self.id, state);
        self.state = state;
    }

    fn join_game(&mut self, user: Arc<RwLock<User>>, addr: Addr<UserSocket>) {
        user.write().unwrap().game_id = Some(self.id);

        if !matches!(self.state, GameStatus::Lobby(_)) {
            addr.do_send(ServerMessage::Error(
                "cannot join game: game is not in lobby state".into(),
            ));
            return;
        }

        self.broadcast_message(ServerMessage::UserJoin(user.read().unwrap().name.clone()));
        self.players.iter().for_each(|player| {
            addr.do_send(ServerMessage::UserJoin(player.read().unwrap().name.clone()));
        });
        self.players.push(user);
    }

    fn leave_game(&mut self, user: Arc<RwLock<User>>) -> bool {
        let user_id = user.read().unwrap().id;

        let name = user.read().unwrap().name.clone();
        self.broadcast_message(ServerMessage::UserLeave(name));
        self.players
            .retain(|player| player.read().unwrap().id != user_id);

        user.write().unwrap().game_id = None;

        self.players.is_empty()
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
                });
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
        self.set_state(GameStatus::Playing(PlayPhase::SelectingSongs(
            HashMap::new(),
        )));
        self.broadcast_message(ServerMessage::GameStartSelect);
    }

    fn start_guessing(&mut self, user: Arc<RwLock<User>>) -> Option<()> {
        let read_usr = user.read().unwrap();
        let user_addr = read_usr.ws.as_ref()?;
        match &mut self.state {
            GameStatus::Playing(playphase) => {
                if !Arc::ptr_eq(&user, &self.players[0]) {
                    user_addr.do_send(ServerMessage::Error(
                        "cannot start guessing: you are not the leader".into(),
                    ));
                    return None;
                }
                let songs = match playphase {
                    PlayPhase::SelectingSongs(songs) => songs,
                    _ => {
                        user_addr.do_send(ServerMessage::Error(
                            "cannot start guessing: game is not in song selection state".into(),
                        ));
                        return None;
                    }
                };
                let (tx, game_handle) = handle_guessing(self.players.clone(), songs);
                handle_game_end(game_handle, self.id);

                *playphase = PlayPhase::GuessingSongs(tx);
                self.broadcast_message(ServerMessage::GameStartGuessing);
            }
            _ => user_addr.do_send(ServerMessage::Error(
                "cannot start guessing: game is not in song selection state".into(),
            )),
        };
        None
    }
}

pub fn handle_user_msg(action: UserAction, user: Arc<RwLock<User>>) -> Option<()> {
    let user_addr = user.read().unwrap().ws.as_ref()?.to_owned();

    // print!("write user lock");
    // let w = user.write().unwrap().name.clone();
    // println!("write user lock");

    let user_ptr_addr = Arc::as_ptr(&user) as usize;
    let send_msg = |msg: ServerMessage| user_addr.do_send(msg);
    let ack = || send_msg(ServerMessage::ServerAck);
    let leave_current = || -> Option<()> {
        let mut games = GAMES.write().unwrap();
        let user_room_id = user.read().unwrap().game_id?;
        if games.get_mut(&user_room_id)?.leave_game(user.clone()) {
            games.remove(&user_room_id);
        }
        Some(())
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
            send_msg(ServerMessage::GameCreated(game_id));
        }
        UserAction::JoinGame(room_id) => {
            leave_current();
            let mut games = GAMES.write().unwrap();
            match games.get_mut(&room_id) {
                Some(game) => {
                    game.join_game(user.clone(), user_addr);
                    println!("joined room")
                }
                None => send_msg(ServerMessage::GameNotFound),
            };
        }
        UserAction::ReadyUp => {
            let user = user.read().unwrap();
            match user.game_id {
                Some(game_id) => {
                    if let Some(game) = GAMES.write().unwrap().get_mut(&game_id) {
                        send_msg(game.ready(user));
                    }
                }
                None => user.ws.as_ref()?.do_send(ServerMessage::Error(
                    "cannot ready up: not in a game".into(),
                )),
            }
        }
        UserAction::Unready => {
            let user = user.read().unwrap();
            match user.game_id {
                Some(game_id) => {
                    let mut games = GAMES.write().unwrap();
                    let game = games.get_mut(&game_id)?;
                    send_msg(game.unready(user));
                }
                None => user
                    .ws
                    .as_ref()?
                    .do_send(ServerMessage::Error("cannot unready: not in a game".into())),
            };
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
            send_msg(match music_handler::get_suggestions(&query) {
                Ok(songs) => ServerMessage::Suggestion(songs),
                Err(err) => ServerMessage::Error(err.to_string()),
            });
        }
        UserAction::AddSong(source_id) => {
            let read_user = user.read().unwrap();
            if read_user.game_id.is_none() {
                send_msg(ServerMessage::Error(
                    "cannot add song: not in a game".into(),
                ));
                return None;
            }
            let games = GAMES.read().unwrap();
            let game = games.get(&read_user.game_id?)?;
            if !matches!(
                &game.state,
                GameStatus::Playing(PlayPhase::SelectingSongs(_))
            ) {
                send_msg(ServerMessage::Error(
                    "cannot add song: game is not in song selection state".into(),
                ));
                return None;
            }
            drop(read_user);
            drop(games); // unlock during download

            let user_c = user.clone();
            let cloned_addr = user_addr.clone();
            thread::spawn(move || {
                let read_user = user_c.read().unwrap();

                let song_or_songs = music_handler::get_one_or_more_songs_from_id(&source_id);
                match song_or_songs {
                    Ok(song_or_songs) => {
                        let mut games = GAMES.write().unwrap();
                        let game = games
                            .get_mut(match &read_user.game_id {
                                Some(id) => id,
                                None => {
                                    return;
                                }
                            })
                            .unwrap();
                        drop(read_user);

                        let user_songs = match &mut game.state {
                            GameStatus::Playing(game_songs) => match game_songs {
                                PlayPhase::SelectingSongs(user_songs) => {
                                    match user_songs.get_mut(&user_ptr_addr) {
                                        Some(user_songs) => user_songs,
                                        None => {
                                            user_songs.insert(user_ptr_addr, Vec::new());
                                            user_songs.get_mut(&user_ptr_addr).unwrap()
                                        }
                                    }
                                }
                                _ => return,
                            },
                            _ => {
                                println!("Game started before song download could finish");
                                return;
                            }
                        };

                        match song_or_songs {
                            music_handler::OneOrMoreSongs::One(song) => {
                                user_songs.push(song);
                            }
                            music_handler::OneOrMoreSongs::More(songs) => {
                                user_songs.extend(songs);
                            }
                        }
                        cloned_addr.do_send(ServerMessage::AddedSong(user_songs.last().unwrap().clone()));
                    }
                    Err(err) => {
                        read_user
                            .ws
                            .as_ref()
                            .unwrap()
                            .do_send(ServerMessage::Error(format!("{:#?}", err)));
                    }
                }
            });
            ack();
        }
        UserAction::RemoveSong(idx) => {
            let read_user = user.read().unwrap();
            let mut games = GAMES.write().unwrap();
            let game = games.get_mut(&read_user.game_id?)?;
            let user_songs = match &mut game.state {
                GameStatus::Playing(phase) => match phase {
                    PlayPhase::SelectingSongs(user_songs) => user_songs.get_mut(&user_ptr_addr)?,
                    _ => return None,
                },
                _ => {
                    send_msg(ServerMessage::Error(
                        "cannot remove song: game is not in song selection state".into(),
                    ));
                    return None;
                }
            };
            if idx as usize >= user_songs.len() {
                send_msg(ServerMessage::Error(
                    "cannot remove song: index out of bounds".into(),
                ));
                return None;
            }
            user_songs.remove(idx as usize);
            send_msg(ServerMessage::RemovedSong(idx));
        }
        UserAction::StartGuessing => {
            let read_user = user.read().unwrap();
            if read_user.game_id.is_none() {
                send_msg(ServerMessage::Error(
                    "cannot start guessing: not in a game".into(),
                ));
                return None;
            }
            let mut games = GAMES.write().unwrap();
            let game = games.get_mut(&read_user.game_id.unwrap()).unwrap();
            game.start_guessing(user.clone());
        }
        UserAction::GuessSong(idx) => {
            let read_user = user.read().unwrap();
            if read_user.game_id.is_none() {
                send_msg(ServerMessage::Error(
                    "cannot add song: not in a game".into(),
                ));
                return None;
            }
            let games = GAMES.read().unwrap();
            let game = games.get(&read_user.game_id.unwrap()).unwrap();
            match &game.state {
                GameStatus::Playing(PlayPhase::GuessingSongs(tx)) => {
                    tx.send((user.clone(), idx)).unwrap();
                }
                _ => {
                    send_msg(ServerMessage::Error(
                        "cannot guess song: game is not in guessing state".into(),
                    ));
                }
            }
        }
        _ => send_msg(ServerMessage::Error("Invalid Action".to_string())),
    };
    Some(())
}
