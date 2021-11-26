use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Room {
    pub(crate) discord_id: u64,
    pub(crate) office_id: u64,
    pub(crate) waiting_id: u64,
    pub(crate) text_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub(crate) roles: HashMap<String, u64>,
    pub(crate) room: u64,
    pub(crate) teacher_category: u64,
    pub(crate) subjects: Vec<SubjectsMessage>,
}

#[derive(Serialize, Deserialize)]
pub struct SubjectsMessage {
    pub(crate) id: u64,
    pub(crate) channels: HashMap<String, u64>,
}
