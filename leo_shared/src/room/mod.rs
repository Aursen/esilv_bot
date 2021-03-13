use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Room {
    discord_id: i64,
    office_id: i64,
    waiting_id: i64,
    text_id: i64
}

impl Room {
    pub fn new(discord_id: u64, office_id: u64, waiting_id: u64, text_id: u64) -> Self {
        Self {
            discord_id: discord_id as i64,
            office_id: office_id as i64,
            waiting_id: waiting_id as i64,
            text_id: text_id as i64
        }
    }

    pub fn get_user_id(&self) -> u64 {
        self.discord_id as u64
    }

    pub fn get_office_id(&self) -> u64 {
        self.office_id as u64
    }

    pub fn get_waiting_id(&self) -> u64 {
        self.waiting_id as u64
    }

    pub fn get_text_id(&self) -> u64 {
        self.text_id as u64
    }
}