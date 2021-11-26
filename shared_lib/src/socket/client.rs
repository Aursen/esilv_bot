use actix::{clock::Instant, io::FramedWrite, prelude::*};
use tokio_util::codec::FramedRead;

use std::{io, net, str::FromStr, time::Duration};
use tokio::{
    io::{split, WriteHalf},
    net::TcpStream,
};

use super::{
    codec::ClientCodec,
    message::{BotResponse, ServerRequest},
};

pub struct ChatClient {
    hb: Instant,
    framed: FramedWrite<BotResponse, WriteHalf<TcpStream>, ClientCodec>,
}

impl Actor for ChatClient {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        // start heartbeats otherwise server will disconnect after 10 seconds
        self.hb(ctx)
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        println!("Disconnected");

        // Stop application on disconnect
        System::current().stop();
    }
}

impl ChatClient {
    fn hb(&self, ctx: &mut Context<Self>) {
        ctx.run_later(Duration::new(1, 0), |act, ctx| {
            act.framed.write(BotResponse::Ping);
            act.hb(ctx);

            // client should also check for a timeout here, similar to the
            // server code
        });
    }
}

impl actix::io::WriteHandler<io::Error> for ChatClient {}

/// Server communication
impl StreamHandler<Result<ServerRequest, io::Error>> for ChatClient {
    fn handle(&mut self, msg: Result<ServerRequest, io::Error>, ctx: &mut Context<Self>) {
        match msg {
            Ok(ServerRequest::Ping) => self.hb = Instant::now(),
            Ok(ServerRequest::GetUser(user)) => {
                println!("Get User: {}", user);
            }
            _ => ctx.stop(),
        }
    }
}

/// Define tcp client that will connect to tcp listener
/// chat actors.
pub async fn tcp_client(s: &str) -> Addr<ChatClient> {
    // Connect to server
    let addr = net::SocketAddr::from_str(s).unwrap();

    let stream = TcpStream::connect(&addr).await.unwrap();

    ChatClient::create(|ctx| {
        let (r, w) = split(stream);
        ChatClient::add_stream(FramedRead::new(r, ClientCodec), ctx);
        ChatClient {
            hb: Instant::now(),
            framed: actix::io::FramedWrite::new(w, ClientCodec, ctx),
        }
    })
}
