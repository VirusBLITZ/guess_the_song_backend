use std::{sync::{mpsc::{Sender, SyncSender, sync_channel}, Arc, RwLock}, thread};

use crate::model::user::User;

use super::GAMES;

pub type PlayerGuess = (Arc<RwLock<User>>, u8);
pub fn handle_guessing(game_id: u16) -> SyncSender<PlayerGuess> {
    let (tx, rx) = sync_channel::<PlayerGuess>(6);
    
    thread::spawn(move || {
        let mut games = GAMES.write().unwrap();
        let game = games.get_mut(&game_id).unwrap();

        for song in 
        rx;

    });
    return tx;
}