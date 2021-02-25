mod commands;
mod utils;

use crate::utils::room::{create_room,remove_room};
use leo_shared::{MongoClient, user::DevinciType};
use serenity::{
    async_trait,
    client::bridge::gateway::{GatewayIntents, ShardManager},
    framework::{standard::macros::group, StandardFramework},
    http::Http,
    model::{
        channel::ReactionType,
        event::ResumedEvent,
        gateway::Ready,
        permissions::Permissions,
        voice::VoiceState,
        prelude::{Message,GuildId,ChannelId, PermissionOverwrite, PermissionOverwriteType, Reaction, RoleId},
    },
    prelude::*,
};
use std::{collections::{HashSet,HashMap}, env, sync::Arc};


use tracing::{error, info};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use commands::owner::*;
use serde::{Deserialize, Serialize};

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn reaction_add(&self, context: Context, reaction: Reaction) {
        let config_lock = {
            let data_read = context.data.read().await;
            data_read
                .get::<ExternalConfig>()
                .expect("Expected Config in TypeMap.")
                .clone()
        };

        let config = config_lock.read().await;

        let subject = config
            .subjects
            .iter()
            .find(|e| e.id == reaction.message_id.0);
        if let Some(s) = subject {
            let emoji = match reaction.emoji {
                ReactionType::Custom {
                    id: _,
                    animated: _,
                    name,
                } => name,
                ReactionType::Unicode(c) => Some(c),
                _ => None,
            };
            if let Some(e) = emoji {
                if let Some(c) = s.channels.get(&e) {
                    if let Some(u) = reaction.user_id {
                        let allow = Permissions::READ_MESSAGES;
                        let overwrite = PermissionOverwrite {
                            allow,
                            deny: Permissions::default(),
                            kind: PermissionOverwriteType::Member(u),
                        };
                        let _ = ChannelId(*c).create_permission(&context, &overwrite).await;
                    }
                }
            }
        }
    }

    async fn reaction_remove(&self, context: Context, reaction: Reaction) {
        let config_lock = {
            let data_read = context.data.read().await;
            data_read
                .get::<ExternalConfig>()
                .expect("Expected Config in TypeMap.")
                .clone()
        };

        let config = config_lock.read().await;

        let subject = config
            .subjects
            .iter()
            .find(|e| e.id == reaction.message_id.0);

        if let Some(s) = subject {
            let emoji = match reaction.emoji {
                ReactionType::Custom {
                    id: _,
                    animated: _,
                    name,
                } => name,
                ReactionType::Unicode(c) => Some(c),
                _ => None,
            };
            if let Some(e) = emoji {
                if let Some(c) = s.channels.get(&e) {
                    if let Some(u) = reaction.user_id {
                        let _ = ChannelId(*c)
                            .delete_permission(&context, PermissionOverwriteType::Member(u))
                            .await;
                    }
                }
            }
        }
    }

    async fn voice_state_update(
        &self,
        context: Context,
        guild_id: Option<GuildId>,
        _: Option<VoiceState>,
        new: VoiceState,
    ) {
        let config_lock = {
            let data_read = context.data.read().await;
            data_read
                .get::<ExternalConfig>()
                .expect("Expected Config in TypeMap.")
                .clone()
        };

        let config = config_lock.read().await;

        if let Some(guild) = guild_id {
            if let Some(channel) = &new.channel_id {
                if channel.0 == config.room {
                    let bdd_result = MongoClient::init().await;
                    if let Ok(bdd) = bdd_result {
                        let room = bdd.get_room(new.user_id.0).await.unwrap_or(None);
                        if let Some(r) = room {
                            let _ = guild
                                .move_member(&context, new.user_id, r.get_office_id())
                                .await;
                        } else {
                            create_room(&context, guild, &new, config).await;
                        }
                    }
                }
            }

            if new.channel_id.is_none() {
                let bdd_result = MongoClient::init().await;
                if let Ok(bdd) = bdd_result {
                    let room = bdd.get_room(new.user_id.0).await.unwrap_or(None);
                    if let Some(r) = room {
                        remove_room(&context, &bdd, &r).await;
                    }
                }
            }
        }
    }

    async fn message(&self, ctx: Context, new_message: Message) {
        let config_lock = {
            let data_read = ctx.data.read().await;
            data_read
                .get::<ExternalConfig>()
                .expect("Expected Config in TypeMap.")
                .clone()
        };

        let config = config_lock.read().await;

        if *new_message.channel_id.as_u64() == config.webhook {
            if let Some(g) = new_message.guild_id {
                let user_id = new_message.content.parse::<u64>();
                if let Ok(u) = user_id {
                    let role = RoleId::from(config.roles["verified"]);

                    let bdd_result = MongoClient::init().await;
                    if let Ok(bdd) = bdd_result {
                        let user = bdd.get_user(u).await.unwrap_or(None);
                        if let Some(bdd_user) = user {
                            let mut roles = vec![role];
                            match bdd_user.get_type() {
                                DevinciType::Student(_) => {
                                    roles.push(RoleId::from(config.roles["a1"]))
                                }
                                DevinciType::Professor => {
                                    roles.push(RoleId::from(config.roles["teacher"]))
                                }
                                DevinciType::Other => (),
                            }
                            let (first_name, last_name) = bdd_user.get_name();
                            let _ = g
                                .edit_member(&ctx, u, |m| {
                                    m.roles(&roles)
                                        .nickname(format!("{} {}", first_name, last_name))
                                })
                                .await;
                        }
                    }
                }
            }
        }
    }
    
    async fn ready(&self, _: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

struct ExternalConfig;

impl TypeMapKey for ExternalConfig {
    type Value = Arc<RwLock<Config>>;
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    roles: HashMap<String, u64>,
    webhook: u64,
    room: u64,
    teacher_category: u64,
    subjects: Vec<SubjectsMessage>,
}

#[derive(Serialize, Deserialize)]
pub struct SubjectsMessage {
    id: u64,
    channels: HashMap<String, u64>,
}

#[group]
#[commands(quit)]
struct General;

#[tokio::main]
async fn main() {
    if cfg!(debug_assertions) {
        dotenv::dotenv().expect("Failed to load .env file.");
        let subscriber = FmtSubscriber::builder()
            .with_env_filter(EnvFilter::from_default_env())
            .finish();

        tracing::subscriber::set_global_default(subscriber).expect("Failed to start the logger.");
    }

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment.");

    let http = Http::new_with_token(&token);

    // We will fetch your bot's owners and id
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}.", why),
    };

    // Create the framework
    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).prefix("~"))
        .group(&GENERAL_GROUP);

    let mut client = Client::builder(&token)
        .framework(framework)
        .event_handler(Handler)
        .intents(
            GatewayIntents::GUILD_MESSAGE_REACTIONS
                | GatewayIntents::GUILD_VOICE_STATES
                | GatewayIntents::GUILD_MEMBERS
                | GatewayIntents::GUILD_MESSAGES,
        ) // Commands are disabled
        .await
        .expect("Err creating client.");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());

        let config: Config = serde_json::from_str(include_str!("config.json")).unwrap();
        data.insert::<ExternalConfig>(Arc::new(RwLock::new(config)));
    }

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler.");
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        error!("Client error: {:?}.", why);
    }
}

