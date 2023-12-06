mod game;
mod model;
mod music_handler;

use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use actix::{Actor, ActorContext, AsyncContext, Handler, StreamHandler};
use actix_web::{get, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws::{self, CloseCode, CloseReason};
use game::{ServerMessage, UserAction};

use model::{search_result::SearchResult, user::User};

pub struct UserSocket {
    pub user: Arc<RwLock<User>>,
    hb: Instant,
}

impl UserSocket {
    pub fn new() -> Self {
        Self {
            hb: Instant::now(),
            user: Arc::new(RwLock::new(User {
                id: rand::random(),
                name: "User ".to_string() + rand::random::<u8>().to_string().as_str(),
                game_id: None,
                ws: None,
            })),
        }
    }

    /// heartbeat
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        const HEARTBEAT_INTERVAL: std::time::Duration = Duration::from_secs(10);
        const CLIENT_TIMEOUT: std::time::Duration = Duration::from_secs(20);
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                println!("Websocket Client heartbeat failed, disconnecting!");

                ctx.stop();
                return;
            }

            ctx.ping(b"");
        });
    }
}

impl Actor for UserSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
        ctx.text(format!(
            "GTS v{} | {} | under {}",
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_REPOSITORY"),
            env!("CARGO_PKG_LICENSE")
        ));
        ctx.text(format!("song_route {}", SONGS_ROUTE));
        self.user.write().unwrap().ws = Some(ctx.address());
    }

    fn stopping(&mut self, _: &mut Self::Context) -> actix::Running {
        println!(
            "[disconnect] actor {} stopped",
            self.user.read().unwrap().id
        );

        game::handle_user_msg(UserAction::LeaveGame, self.user.clone());
        actix::Running::Stop
    }
}
impl Handler<ServerMessage<'_>> for UserSocket {
    type Result = ();

    fn handle(&mut self, msg: ServerMessage, ctx: &mut Self::Context) {
        ctx.text(match msg {
            ServerMessage::ServerAck => "k".to_string(),
            ServerMessage::Error(msg) => format!("ERR \"{:?}\"", msg),
            ServerMessage::GameCreated(id) => format!("game_created {}", id),
            ServerMessage::GameNotFound => "game_not_found".to_string(),
            ServerMessage::UserJoin(name) => format!("user_join \"{}\"", name),
            ServerMessage::UserLeave(name) => format!("user_leave \"{}\"", name),
            ServerMessage::UserReady(name) => format!("user_ready \"{}\"", name),
            ServerMessage::UserUnready(name) => format!("user_unready \"{}\"", name),
            ServerMessage::GameStartAt(time) => format!("game_start_at {}", time),
            ServerMessage::GameStartSelect => "game_start_select".to_string(),
            ServerMessage::Suggestion(songs) => format!(
                "suggestions {}",
                serde_json::to_string(
                    &songs
                        .into_iter()
                        .map(|s| SearchResult::from(s))
                        .collect::<Vec<_>>()
                )
                .unwrap()
            ),
            ServerMessage::GameStartGuessing => "game_start_guessing".to_string(),
            ServerMessage::GamePlayAudio(id) => format!("game_play_audio {}", id),
            ServerMessage::GameGuessOptions(options) => serde_json::to_string(&options).unwrap(),
            ServerMessage::Correct(idx) => format!("correct {}", idx),
            // ServerMessage::GamePlayAudio
            _ => format!("{:?}", msg),
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for UserSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => match text.to_string().trim().split_once(' ') {
                Some((action, body)) => {
                    self.hb = Instant::now();
                    game::handle_user_msg(UserAction::from((action, body)), self.user.clone())
                }
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
    let resp = ws::start(UserSocket::new(), &req, stream);
    // println!("{:?}", resp);
    resp
}

const SONGS_ROUTE: &str = "/songs";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    music_handler::start_instance_finder();

    HttpServer::new(|| {
        App::new()
            .service(index)
            .service(actix_files::Files::new(SONGS_ROUTE, "./songs_cache"))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
