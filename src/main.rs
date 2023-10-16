mod game;
mod model;

use std::sync::{Arc, RwLock};

use actix::{Actor, ActorContext, AsyncContext, Handler, Message, StreamHandler};
use actix_web::{get, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws::{self, CloseCode, CloseReason};
use game::ServerMessage;
use model::{user::User, *};

pub struct UserSocket {
    pub user: Arc<RwLock<User>>,
}

impl Actor for UserSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.text("Hello World!");
        self.user.write().unwrap().ws = Some(ctx.address());
    }
}

impl Handler<ServerMessage> for UserSocket {
    type Result = ();

    fn handle(&mut self, msg: ServerMessage, ctx: &mut Self::Context) {
        ctx.text(match msg {
            ServerMessage::ServerAck => "k".to_string(),
            ServerMessage::UserJoin(name) => format!("{} joined the game", name),
            _ => format!("{:?}", msg),
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for UserSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        dbg!(&msg);
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => match text.to_string().trim().split_once(" ") {
                Some((action, body)) => game::handle_user_msg(action, body, self.user.clone()),
                _ => ctx.text("?"),
            },
            _ => {
                ctx.close(Some(CloseReason {
                    code: CloseCode::Invalid,
                    description: Some("Unsupported Format".to_owned()),
                }));
                ctx.stop();
            }
        }
    }
}

#[get("/ws")]
async fn index(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let resp = ws::start(
        UserSocket {
            user: Arc::new(RwLock::new(User {
                id: rand::random(),
                name: "User ".to_string() + rand::random::<u8>().to_string().as_str(),
                score: 0,
                ws: None,
            })),
        },
        &req,
        stream,
    );
    println!("{:?}", resp);
    resp
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(index))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
