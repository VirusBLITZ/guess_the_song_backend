// const API_CLIENT: invidious::

#[cfg(not(debug_assertions))]
use std::process::Stdio;
use std::{
    collections::BTreeMap,
    fs,
    process::{self},
    sync::{
        atomic::{AtomicUsize, Ordering::SeqCst},
        mpsc::channel,
        RwLock,
    },
    thread,
    time::Duration,
};

use invidious::{
    hidden::SearchItem, ClientSync, ClientSyncTrait, CommonVideo, InvidiousError, MethodSync,
};
use once_cell::sync::Lazy;
use rand::{seq::SliceRandom, Rng};
use serde::Deserialize;

use crate::model::song::{GettingSongError, Song};

// static API_CLIENT: Lazy<RwLock<invidious::ClientSync>> =
//     Lazy::new(|| RwLock::new(invidious::ClientSync::default()));

const INSTANCE_COUNT: usize = 3;

#[cfg(debug_assertions)]
const INSTANCES_API_URI: &str = "NOT_THE_API";
#[cfg(not(debug_assertions))]
const INSTANCES_API_URI: &'static str = "https://api.invidious.io/instances.json?sort_by=health";
const BACKUP_INSTANCES: [&str; 3] = [
    "yt.oelrichsgarcia.de",
    "invidious.einfachzocken.eu",
    "iv.nboeck.de",
    // "inv.bp.projectsegfau.lt",
];
static INSTANCE_FINDER: Lazy<InstanceFinder> =
    Lazy::new(|| InstanceFinder::new(Vec::with_capacity(INSTANCE_COUNT)));

#[derive(Deserialize)]
struct Skip {}

#[derive(Debug)]
pub struct InstanceFinder {
    instances: RwLock<Vec<String>>,
    rr_index: AtomicUsize,
}

impl InstanceFinder {
    fn new(instances: Vec<String>) -> Self {
        Self {
            instances: RwLock::new(instances),
            rr_index: AtomicUsize::new(0),
        }
    }

    pub fn get_instance(&self) -> String {
        let instances = self.instances.read().unwrap();
        let rr_idx = self.rr_index.load(SeqCst);
        self.rr_index.store((rr_idx + 1) % instances.len(), SeqCst);
        instances.get(rr_idx).unwrap().clone()
    }

    fn backup_instances() -> Vec<String> {
        BACKUP_INSTANCES
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    }

    /// Select the healthiest instance from the list of instances and replace the current ones
    fn update_instances(&self) {
        let best_instances = match reqwest::blocking::get(INSTANCES_API_URI) {
            Ok(res) => match res.json::<Vec<(String, Skip)>>() {
                Ok(instances) => {
                    let mut uris: Vec<String> = Vec::with_capacity(INSTANCE_COUNT);
                    instances
                        .into_iter()
                        .take(INSTANCE_COUNT)
                        .for_each(|(uri, _)| uris.push(uri));
                    uris
                }
                Err(_) => {
                    eprintln!("Failed to parse instances.json");
                    InstanceFinder::backup_instances()
                }
            },
            Err(_) => {
                eprintln!("[UPDATER] couldn't get instances, using backup instances");
                InstanceFinder::backup_instances()
            }
        };
        println!("[UPDATER] using instances: {:?}", best_instances);
        let mut instances = self.instances.write().unwrap();
        instances.clear();
        instances.extend(best_instances);
    }
}

pub fn start_instance_finder() {
    println!("[MUSIC] starting instance updater");
    thread::spawn(|| loop {
        INSTANCE_FINDER.update_instances();
        thread::sleep(Duration::from_secs(60 * 480)); // 8 hours
    });
}

/// multithreaded playlist resolver
fn common_vids_from_id(id: &str) -> Result<Vec<CommonVideo>, GettingSongError> {
    if id.starts_with("UC") {
        let get_channel_vids = || get_client().channel_videos(id, Some("sort_by=popular"));

        let channel_vids = get_channel_vids();

        return match channel_vids {
            Ok(vids) => Ok(vids.videos),
            // second try with other instance/client, needed when region blocked
            Err(_) => Ok(get_channel_vids()?.videos),
        };
    }
    if !id.starts_with("PL") {
        return Ok(vec![]);
    };
    let playlist = get_client().playlist(id, None)?;
    let (tx, rx) = channel::<Option<CommonVideo>>();

    playlist.videos.into_iter().for_each(|playlist_item| {
        let tx = tx.clone();
        thread::spawn(move || {
            // tx.send(Some(get_client().video(&playlist_item.id.as_str(), None).unwrap().into()))
            tx.send(match get_client().video(&playlist_item.id, None) {
                Ok(vid) => Some(vid.into()),
                Err(_) => None,
            })
        });
    });
    let mut common_vids: Vec<CommonVideo> = vec![];
    for _ in 0..playlist.video_count {
        common_vids.push(rx.recv().unwrap().ok_or(GettingSongError::OtherError)?);
    }
    Ok(common_vids)
}

static QUERY_CACHE: Lazy<RwLock<BTreeMap<String, Vec<SearchItem>>>> =
    Lazy::new(|| RwLock::new(BTreeMap::new()));
static ID_METADATA_CACHE: Lazy<RwLock<BTreeMap<String, CommonVideo>>> =
    Lazy::new(|| RwLock::new(BTreeMap::new()));
