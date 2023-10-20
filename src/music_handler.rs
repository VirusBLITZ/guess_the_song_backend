// const API_CLIENT: invidious::

use std::sync::RwLock;

use invidious::{ClientSync, InvidiousError, MethodSync};
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::{value::RawValue, Value};

// static API_CLIENT: Lazy<RwLock<invidious::ClientSync>> =
//     Lazy::new(|| RwLock::new(invidious::ClientSync::default()));

const INSTANCE_COUNT: usize = 3;
static INSTANCES_API_URI: &'static str = "https://api.invidious.io/instances.json?sort_by=health";
static BACKUP_INSTANCES: [&str; 3] = [
    "yt.oelrichsgarcia.de",
    "invidious.einfachzocken.eu",
    "yt.cdaut.de",
    // "inv.bp.projectsegfau.lt",
];
const INSTANCE_FINDER: Lazy<InstanceFinder> = Lazy::new(|| InstanceFinder {
    instances: RwLock::new(BACKUP_INSTANCES),
    rr_index: 0,
});

#[derive(Deserialize)]
struct StatusInstance(String, #[serde(skip)] Value);

type Instance<'a> = &'a str;

struct InstanceFinder<'a> {
    instances: RwLock<[&'a str; INSTANCE_COUNT]>,
    rr_index: usize,
}

impl InstanceFinder<'_> {
    fn get_instance(&mut self) -> &str {
        let instances = self.instances.read().unwrap();
        self.rr_index += 1;
        if self.rr_index >= instances.len() {
            self.rr_index = 0;
        }
        instances[self.rr_index]
    }

    /// Select the healthiest instance from the list of instances and replace the current ones
    fn update_instances(&mut self) {
        let best_instances = match reqwest::blocking::get(INSTANCES_API_URI) {
            Ok(res) => match res.json::<Vec<StatusInstance>>() {
                Ok(instances) => {
                    let mut uris = Vec::with_capacity(INSTANCE_COUNT);
                    instances.into_iter().for_each(|i| uris.push(i.0));
                    uris
                }
                Err(_) => {
                    eprintln!("Failed to parse instances.json");
                    return;
                }
            },
            Err(_) => {
                eprintln!("Failed to fetch instances.json");
                BACKUP_INSTANCES.iter().map(|s| s.to_string()).collect::<Vec<_>>()
            }
        };
        let mut instances = self.instances.write().unwrap();
        best_instances.into_iter().enumerate().for_each(|(i, instance)| {
            
        });
    }
}

pub fn get_suggestions() -> Result<Vec<invidious::hidden::SearchItem>, InvidiousError> {
    // *invidious::INSTANCE = *INSTANCE;
    // set invidious::INSTANCE to INSTANCE
    let client = ClientSync::with_method("yt.oelrichsgarcia.de".to_string(), MethodSync::HttpReq);

    // let suggestions = client.search("https://www.youtube.com/watch?v=2Vv-BfVoq4g")?;
    Ok(vec![])
}
