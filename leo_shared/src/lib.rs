pub mod user;

use crate::user::DevinciUser;
use mongodb::bson::doc;
use mongodb::error::Error;
use mongodb::options::ClientOptions;
use mongodb::results::DeleteResult;
use mongodb::Client;
use mongodb::Database;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
pub struct Room {
    discord_id: i64,
    office_id: i64,
    waiting_id: i64,
}

impl Room {
    pub fn new(discord_id: u64, office_id: u64, waiting_id: u64) -> Self {
        Self {
            discord_id: discord_id as i64,
            office_id: office_id as i64,
            waiting_id: waiting_id as i64,
        }
    }

    pub fn get_office_id(&self) -> u64 {
        self.office_id as u64
    }

    pub fn get_waiting_id(&self) -> u64 {
        self.waiting_id as u64
    }
}

#[derive(Debug)]
pub struct MongoClient {
    db: Database,
}

impl MongoClient {
    pub async fn init() -> Result<Self, Error> {
        let client_uri =
            env::var("MONGODB_URI").expect("You must set the MONGODB_URI environment var!");
        let mut client_options = ClientOptions::parse(&client_uri).await?;
        client_options.app_name = Some("leo_app".to_string());
        let client = Client::with_options(client_options)?;
        let db = client.database("leobot");

        Ok(Self { db })
    }

    pub async fn get_user(&self, id: u64) -> Result<Option<DevinciUser>, Error> {
        let collection = self.db.collection("users");
        let doc = doc! { "discord_id": id };
        let user = collection.find_one(doc, None).await?;
        match user {
            Some(u) => Ok(Some(bson::from_document::<DevinciUser>(u)?)),
            None => Ok(None)
        }
    }

    pub async fn add_user(&self, id: u64, user: &mut DevinciUser) -> Result<(), Error> {
        let collection = self.db.collection("users");
        user.set_discord_id(id);
        collection
            .insert_one(bson::to_document(user)?, None)
            .await?;
        Ok(())
    }

    pub async fn get_room(&self, discord_id: u64) -> Result<Option<Room>, Error> {
        let collection = self.db.collection("rooms");
        let doc = doc! { "discord_id": discord_id };
        let room = collection.find_one(doc.clone(), None).await?;
        match room {
            Some(r) => Ok(Some(bson::from_document::<Room>(r)?)),
            None => Ok(None)
        }
    }

    pub async fn add_room(&self, room: &Room) -> Result<(), Error> {
        let collection = self.db.collection("rooms");
        collection
            .insert_one(bson::to_document(room)?, None)
            .await?;
        Ok(())
    }

    pub async fn remove_room(&self, room: &Room) -> Result<DeleteResult, Error> {
        let collection = self.db.collection("rooms");
        collection.delete_one(bson::to_document(room)?, None).await
    }
}

#[cfg(test)]
mod tests {

use crate::user::{DevinciUser, DevinciType};
use crate::Room;

    #[test]
    fn serialize_deserialize_room_test() {
        let room = Room::new(0, 0, 0);
        let new_room = bson::from_document::<Room>(bson::to_document(&room).unwrap()).unwrap();

        assert_eq!(0, new_room.get_office_id());
        assert_eq!(0, new_room.get_waiting_id());
    }

    #[test]
    fn serialize_deserialize_user_test() {
        let user = DevinciUser::new(String::from("test"), String::from("test"), String::from("test@devinci.fr"), DevinciType::Professor);
        bson::from_document::<DevinciUser>(bson::to_document(&user).unwrap()).unwrap();
    }
}
