use std::string::String;

use invidious::{hidden::AdaptiveFormat, CommonVideo};

use crate::music_handler;

#[derive(Debug)]
pub enum GettingSongError {
    ReqwestErr(reqwest::Error),
    InvidiousErr(invidious::InvidiousError),
    DownloadFailed(std::io::Error),
}

impl From<reqwest::Error> for GettingSongError {
    fn from(value: reqwest::Error) -> Self {
        GettingSongError::ReqwestErr(value)
    }
}

impl From<invidious::InvidiousError> for GettingSongError {
    fn from(value: invidious::InvidiousError) -> Self {
        GettingSongError::InvidiousErr(value)
    }
}

#[derive(Clone, Debug)]
pub struct Song {
    id: String,
    name: String,
    artist: String,
    bytes: Vec<u8>,
}

pub type SongTitle = String;
pub type SongId = String;
pub type Author = String;
pub type InstanceUrl = String;
