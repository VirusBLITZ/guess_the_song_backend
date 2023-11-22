use std::string::String;

use invidious::{hidden::AdaptiveFormat, CommonVideo};

use crate::music_handler;

#[derive(Debug)]
pub enum GettingSongError {
    ReqwestErr(reqwest::Error),
    InvidiousErr(invidious::InvidiousError),
    DownloadFailed(std::io::Error),
    OtherError
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
    pub id: String,
    pub title: String,
    pub artist: String,
}