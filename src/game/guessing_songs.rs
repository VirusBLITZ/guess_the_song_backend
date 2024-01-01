use std::{
    collections::HashMap,
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc, RwLock,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use rand::{
    seq::{IteratorRandom, SliceRandom},
    thread_rng,
};

use crate::model::{song::Song, user::User};

use super::{GameStatus, ServerMessage, GAMES};

pub type PlayerGuess = (Arc<RwLock<User>>, u8);
pub fn handle_guessing(
    players: Vec<Arc<RwLock<User>>>,
    player_songs: &mut HashMap<usize, Vec<Song>>,
) -> (SyncSender<PlayerGuess>, JoinHandle<()>) {
    let (tx, rx) = sync_channel::<PlayerGuess>(2);

    let songs = player_songs
        .values_mut()
        .map(|v| std::mem::take(v))
        .flatten()
        .collect();

    let handle = thread::spawn(move || handle_game(players, songs, rx));
    (tx, handle)
}

pub fn handle_game_end(handle: JoinHandle<()>, game_id: u16) {
    thread::spawn(move || -> Option<()> {
        handle.join().unwrap();
        let mut games = GAMES.write().unwrap();
        let game = games.get_mut(&game_id)?;
        game.set_state(GameStatus::Lobby(0));
        game.broadcast_message(ServerMessage::GameEnded);
        Some(())
    });
}

fn broadcast_users(users: &Vec<Arc<RwLock<User>>>, msg: ServerMessage) {
    for user in users {
        if let Some(ws) = user.read().unwrap().ws.as_ref() {
            ws.do_send(msg.clone());
        }
    }
}

fn song_to_title_artist_tuple(song: &Song) -> (String, String) {
    (song.title.clone(), song.artist.clone())
}

fn handle_game(
    players: Vec<Arc<RwLock<User>>>,
    mut songs: Vec<Song>,
    user_msgs: Receiver<PlayerGuess>,
) {
    let mut leaderboad: Vec<(Arc<RwLock<User>>, usize)> = Vec::new();
    songs.shuffle(&mut thread_rng());

    let mut remaining_songs = songs.iter();
    for song in &songs {
        broadcast_users(&players, ServerMessage::GamePlayAudio(song.id.clone()));

        let mut options = remaining_songs
            .clone()
            .map(song_to_title_artist_tuple)
            .choose_multiple(&mut rand::thread_rng(), 4);
        if !options
            .iter()
            .any(|(t, a)| t == &song.title && a == &song.artist)
        {
            *options.choose_mut(&mut thread_rng()).unwrap() = song_to_title_artist_tuple(song);
        }
        if options.len() < 4 {
            options.extend(
                songs
                    .iter()
                    .filter(|s| s.id != song.id)
                    .choose_multiple(&mut thread_rng(), 4 - options.len())
                    .into_iter()
                    .map(song_to_title_artist_tuple),
            );
        }
        options.shuffle(&mut thread_rng());
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
        remaining_songs.next();
        broadcast_users(&players, ServerMessage::Correct(correct_idx));
        thread::sleep(Duration::from_secs(2));

        // rx.try_recv()
        broadcast_users(
            &players,
            ServerMessage::LeaderBoard(
                leaderboad
                    .clone()
                    .into_iter()
                    .map(|(user, score)| (user.read().unwrap().name.clone(), score))
                    .collect(),
            ),
        );
        thread::sleep(Duration::from_secs(5));
    }
    thread::sleep(Duration::from_secs(10));
}
