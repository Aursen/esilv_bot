pub mod user;
pub mod room;

use crate::{user::DevinciUser, room::Room};
use mongodb::{
    bson::doc, error::Error, options::ClientOptions, results::DeleteResult, Client, Database,
};
use std::env;

#[derive(Debug)]
pub struct MongoClient {
    db: Database,
}

impl MongoClient {
    // Inits the database connection
    pub async fn init() -> Result<Self, Error> {
        let client_uri =
            env::var("MONGODB_URI").expect("You must set the MONGODB_URI environment var!");
        let mut client_options = ClientOptions::parse(&client_uri).await?;
        client_options.app_name = Some("leo_app".to_string());
        let client = Client::with_options(client_options)?;
        let db = client.database("leobot");

        Ok(Self { db })
    }

    // Check if mail used
    pub async fn check_mail(&self, mail: &str) -> Result<bool, Error> {
        let collection = self.db.collection("users");
        let doc = doc! { "mail": mail };

        let user = collection.find_one(doc, None).await?;
        match user {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    // Gets user in the database
    pub async fn get_user(&self, id: u64) -> Result<Option<DevinciUser>, Error> {
        let collection = self.db.collection("users");
        let doc = doc! { "discord_id": id };

        let user = collection.find_one(doc, None).await?;
        match user {
            Some(u) => Ok(Some(bson::from_document::<DevinciUser>(u)?)),
            None => Ok(None),
        }
    }

    // Adds user in the database
    pub async fn add_user(&self, id: u64, user: &mut DevinciUser) -> Result<(), Error> {
        let collection = self.db.collection("users");
        user.set_discord_id(id);
        collection
            .insert_one(bson::to_document(user)?, None)
            .await?;
        Ok(())
    }

    // Gets room in the database
    pub async fn get_room_by_user(&self, discord_id: u64) -> Result<Option<Room>, Error> {
        let collection = self.db.collection("rooms");
        let doc = doc! { "discord_id": discord_id };
        let room = collection.find_one(doc.clone(), None).await?;
        match room {
            Some(r) => Ok(Some(bson::from_document::<Room>(r)?)),
            None => Ok(None),
        }
    }

    pub async fn get_room_by_channel(&self, channel_id: u64) -> Result<Option<Room>, Error> {
        let collection = self.db.collection("rooms");
        let doc = doc! { "office_id": channel_id };
        let room = collection.find_one(doc.clone(), None).await?;
        match room {
            Some(r) => Ok(Some(bson::from_document::<Room>(r)?)),
            None => Ok(None),
        }
    }

    // Adds room in the database
    pub async fn add_room(&self, room: &Room) -> Result<(), Error> {
        let collection = self.db.collection("rooms");
        collection
            .insert_one(bson::to_document(room)?, None)
            .await?;
        Ok(())
    }

    // Removes room in the database
    pub async fn remove_room(&self, room: &Room) -> Result<DeleteResult, Error> {
        let collection = self.db.collection("rooms");
        collection.delete_one(bson::to_document(room)?, None).await
    }
}

#[cfg(test)]
mod tests {

    use crate::user::{DevinciType, DevinciUser};
    use crate::Room;

    #[test]
    fn serialize_deserialize_room_test() {
        let room = Room::new(0, 0, 0, 0);
        let new_room = bson::from_document::<Room>(bson::to_document(&room).unwrap()).unwrap();

        assert_eq!(0, new_room.get_office_id());
        assert_eq!(0, new_room.get_waiting_id());
        assert_eq!(0, new_room.get_text_id());
    }

    #[test]
    fn serialize_deserialize_user_test() {
        let user = DevinciUser::new(
            String::from("test"),
            String::from("test"),
            String::from("test@devinci.fr"),
            DevinciType::Professor,
        );
        bson::from_document::<DevinciUser>(bson::to_document(&user).unwrap()).unwrap();
    }
}
