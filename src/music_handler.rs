// const API_CLIENT: invidious::

use invidious::{ClientSync, InvidiousError, MethodSync};
use once_cell::sync::Lazy;
use serde::Deserialize;

// static API_CLIENT: Lazy<RwLock<invidious::ClientSync>> =
//     Lazy::new(|| RwLock::new(invidious::ClientSync::default()));
static INSTANCES_API_URI: &'static str = "https://api.invidious.io/instances.json?sort_by=health";
static BACKUP_INSTANCES: Vec<&str> = vec![
    "yt.oelrichsgarcia.de",
    "invidious.einfachzocken.eu",
    "yt.cdaut.de",
    // "inv.bp.projectsegfau.lt",
];

type StatusInstance = (String, InstanceDetails);

#[derive(Deserialize)]
struct InstanceDetails;
type Instance<'a> = &'a str;

struct InstanceFinder<'a> {
    instances: [&'a str; 3],
    rr_index: usize,
}

impl InstanceFinder<'_> {
    fn get_instance(&mut self) -> &str {
        self.rr_index += 1;
        if self.rr_index >= self.instances.len() {
            self.rr_index = 0;
        }
        self.instances[self.rr_index]
    }

    /// Select the healthiest instance from the list of instances and replace the current ones
    fn update_instances(&mut self) {
        let instances = match reqwest::blocking::get(INSTANCES_API_URI) {
            Ok(res) => match res.json::<Vec<(&str, InstanceDetails)>>() {
                Ok(instances) => instances
                    .into_iter()
                    .map(|(addr, _)| addr)
                    .collect::<Vec<&str>>(),
                Err(_) => {
                    eprintln!("Failed to parse instances.json");
                    return;
                }
            },
            Err(_) => {
                eprintln!("Failed to fetch instances.json");
                BACKUP_INSTANCES
            }
        };
    }
}

pub fn get_suggestions() -> Result<Vec<invidious::hidden::SearchItem>, InvidiousError> {
    // *invidious::INSTANCE = *INSTANCE;
    // set invidious::INSTANCE to INSTANCE
    let client = ClientSync::with_method("yt.oelrichsgarcia.de".to_string(), MethodSync::HttpReq);

    // let suggestions = client.search("https://www.youtube.com/watch?v=2Vv-BfVoq4g")?;
    Ok(vec![])
}
