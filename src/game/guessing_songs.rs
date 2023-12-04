use std::sync::{mpsc::Sender, Arc, RwLock};

use crate::model::user::User;

pub type PlayerGuess = (Arc<RwLock<User>>, u8);
pub fn handle_guessing(game_id: u16) -> Sender<PlayerGuess> {
    
}