mod commands;
mod models;
mod utils;

use crate::{
    models::{config::Config, room::Room},
    utils::{
        offices::{handle_teacher_opening, handle_teacher_leaving},
        subject::{close_subject_channel, open_subject_channel},
    },
};
use leo_shared::{user::DevinciType, MongoClient};
use serenity::{
    async_trait,
    client::bridge::gateway::ShardManager,
    framework::{standard::macros::group, StandardFramework},
    http::Http,
    model::{
        event::ResumedEvent,
        gateway::Ready,
        permissions::Permissions,
        prelude::{
            ChannelId, GuildId, Message, PermissionOverwrite, PermissionOverwriteType, Reaction,
            RoleId,
        },
        voice::VoiceState,
    },
    prelude::*,
};
use std::{
    collections::HashSet,
    env,
    sync::Arc,
};

use tracing::{error, info};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use commands::owner::*;

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn reaction_add(&self, context: Context, reaction: Reaction) {
        let config_lock = {
            context
                .data
                .read()
                .await
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
            open_subject_channel(&context, reaction.clone(), s).await;
        }
    }

    async fn reaction_remove(&self, context: Context, reaction: Reaction) {
        let config_lock = {
            context
                .data
                .read()
                .await
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
            close_subject_channel(&context, reaction.clone(), s).await;
        }
    }

    async fn voice_state_update(
        &self,
        context: Context,
        guild_id: Option<GuildId>,
        old: Option<VoiceState>,
        new: VoiceState,
    ) {
        let config_lock = {
            context
                .data
                .read()
                .await
                .get::<ExternalConfig>()
                .expect("Expected Config in TypeMap.")
                .clone()
        };
        let _ = handle_teacher_leaving(&context, &new).await;

        if let Some(guild) = guild_id {
            if handle_teacher_opening(config_lock.clone(), &context, &guild, &new).await.is_ok() {
                return
            }

            let lock = get_room_lock(&context).await;
            let room_storage = lock.read().await;

            if let Some(channel) = &new.channel_id {
                if let Some(room) = room_storage.iter().find(|e| e.get_office_id() == channel.0) {
                    let _ = ChannelId(room.get_text_id()).create_permission(
                        &context,
                        &PermissionOverwrite {
                            allow: Permissions::SEND_MESSAGES,
                            deny: Permissions::default(),
                            kind: PermissionOverwriteType::Member(new.user_id),
                        },
                    );
                    return
                }
            }
        }

        if let Some(o) = old {
            
            let lock = get_room_lock(&context).await;
            let room_storage = lock.read().await;

            if let Some(channel) = o.channel_id {
                if let Some(room) = room_storage.iter().find(|e| e.get_office_id() == channel.0) {
                    let _ = ChannelId(room.get_text_id()).create_permission(
                        &context,
                        &PermissionOverwrite {
                            allow: Permissions::default(),
                            deny: Permissions::SEND_MESSAGES,
                            kind: PermissionOverwriteType::Member(o.user_id),
                        },
                    );
                    return;
                }
            }
        }
    }

    async fn message(&self, ctx: Context, new_message: Message) {
        let config_lock = {
            ctx.data
                .read()
                .await
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

pub struct RoomStorage;
impl TypeMapKey for RoomStorage {
    type Value = Arc<RwLock<Vec<Room>>>;
}

struct ExternalConfig;
impl TypeMapKey for ExternalConfig {
    type Value = Arc<RwLock<Config>>;
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
        .await
        .expect("Err creating client.");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());

        let config_file = if cfg!(debug_assertions) {
            include_str!("config-dev.json")
        } else {
            include_str!("config.json")
        };

        let config: Config = serde_json::from_str(config_file).unwrap();
        data.insert::<ExternalConfig>(Arc::new(RwLock::new(config)));

        data.insert::<RoomStorage>(Arc::new(RwLock::new(Vec::new())));
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

pub async fn get_room_lock(context: &Context) -> Arc<RwLock<Vec<Room>>> {
    let data_read = context.data.read().await;
    data_read
        .get::<RoomStorage>()
        .expect("Expected RoomStorage in TypeMap.")
        .clone()
}
