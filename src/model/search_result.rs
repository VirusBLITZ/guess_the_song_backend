use invidious::hidden::SearchItem;
use serde::Serialize;

#[derive(Serialize)]
pub struct SearchResult {
    name: String,
    id: String,
    r#type: String,
}

impl From<SearchItem> for SearchResult {
    fn from(item: SearchItem) -> Self {
        let (name, id, type_) = match item {
            SearchItem::Video(video) => (video.title, video.id, "video"),
            SearchItem::Channel(channel) => (channel.name, channel.id, "channel"),
            SearchItem::Playlist(playlist) => (playlist.title, playlist.id, "playlist"),
        };

        Self {
            name,
            id,
            r#type: type_.to_string(),
        }
    }
}
