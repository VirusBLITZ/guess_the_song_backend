use std::{
    mem::replace,
    sync::{
        mpsc::{sync_channel, Receiver, Sender, SyncSender},
        Arc, RwLock,
    },
    thread,
};

use invidious::hidden::Users;
use rand::{
    seq::{IteratorRandom, SliceRandom},
    thread_rng,
};

use crate::model::user::User;

use super::{ServerMessage, GAMES};

pub type PlayerGuess = (Arc<RwLock<User>>, u8);
pub fn handle_guessing(game_id: u16) -> SyncSender<PlayerGuess> {
    let (tx, rx) = sync_channel::<PlayerGuess>(6);

    thread::spawn(move || handle_game(game_id, rx));
    return tx;
}

fn broadcast_users(users: &Vec<Arc<RwLock<User>>>, msg: ServerMessage) {
    for user in users {
        user.read()
            .unwrap()
            .ws
            .as_ref()
            .unwrap()
            .do_send(msg.clone());
    }
}

fn handle_game(game_id: u16, user_msgs: Receiver<(Arc<RwLock<User>>, u8)>) {
    let mut games = GAMES.write().unwrap();
    let game = games.get_mut(&game_id).unwrap();
    let songs = match game.state {
        super::GameStatus::Playing(ref mut songs, _) => songs,
        _ => return,
    };
    let songs = replace(songs, Vec::new());
    let players = game.players.clone();
    drop(games);
    println!("took songs: {:?}", songs);

    for song in &songs {
        broadcast_users(&players, ServerMessage::GamePlayAudio(song.id.clone()));
        let mut options = songs
            .iter()
            .map(|s| ((s.title.clone(), s.artist.clone())))
            .choose_multiple(&mut rand::thread_rng(), 3);
        if options
            .iter()
            .find(|(t, a)| t == &song.title && a == &song.artist)
            .is_none()
        {
            *options.choose_mut(&mut thread_rng()).unwrap() =
                (song.title.clone(), song.artist.clone());
        }
        dbg!(&song);
        broadcast_users(&players, ServerMessage::GameGuessOptions(options));
        // rx.try_recv()
    }
}
