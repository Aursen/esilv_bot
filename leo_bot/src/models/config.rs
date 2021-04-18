use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub roles: HashMap<String, u64>,
    pub webhook: u64,
    pub room: u64,
    pub teacher_category: u64,
    pub gamer: u64,
    pub subjects: Vec<SubjectsMessage>,
}

#[derive(Serialize, Deserialize)]
pub struct SubjectsMessage {
    pub id: u64,
    pub channels: HashMap<String, u64>,
}