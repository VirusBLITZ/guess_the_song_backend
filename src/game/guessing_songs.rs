use std::{
    mem::replace,
    sync::{
        mpsc::{sync_channel, Receiver, Sender, SyncSender},
        Arc, RwLock,
    },
    thread,
    time::Duration,
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

    let mut leaderboad: Vec<(Arc<RwLock<User>>, usize)> = Vec::new();

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
        broadcast_users(&players, ServerMessage::GameGuessOptions(options.clone()));

        let guessing_start: std::time::Instant = std::time::Instant::now();
        let guess_timeout = Duration::from_secs(30);
        let remaining = guess_timeout - guessing_start.elapsed();
        let mut guessed_count = 0;
        while guessed_count < players.len() && guessing_start.elapsed() < Duration::from_secs(30) {
            if let Ok((user, guess)) = user_msgs.recv_timeout(remaining) {
                let guessed_at = guessing_start.elapsed().as_secs() * 10;
                match options.get(guess as usize) {
                    Some((title, artist)) if title == &song.title && artist == &song.artist => {
                        let mut leader_entry =
                            leaderboad.iter_mut().find(|(u, _)| Arc::ptr_eq(&user, u));
                        if let None = leader_entry {
                            leaderboad.push((user.clone(), 0));
                            leader_entry = leaderboad.last_mut();
                        }
                        leader_entry.unwrap().1 += (33
                            + ((1) / (guessed_at + 33))
                                * 2500
                                * (f64::log10((guessed_at + 100) as f64) as u64))
                            as usize;
                    }
                    _ => {}
                }
                guessed_count += 1;
            }
        }
        // rx.try_recv()
        broadcast_users(
            &players,
            ServerMessage::LeaderBoard(
                leaderboad
                    .clone()
                    .into_iter()
                    .map(|(u, s)| (u.read().unwrap().name.clone(), s))
                    .collect(),
            ),
        );
    }
    broadcast_users(&players, ServerMessage::GameEnded);
    let mut games = GAMES.write().unwrap();
    games.get_mut(&game_id).unwrap().state = super::GameStatus::Lobby(0);
}
