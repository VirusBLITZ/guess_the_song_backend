use actix::Addr;

use crate::UserSocket;

type Message = String;

#[derive(Clone, Debug)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub score: u8,
    pub game_id: Option<u16>,
    pub ws: Option<Addr<UserSocket>>,
}
