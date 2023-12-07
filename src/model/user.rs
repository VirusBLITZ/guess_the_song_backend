use actix::Addr;

use crate::UserSocket;

type Message = String;

#[derive(Clone, Debug)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub game_id: Option<u16>,
    pub ws: Option<Addr<UserSocket>>,
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
