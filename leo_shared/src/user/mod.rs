use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum DevinciType {
    Student(i32),
    Professor,
    Other
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DevinciUser {
    discord_id: i64,
    first_name: String,
    last_name: String,
    mail: String,
    func: DevinciType,
}

impl DevinciUser {
    pub fn new(first_name: String, last_name: String, mail: String, func: DevinciType) -> Self {
        Self {
            discord_id: i64::default(),
            first_name,
            last_name,
            mail,
            func
        }
    }

    pub fn set_discord_id(&mut self, id: u64) {
        self.discord_id = id as i64;
    }

    pub fn get_type(&self) -> &DevinciType {
        &self.func
    }

    pub fn get_name(&self) -> (&str, &str){
        (&self.first_name, &self.last_name)
    }
}
