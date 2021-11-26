use std::collections::HashMap;

use actix::prelude::*;
use rand::{prelude::ThreadRng, Rng};

use crate::socket::{
    message::{Connect, Disconnect},
    session::Session,
};

pub struct Server {
    sessions: HashMap<usize, Addr<Session>>,
    rng: ThreadRng,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            sessions: HashMap::new(),
            rng: rand::thread_rng(),
        }
    }
}

impl Actor for Server {
    type Context = Context<Self>;
}

/// Handler for Connect message.
impl Handler<Connect> for Server {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        println!("Bot connected");

        // register session with random id
        let id = self.rng.gen::<usize>();
        self.sessions.insert(id, msg.addr);

        // send id back
        id
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for Server {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        println!("Bot disconnected");

        // remove address
        self.sessions.remove(&msg.id);
    }
}
