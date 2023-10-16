use actix::Addr;

use crate::UserSocket;

type Message = String;

#[derive(Clone)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub score: u8,
    pub ws: Option<Addr<UserSocket>>,
}
