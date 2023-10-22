use std::string::String;

use invidious::{hidden::AdaptiveFormat, CommonVideo};

pub struct GettingSongError {
    reqwest_err: Option<reqwest::Error>,
    invidious_err: Option<invidious::InvidiousError>,
}

impl From<reqwest::Error> for GettingSongError {
    fn from(value: reqwest::Error) -> Self {
        Self {
            reqwest_err: Some(value),
            invidious_err: None,
        }
    }
}

impl From<invidious::InvidiousError> for GettingSongError {
    fn from(value: invidious::InvidiousError) -> Self {
        Self {
            reqwest_err: None,
            invidious_err: Some(value),
        }
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
impl TryFrom<(SongId, SongTitle, Author, AdaptiveFormat)> for Song {
    type Error = reqwest::Error;
    /// Downloads the song from the given adaptive format
    fn try_from(song_info: (SongId, SongTitle, Author, AdaptiveFormat)) -> Result<Self, Self::Error> {
        let (id, title, author, format) = song_info;
        let bytes = reqwest::blocking::get(format.url.as_str())?.bytes()?;   // TODO: stream while playing
        Ok(Self {
            id,
            name: title,
            artist: author,
            bytes: bytes.to_vec(),
        })
    }
}
