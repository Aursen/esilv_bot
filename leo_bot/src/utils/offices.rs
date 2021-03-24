use crate::models::room::Room;
use crate::Config;
use crate::RoomStorage;
use serenity::{
    model::{
        channel::ChannelType,
        permissions::Permissions,
        prelude::{ChannelId, GuildId, PermissionOverwrite, PermissionOverwriteType, RoleId},
        voice::VoiceState,
    },
    prelude::*,
};
use std::sync::Arc;
use tokio::sync::RwLockReadGuard;

pub async fn handle_teacher_leaving(
    context: &Context,
    new: &VoiceState,
) -> Result<(), SerenityError> {
    let index = get_room_index(context, new).await?;
    let lock = get_room_lock(context).await;
    let mut room_storage = lock.write().await;

    if let Some(i) = index{
        room_storage.remove(i);
    }

    Ok(())
}

// Uses to remove room in bdd and in Discord
async fn get_room_index(context: &Context, new: &VoiceState) -> Result<Option<usize>, SerenityError> {
    let lock = get_room_lock(context).await;
    let room_storage = lock.read().await;

    if new.channel_id.is_none() {
        if let Some(index) = room_storage
            .iter()
            .position(|x| x.get_user_id() == new.user_id.0)
        {
            ChannelId(room_storage[index].get_office_id())
                .delete(&context)
                .await?;
            ChannelId(room_storage[index].get_waiting_id())
                .delete(&context)
                .await?;
            ChannelId(room_storage[index].get_text_id())
                .delete(&context)
                .await?;
            return Ok(Some(index));
        }
    }

    Ok(None)
}

// Uses to create room in bdd and in Discord
pub async fn create_room(
    context: &Context,
    guild: GuildId,
    new: &VoiceState,
    config: RwLockReadGuard<'_, Config>,
) -> Result<Room, SerenityError> {
    let permissions = vec![
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
                .permissions(permissions.clone())
                .user_limit(2)
                .kind(ChannelType::Voice)
        })
        .await?;
    let text_channel = guild
        .create_channel(&context, |c| {
            c.name(format!("💬 {}", prof_name))
                .category(config.teacher_category)
                .permissions(permissions)
                .kind(ChannelType::Text)
        })
        .await?;

    let waiting = guild
        .create_channel(&context, |c| {
            c.name(format!("⏳ {}", prof_name))
                .category(config.teacher_category)
                .permissions(permissions_waiting)
                .user_limit(5)
                .kind(ChannelType::Voice)
        })
        .await?;

    let _ = guild
        .move_member(&context, new.user_id, new_channel.id)
        .await;

    Ok(Room::new(
        new.user_id.0,
        new_channel.id.0,
        waiting.id.0,
        text_channel.id.0,
    ))
}

pub async fn get_room_lock(context: &Context) -> Arc<RwLock<Vec<Room>>> {
    let data_read = context.data.read().await;
    data_read
        .get::<RoomStorage>()
        .expect("Expected RoomStorage in TypeMap.")
        .clone()
}
