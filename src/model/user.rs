type Message = String;
pub struct User {
    pub id: i32,
    pub name: String,
    pub ws: Option<std::sync::mpsc::Sender<Message>>
}