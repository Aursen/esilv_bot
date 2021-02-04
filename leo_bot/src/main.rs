mod commands;

use serenity::model::prelude::Member;
use leo_shared::MongoClient;
use leo_shared::Room;
use leo_shared::user::DevinciType;

use serenity::model::channel::ChannelType;
use serenity::model::prelude::GuildId;
use serenity::model::voice::VoiceState;
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
        prelude::{ChannelId, PermissionOverwrite, PermissionOverwriteType, Reaction, RoleId},
    },
    prelude::*,
};
use std::collections::HashMap;
use std::{collections::HashSet, env, sync::Arc};
use tokio::sync::RwLockReadGuard;

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

        if reaction.message_id == config.rules {
            if let Some(g) = reaction.guild_id {
                if let Some(u) = reaction.user_id {
                    let role = RoleId::from(config.roles["verified"]);

                    let client = MongoClient::init().await.unwrap();
                    let user = client.get_user(u.0).await.unwrap();
                    if let Some(bdd_user) = user{
                        let mut roles = vec![role];
                        match bdd_user.get_type() {
                            DevinciType::Student(_) => roles.push(RoleId::from(config.roles["a1"])),
                            DevinciType::Professor => roles.push(RoleId::from(config.roles["teacher"])),
                            DevinciType::Other => (), 
                        }
                        let (first_name, last_name) = bdd_user.get_name();
                        g.edit_member(&context, u, |m| m.roles(&roles).nickname(format!("{} {} | TD-X", first_name, last_name)))
                            .await
                            .unwrap();
                    }
                }
            }
        }

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
                        ChannelId(*c)
                            .create_permission(&context, &overwrite)
                            .await
                            .unwrap();
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
                        ChannelId(*c)
                            .delete_permission(&context, PermissionOverwriteType::Member(u))
                            .await
                            .unwrap();
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
                    let db = MongoClient::init().await.unwrap();
                    let room = db.get_room(new.user_id.0).await.unwrap();
                    if let Some(r) = room {
                        guild
                        .move_member(&context, new.user_id, r.get_office_id())
                        .await
                        .unwrap();
                    }else{
                        create_room(&context, guild, &new, config).await;
                    }
                }
            }

            if new.channel_id.is_none() {
                let db = MongoClient::init().await.unwrap();
                let room = db.get_room(new.user_id.0).await.unwrap();
                if let Some(r) = room {
                    db.remove_room(&r).await.unwrap();
                    info!("{:?}", r);
                    ChannelId(r.get_office_id()).delete(&context).await.unwrap();
                    ChannelId(r.get_waiting_id())
                        .delete(&context)
                        .await
                        .unwrap();
                }
            }
        }
    }

    // Called when a user joins a guild.
    // Sends the connection URL.
    async fn guild_member_addition(&self, context: Context, _: GuildId, new_member: Member){
        new_member.user.dm(&context, format!("Veuillez vous connecter sur: https://leobot.site?id={}", new_member.user.id.0));
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
struct Config {
    roles: HashMap<String, u64>,
    rules: u64,
    room: u64,
    teacher_category: u64,
    subjects: Vec<SubjectsMessage>,
}

#[derive(Serialize, Deserialize)]
struct SubjectsMessage {
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
        .intents(GatewayIntents::GUILD_MESSAGE_REACTIONS | GatewayIntents::GUILD_VOICE_STATES | GatewayIntents::GUILD_MEMBERS) // Commands are disabled
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

async fn create_room(
    context: &Context,
    guild: GuildId,
    new: &VoiceState,
    config: RwLockReadGuard<'_, Config>,
) {
    let permissions_office = vec![
        PermissionOverwrite {
            allow: Permissions::READ_MESSAGES,
            deny: Permissions::default(),
            kind: PermissionOverwriteType::Member(new.user_id),
        },
        PermissionOverwrite {
            allow: Permissions::default(),
            deny: Permissions::READ_MESSAGES,
            kind: PermissionOverwriteType::Role(RoleId(*config.roles.get("everyone").unwrap())),
        },
    ];

    let permissions_waiting = vec![PermissionOverwrite {
        allow: Permissions::READ_MESSAGES,
        deny: Permissions::SPEAK,
        kind: PermissionOverwriteType::Role(RoleId(*config.roles.get("everyone").unwrap())),
    }];

    let new_channel = guild
        .create_channel(&context, |c| {
            c.name("Bureau")
                .category(config.teacher_category)
                .permissions(permissions_office)
                .user_limit(2)
                .kind(ChannelType::Voice)
        })
        .await
        .unwrap();
    let member = new.member.as_ref().unwrap();
    let waiting = guild
        .create_channel(&context, |c| {
            c.name(member.nick.as_ref().unwrap_or(&member.user.name).to_string())
            .category(config.teacher_category)
            .permissions(permissions_waiting)
            .user_limit(5)
            .kind(ChannelType::Voice)
        })
        .await
        .unwrap();

    guild
        .move_member(&context, new.user_id, new_channel.id)
        .await
        .unwrap();

    let bdd = MongoClient::init().await.unwrap();

    let room = Room::new(new.user_id.0, new_channel.id.0, waiting.id.0);
    bdd.add_room(&room).await.unwrap();
}
