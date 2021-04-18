pub mod user;

use crate::user::DevinciUser;
use mongodb::{
    bson::doc, error::Error, options::ClientOptions, Client, Database,
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
            Some(u) => Ok(Some(mongodb::bson::from_document::<DevinciUser>(u)?)),
            None => Ok(None),
        }
    }

    // Adds user in the database
    pub async fn add_user(&self, id: u64, user: &mut DevinciUser) -> Result<(), Error> {
        let collection = self.db.collection("users");
        user.set_discord_id(id);
        collection
            .insert_one(mongodb::bson::to_document(user)?, None)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::user::{DevinciType, DevinciUser};

    #[test]
    fn serialize_deserialize_user_test() {
        let user = DevinciUser::new(
            String::from("test"),
            String::from("test"),
            String::from("test@devinci.fr"),
            DevinciType::Professor,
        );
        mongodb::bson::from_document::<DevinciUser>(mongodb::bson::to_document(&user).unwrap()).unwrap();
    }
}
