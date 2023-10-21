// const API_CLIENT: invidious::

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering::Relaxed},
        RwLock,
    },
    thread,
    time::Duration,
    vec,
};

use invidious::{ClientSync, ClientSyncTrait, InvidiousError, MethodSync};
use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::model::song::Song;

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
        thread::sleep(Duration::from_secs(60 * 10));
    });
}

pub fn get_suggestions(query: &str) -> Result<Vec<invidious::hidden::SearchItem>, InvidiousError> {
    let client = ClientSync::with_method(
        format!("https://{}", INSTANCE_FINDER.get_instance()),
        MethodSync::HttpReq,
    );
    println!("using instance: {} for this query", client.get_instance());

    let query = query.trim_matches('"');
    Ok(client
        .search(Some(format!("q=\"{}\"", query.replace(" ", "+")).as_str()))?
        .items
        .into_iter()
        .take(6)
        .collect::<Vec<_>>())
}

pub fn songs_from_id(id: &str) -> Vec<Song> {
    let mut client = ClientSync::default();
    client.set_instance(INSTANCE_FINDER.get_instance());

    // let client = ClientSync::with_method(
    //     format!("https://{}", INSTANCE_FINDER.get_instance()),
    //     MethodSync::Reqwest,
    // );

    let songs: Vec<Song> = vec![];
    match id.get(0..2) {
        Some(start) => match start {
            "UC" => println!("channel"),
            "PL" => println!("playlist"),
            _ => println!("video"),
        },
        None => println!("no id"),
    };
    songs
}