pub fn get_suggestions(query: &str) -> Result<Vec<SearchItem>, InvidiousError> {
    {
        let cache_read = QUERY_CACHE.read().unwrap();
        if let Some(items) = cache_read.get(query) {
            return Ok(items.clone());
        }
    }

    let client = get_client();
    println!("using instance: {} for this query", client.get_instance());

    let query = query.trim_matches('"');
    let results = client
        .search(Some(format!("q={}", query.replace(' ', "+")).as_str()))?
        .items
        .into_iter()
        .take(6)
        .collect::<Vec<_>>();

    if results.is_empty() {
        return Ok(vec![]);
    }

    QUERY_CACHE
        .write()
        .unwrap()
        .insert(query.to_owned(), results.clone());

    let mut write_id_cache = ID_METADATA_CACHE.write().unwrap();
    results.iter().for_each(|search_item: &SearchItem| {
        match search_item {
            SearchItem::Video(vd) => {
                write_id_cache.insert(vd.id.clone(), vd.clone());
            }
            _ => (), // channel & playlist vids would need another request
        };
    });

    Ok(results)
}

fn get_client() -> ClientSync {
    ClientSync::with_method(
        format!("https://{}", INSTANCE_FINDER.get_instance()),
        MethodSync::Isahc,
    )
}

fn songs_from_common_vids(vids: Vec<CommonVideo>) -> Result<Vec<Song>, GettingSongError> {
    let mut songs = vec![];
    let (tx, rx) = std::sync::mpsc::channel::<Option<Song>>();

    let song_count = std::cmp::min(vids.len(), 5);
    let mut top30: Vec<CommonVideo> = vids.into_iter().take(30).collect();
    let mut initial_vids = vec![];
    for _ in 0..song_count {
        initial_vids.push(top30.swap_remove(rand::thread_rng().gen_range(0..top30.len())));
    }

    for vid in initial_vids {
        let tx = tx.clone();
        thread::spawn(move || {
            tx.send(download_song_from_id(&vid.id).ok()).unwrap();
        });
    }

    for _ in 0..song_count {
        if let Some(song) = rx.recv().unwrap() {
            songs.push(song);
        }
    }

    let mut attempts = 0;
    while songs.len() < song_count && attempts < 5 {
        let vid = top30.choose(&mut rand::thread_rng()).unwrap();
        if !songs.iter().any(|s| s.id == vid.id) {
            if let Some(song) = download_song_from_id(&vid.id).ok() {
                songs.push(song);
            }
        }
        attempts += 1;
    }

    Ok(songs)
}

fn download_song_from_id(id: &str) -> Result<Song, GettingSongError> {
    let mut songs_dir = std::env::current_dir().unwrap();
    songs_dir.push("songs_cache");

    if !songs_dir.exists() {
        fs::create_dir(&songs_dir).unwrap();
    }

    let metadata = {
        let read_metadata = ID_METADATA_CACHE.read().unwrap();
        if let Some(metadata) = read_metadata.get(id) {
            metadata.clone()
        } else {
            drop(read_metadata);
            let vid = CommonVideo::from(
                get_client()
                    .video(id, None)
                    .map_err(GettingSongError::from)?,
            );
            let mut write_metadata = ID_METADATA_CACHE.write().unwrap();
            write_metadata.insert(id.to_owned(), vid.clone());
            vid
        }
    };

    if !fs::read_dir(&songs_dir)
        .unwrap()
        .any(|entry| entry.unwrap().file_name() == id)
    {
        let command = &mut process::Command::new("yt-dlp");
        let cmd = command.current_dir(songs_dir.to_str().unwrap()).args([
            "-f",
            "bestaudio[acodec=opus]",
            "--max-filesize",
            "6000k",
            "-o",
            "%(id)s",
            "--",
            id,
        ]);

        #[cfg(not(debug_assertions))]
        let mut handle = cmd
            .stdout(Stdio::null())
            .spawn()
            .expect("spawning yt-dlp to work");
        #[cfg(debug_assertions)]
        let mut handle = cmd.spawn().expect("spawning yt-dlp to work");

        match handle.wait() {
            Ok(_) => {
                println!("yt-dlp download successful, filename: {}", id);
            }
            Err(e) => {
                eprintln!("yt-dlp download failed: {}", e);
                return Err(GettingSongError::DownloadFailed(e));
            }
        };
    }

    // ensure the file is actually there
    if !songs_dir.join(id).exists() {
        return Err(GettingSongError::DownloadFailed(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        )));
    }

    Ok(Song {
        id: id.to_owned(),
        title: metadata.title,
        artist: metadata.author,
    })
}

#[derive(Debug)]
pub enum OneOrMoreSongs {
    One(Song),
    More(Vec<Song>),
}

pub fn get_one_or_more_songs_from_id(id: &str) -> Result<OneOrMoreSongs, GettingSongError> {
    match id[..2].as_ref() {
        "UC" | "PL" => {
            let vids = common_vids_from_id(id)?;
            let songs = songs_from_common_vids(vids)?;
            Ok(OneOrMoreSongs::More(songs))
        }
        _ => Ok(OneOrMoreSongs::One(download_song_from_id(id)?)),
    }
}
