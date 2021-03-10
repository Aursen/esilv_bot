use crate::Config;
use leo_shared::MongoClient;
use leo_shared::Room;
use serenity::model::channel::ChannelType;
use serenity::model::prelude::GuildId;
use serenity::model::voice::VoiceState;
use serenity::{
    model::{
        permissions::Permissions,
        prelude::{ChannelId, PermissionOverwrite, PermissionOverwriteType, RoleId},
    },
    prelude::*,
};
use tokio::sync::RwLockReadGuard;

// Uses to remove room in bdd and in Discord
pub async fn remove_room(context: &Context, db: &MongoClient, room: &Room) {
    let _ = db.remove_room(room).await;
    let _ = ChannelId(room.get_office_id()).delete(&context).await;
    let _ = ChannelId(room.get_waiting_id()).delete(&context).await;
    let _ = ChannelId(room.get_text_id()).delete(&context).await;
}

// Uses to create room in bdd and in Discord
pub async fn create_room(
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
            allow: Permissions::READ_MESSAGES,
            deny: Permissions::CONNECT,
            kind: PermissionOverwriteType::Role(RoleId(*config.roles.get("everyone").unwrap())),
        },
    ];

    let permissions_text = vec![
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

    let member = new.member.as_ref().unwrap();

    let prof_name = member
        .nick
        .as_ref()
        .unwrap_or(&member.user.name)
        .to_string();
    let new_channel = guild
        .create_channel(&context, |c| {
            c.name(format!("🔊 {}", prof_name))
                .category(config.teacher_category)
                .permissions(permissions_office)
                .user_limit(2)
                .kind(ChannelType::Voice)
        })
        .await
        .unwrap();
    let text_channel = guild
        .create_channel(&context, |c| {
            c.name(format!("💬 {}", prof_name))
                .category(config.teacher_category)
                .permissions(permissions_text)
                .kind(ChannelType::Text)
        })
        .await
        .unwrap();

    let waiting = guild
        .create_channel(&context, |c| {
            c.name(format!("⏳ {}", prof_name))
                .category(config.teacher_category)
                .permissions(permissions_waiting)
                .user_limit(5)
                .kind(ChannelType::Voice)
        })
        .await
        .unwrap();

    let _ = guild
        .move_member(&context, new.user_id, new_channel.id)
        .await;

    let bdd_result = MongoClient::init().await;

    let room = Room::new(
        new.user_id.0,
        new_channel.id.0,
        waiting.id.0,
        text_channel.id.0,
    );

    if let Ok(bdd) = bdd_result {
        let _ = bdd.add_room(&room).await;
    }
}
