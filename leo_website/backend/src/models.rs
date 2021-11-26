use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub(crate) aud: String,
    pub(crate) iss: String,
    pub(crate) iat: usize,
    pub(crate) exp: usize,
    pub(crate) email: String,
    pub(crate) family_name: String,
    pub(crate) given_name: String,
    pub(crate) sub: String,
    pub(crate) group: Option<String>,
    pub(crate) auth_time: String,
    pub(crate) authmethod: String,
    pub(crate) ver: String,
    pub(crate) appid: String,
}

#[crud_table(table_name:"users")]
#[derive(Debug, Serialize, Deserialize)]
pub struct DevinciUser {
    pub(crate) discord_id: u64,
    pub(crate) first_name: String,
    pub(crate) last_name: String,
    pub(crate) mail: String,
    pub(crate) func: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DevinciType {
    Student(u8),
    Professor,
    Other,
}

impl From<u8> for DevinciType {
    fn from(value: u8) -> Self {
        match value {
            0 => DevinciType::Professor,
            x if (1..=5).contains(&x) => DevinciType::Student(x),
            _ => DevinciType::Other,
        }
    }
}

impl From<DevinciType> for u8 {
    fn from(value: DevinciType) -> u8 {
        match value {
            DevinciType::Professor => 0,
            DevinciType::Student(year) => year,
            DevinciType::Other => 6,
        }
    }
}
