use std::{
    sync::{
        mpsc::{sync_channel, Sender, SyncSender, Receiver},
        Arc, RwLock,
    },
    thread, mem::replace,
};

use crate::model::user::User;

use super::GAMES;

pub type PlayerGuess = (Arc<RwLock<User>>, u8);
pub fn handle_guessing(game_id: u16) -> SyncSender<PlayerGuess> {
    let (tx, rx) = sync_channel::<PlayerGuess>(6);

    thread::spawn(move || handle_game(game_id, rx));
    return tx;
}

fn handle_game(game_id: u16, user_msgs: Receiver<(Arc<RwLock<User>>, u8)>) {
    let mut games = GAMES.write().unwrap();
    let game = games.get_mut(&game_id).unwrap();
    let songs = match game.state {
        super::GameStatus::Playing(ref mut songs, _) => songs,
        _ => return,
    };
    let songs = replace(songs, Vec::new());
    println!("took songs: {:?}", songs);

    for song in songs {
        // rx.try_recv()
    }
}
