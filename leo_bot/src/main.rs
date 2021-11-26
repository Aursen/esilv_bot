mod actions;
mod events;
mod models;

use crate::{
    events::Handler,
    models::{Config, Room},
};
use serenity::{
    client::{Client, Context},
    prelude::{RwLock, TypeMapKey},
};
use shared_lib::socket::client::tcp_client;
use std::{env, fs::File, sync::Arc};

struct ExternalConfig;
impl TypeMapKey for ExternalConfig {
    type Value = Arc<RwLock<Config>>;
}

pub struct RoomStorage;
impl TypeMapKey for RoomStorage {
    type Value = Arc<RwLock<Vec<Room>>>;
}

#[actix_web::main]
async fn main() {
    dotenv::dotenv().ok();

    let addr = tcp_client("127.0.0.1:1234").await;

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("token");
    let mut client = Client::builder(token)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    {
        let mut data = client.data.write().await;
        let file = File::open("config.json").expect("config file");
        let config: Config = serde_json::from_reader(file).unwrap();

        data.insert::<ExternalConfig>(Arc::new(RwLock::new(config)));
        data.insert::<RoomStorage>(Arc::new(RwLock::new(Vec::new())));
    }

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

//TODO generic lock
async fn get_config_lock(context: &Context) -> Arc<RwLock<Config>> {
    let data_read = context.data.read().await;
    data_read
        .get::<ExternalConfig>()
        .expect("Expected Config in TypeMap.")
        .clone()
}

async fn get_rooms_lock(context: &Context) -> Arc<RwLock<Vec<Room>>> {
    let data_read = context.data.read().await;
    data_read
        .get::<RoomStorage>()
        .expect("Expected Room in TypeMap.")
        .clone()
}
