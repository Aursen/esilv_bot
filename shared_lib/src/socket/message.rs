use actix::{Addr, Message};
use serde::{Deserialize, Serialize};

use crate::socket::session::Session;

#[derive(Serialize, Deserialize, Message, Debug)]
#[rtype(result = "()")]
pub enum ServerRequest {
    Ping,
    GetUser(String),
}

/// Messages TODO
#[derive(Serialize, Deserialize, Message, Debug)]
#[rtype(result = "()")]
pub enum BotResponse {
    Ping,
    User(String),
}

/// New session is created
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Addr<Session>,
}

/// Session is disconnected
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}
