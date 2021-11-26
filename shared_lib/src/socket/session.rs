use std::{
    net,
    str::FromStr,
    time::{Duration, Instant},
};

use tokio::{
    io::{split, WriteHalf},
    net::{TcpListener, TcpStream},
};

use actix::{io::FramedWrite, prelude::*};
use tokio_util::codec::FramedRead;

use crate::socket::{
    codec::ServerCodec,
    message::{BotResponse, Connect, Disconnect, ServerRequest},
    server::Server,
};

pub struct Session {
    id: usize,
    addr: Addr<Server>,
    hb: Instant,
    framed: FramedWrite<ServerRequest, WriteHalf<TcpStream>, ServerCodec>,
}

impl Actor for Session {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        let addr = ctx.address();

        self.addr
            .send(Connect { addr })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    _ => ctx.stop(),
                }
                actix::fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.addr.do_send(Disconnect { id: self.id });
        Running::Stop
    }
}

impl actix::io::WriteHandler<std::io::Error> for Session {}

/// To use `Framed` we have to define Io type and Codec
impl StreamHandler<Result<BotResponse, std::io::Error>> for Session {
    fn handle(&mut self, msg: Result<BotResponse, std::io::Error>, ctx: &mut Context<Self>) {
        match msg {
            // we update heartbeat time on ping from peer
            Ok(BotResponse::Ping) => self.hb = Instant::now(),
            _ => ctx.stop(),
        }
    }
}

impl Session {
    pub fn new(
        addr: Addr<Server>,
        framed: FramedWrite<ServerRequest, WriteHalf<TcpStream>, ServerCodec>,
    ) -> Session {
        Session {
            id: 0,
            addr,
            hb: Instant::now(),
            framed,
        }
    }
    /// helper method that sends ping to client every second.
    ///
    /// also this method check heartbeats from client
    fn hb(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(Duration::new(1, 0), |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > Duration::new(10, 0) {
                // heartbeat timed out
                println!("Client heartbeat failed, disconnecting!");

                // notify chat server
                act.addr.do_send(Disconnect { id: act.id });

                // stop actor
                ctx.stop();
            }

            act.framed.write(ServerRequest::Ping);
            // if we can not send message to sink, sink is closed (disconnected)
        });
    }
}

/// Define tcp server that will accept incoming tcp connection and create
/// chat actors.
pub fn tcp_server(s: &str, server: Addr<Server>) {
    // Create server listener
    let addr = net::SocketAddr::from_str(s).unwrap();

    actix_web::rt::spawn(async move {
        let listener = TcpListener::bind(&addr).await.unwrap();

        loop {
            let server = server.clone();
            match listener.accept().await {
                Ok((stream, _)) => {
                    Session::create(|ctx| {
                        let (r, w) = split(stream);
                        Session::add_stream(FramedRead::new(r, ServerCodec), ctx);
                        Session::new(server, FramedWrite::new(w, ServerCodec, ctx))
                    });
                }
                Err(e) => println!("couldn't get client: {:?}", e),
            }
        }
    });
}
