use std::sync::Arc;

use actix::Addr;
use actix_web_actors::ws::WebsocketContext;

use crate::UserSocket;

type Message = String;

#[derive(Clone)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub score: u8,
    pub ws: Option<Addr<UserSocket>>,
}
