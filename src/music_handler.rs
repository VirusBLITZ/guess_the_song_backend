// const API_CLIENT: invidious::

use std::{
    process,
    sync::{
        atomic::{AtomicUsize, Ordering::Relaxed},
        RwLock,
    },
    thread,
    time::Duration,
    vec,
};

use invidious::{
    channel::Channel,
    hidden::{AdaptiveFormat, FormatStream},
    video::Video,
    ClientSync, ClientSyncTrait, CommonVideo, InvidiousError, MethodSync,
};
use once_cell::sync::Lazy;
use rand::seq::IteratorRandom;
use serde::Deserialize;

use crate::model::song::{self, GettingSongError, Song};

// static API_CLIENT: Lazy<RwLock<invidious::ClientSync>> =
//     Lazy::new(|| RwLock::new(invidious::ClientSync::default()));

const INSTANCE_COUNT: usize = 3;

#[cfg(debug_assertions)]
const INSTANCES_API_URI: &'static str = "NOT_THE_API";
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
        let rr_idx = self.rr_index.load(Relaxed);
        self.rr_index.store(rr_idx + 1, Relaxed);
        if rr_idx + 1 >= instances.len() {
            self.rr_index.store(0, Relaxed);
        }
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
            Ok(res) => {
                // dbg!(res.text());
                match res.json::<Vec<(String, Skip)>>() {
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
                }
            }
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

pub fn get_suggestions(query: &str) -> Result<Vec<invidious::hidden::SearchItem>, InvidiousError> {
    let client = get_client();
    println!("using instance: {} for this query", client.get_instance());

    let query = query.trim_matches('"');
    Ok(client
        .search(Some(format!("q=\"{}\"", query.replace(" ", "+")).as_str()))?
        .items
        .into_iter()
        .take(6)
        .collect::<Vec<_>>())
}

fn get_client() -> ClientSync {
    ClientSync::with_method(
        format!("https://{}", INSTANCE_FINDER.get_instance()),
        MethodSync::HttpReq,
    )
}

trait SongSource {
    fn try_get_songs(self, instance_url: String) -> Result<Vec<Song>, GettingSongError>;
}

// impl SongSource for invidious::video::Video {}

impl SongSource for Video {
    fn try_get_songs(self, instance_url: String) -> Result<Vec<Song>, GettingSongError> {
        let mut fmts: Vec<_> = self
            .adaptive_formats
            .into_iter()
            .filter(|s| s.r#type.starts_with("audio"))
            .collect();
        fmts.sort_by(|a, b| b.bitrate.cmp(&a.bitrate));
        fmts.drain(1..);
        let song = Song::try_from((
            self.id,
            self.title,
            self.author,
            fmts.into_iter().next().unwrap(),
            instance_url,
        ))?;
        Ok(vec![song])
    }
}

fn songs_from_common_vids(vids: Vec<CommonVideo>) -> Result<Vec<Song>, GettingSongError> {
    let mut songs = vec![];
    let (tx, rx) = std::sync::mpsc::channel::<Option<Song>>();

    for video in vids
        .into_iter()
        .take(30)
        .choose_multiple(&mut rand::thread_rng(), 5)
    {
        let tx = tx.clone();
        let client = get_client();
        thread::spawn(move || {
            let vid_res = client.video(&video.id, None);
            let song = match vid_res {
                Ok(vid) => match vid.try_get_songs(client.instance) {
                    Ok(song) => Some(song.into_iter().next().unwrap()),
                    Err(_) => None,
                },
                _ => None,
            };
            tx.send(song).unwrap();
        });
    }

    for _ in 0..5 {
        if let Some(song) = rx.recv().unwrap() {
            songs.push(song);
        }
    }
    Ok(songs)
}

impl SongSource for Channel {
    fn try_get_songs(self, instance_url: String) -> Result<Vec<Song>, GettingSongError> {
        // load channel videos
        // get first 30 most popular videos
        // select 5 random videos from those
        // get songs from those videos
        let client = get_client();

        let videos = client
            .channel_videos(&self.id, Some("sort_by=popular"))?
            .videos;

        Ok(songs_from_common_vids(videos)?)
    }
}

pub fn songs_from_id(id: &str) -> Result<Vec<Song>, GettingSongError> {
    let client = get_client();

    let cwd = std::env::current_dir().unwrap();
    cwd.push("songs_cache");
    let handle = process::Command::new("yt-dlp")
        .current_dir(cwd.to_str().unwrap())
        .args(["-f", "bestaudio[acodec=opus]", id])
        .spawn();
    
}
