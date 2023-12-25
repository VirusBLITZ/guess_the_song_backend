use std::{
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc, RwLock,
    },
    thread,
    time::Duration,
};

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
    tx
}

fn broadcast_users(users: &Vec<Arc<RwLock<User>>>, msg: ServerMessage) {
    for user in users {
        if let Some(ws) = user.read().unwrap().ws.as_ref() {
            ws.do_send(msg.clone());
        }
    }
}

fn handle_game(game_id: u16, user_msgs: Receiver<PlayerGuess>) {
    let mut games = GAMES.write().unwrap();
    let game = games.get_mut(&game_id).unwrap();
    let songs = match game.state {
        super::GameStatus::Playing(ref mut songs, _) => songs,
        _ => return,
    };
    let songs = std::mem::take(songs);
    let players = game.players.clone();
    drop(games);

    let mut leaderboad: Vec<(Arc<RwLock<User>>, usize)> = Vec::new();

    for song in &songs {
        broadcast_users(&players, ServerMessage::GamePlayAudio(song.id.clone()));

        let mut options = songs
            .iter()
            .map(|s| ((s.title.clone(), s.artist.clone())))
            .choose_multiple(&mut rand::thread_rng(), 4);
        if !options
            .iter()
            .any(|(t, a)| t == &song.title && a == &song.artist)
        {
            *options.choose_mut(&mut thread_rng()).unwrap() =
                (song.title.clone(), song.artist.clone());
        }
        let correct_idx = options
            .iter()
            .position(|(t, a)| t == &song.title && a == &song.artist)
            .unwrap() as u8;

        broadcast_users(&players, ServerMessage::GameGuessOptions(options.clone()));

        let guessing_start: std::time::Instant = std::time::Instant::now();
        let guess_timeout = Duration::from_secs(180);
        let remaining = guess_timeout - guessing_start.elapsed();
        let mut guessed_count = 0;
        while guessed_count < players.len() && guessing_start.elapsed() < Duration::from_secs(180) {
            if let Ok((user, guess)) = user_msgs.recv_timeout(remaining) {
                let guessed_at = guessing_start.elapsed().as_secs() * 10;
                if guess == correct_idx {
                    let user_score =
                        &mut match leaderboad.iter_mut().find(|(u, _)| Arc::ptr_eq(&user, u)) {
                            Some(entry) => entry,
                            None => {
                                leaderboad.push((user.clone(), 0));
                                leaderboad.last_mut().unwrap()
                            }
                        }
                        .1;
                    *user_score += (33
                        + ((1) / (guessed_at + 33))
                            * 2500
                            * (f64::log10((guessed_at + 100) as f64) as u64))
                        as usize;
                }
                guessed_count += 1;
            }
        }
        broadcast_users(&players, ServerMessage::Correct(correct_idx));

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
        thread::sleep(Duration::from_secs(5));
    }
    broadcast_users(&players, ServerMessage::GameEnded);
    let mut games = GAMES.write().unwrap();
    if let Some(game) = games.get_mut(&game_id) {
        game.state = super::GameStatus::Lobby(0);
    }
}
