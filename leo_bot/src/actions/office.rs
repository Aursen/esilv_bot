use crate::{actions::action::Action, get_config_lock, get_rooms_lock, models::Room};
use async_trait::async_trait;
use serenity::{
    client::Context,
    model::{
        channel::ChannelType,
        id::{ChannelId, GuildId},
        prelude::VoiceState,
    },
};

/// Action to open teachers' rooms
pub(crate) struct OpenRoomAction<'a> {
    context: &'a Context,
    guild_id: &'a Option<GuildId>,
    voice: &'a VoiceState,
}

/// Implement utility functions for action
impl<'a> OpenRoomAction<'a> {
    pub(crate) fn new(
        context: &'a Context,
        guild_id: &'a Option<GuildId>,
        voice: &'a VoiceState,
    ) -> Self {
        OpenRoomAction {
            context,
            guild_id,
            voice,
        }
    }

    async fn move_user(&self, guild_id: &GuildId, office_id: u64) {
        guild_id
            .move_member(self.context, self.voice.user_id, office_id)
            .await
            .unwrap();
    }

    async fn create_rooms(&self, guild_id: &GuildId) -> Result<Room, serenity::Error> {
        let config_lock = get_config_lock(self.context).await;
        let config = config_lock.read().await;

        let office = guild_id
            .create_channel(self.context, |c| {
                c.name("Channel test".to_string())
                    .category(config.teacher_category)
                    .permissions(Vec::new())
                    .kind(ChannelType::Voice)
            })
            .await?;

        Ok(Room {
            discord_id: self.voice.user_id.0,
            office_id: office.id.0,
            waiting_id: 0,
            text_id: 0,
        })
    }
}

/// Implement the action trait
#[async_trait]
impl Action for OpenRoomAction<'_> {
    async fn can_execute(&self) -> bool {
        let config_lock = get_config_lock(self.context).await;
        let config = config_lock.read().await;

        match self.voice.channel_id {
            Some(id) => (config.room == id.0) && self.guild_id.is_some(),
            None => false,
        }
    }

    async fn execute(&self) {
        let rooms_lock = get_rooms_lock(self.context).await;
        let rooms = rooms_lock.read().await;

        if let Some(guild_id) = self.guild_id {
            match rooms.iter().find(|r| r.discord_id == self.voice.user_id.0) {
                Some(room) => self.move_user(guild_id, room.office_id).await,
                None => {
                    if let Ok(room) = self.create_rooms(guild_id).await {
                        self.move_user(guild_id, room.office_id).await;

                        drop(rooms); //We need to drop LockReadGuard before write a new value
                        let mut room_storage = rooms_lock.write().await;
                        room_storage.push(room);
                    }
                }
            }
        }
    }
}

/// Action to close teachers' rooms
pub(crate) struct CloseRoomAction<'a> {
    context: &'a Context,
    new: &'a VoiceState,
}

/// Implement utility functions for action
impl<'a> CloseRoomAction<'a> {
    pub(crate) fn new(context: &'a Context, new: &'a VoiceState) -> Self {
        CloseRoomAction { context, new }
    }

    async fn delete_rooms(&self, room: &Room) -> Result<(), serenity::Error> {
        ChannelId(room.office_id).delete(self.context).await?;

        Ok(())
    }
}

/// Implement the action trait
#[async_trait]
impl Action for CloseRoomAction<'_> {
    async fn can_execute(&self) -> bool {
        let lock = get_rooms_lock(self.context).await;
        let room_storage = lock.read().await;

        let has_room = room_storage
            .iter()
            .any(|e| e.discord_id == self.new.user_id.0);

        matches!(self.new.channel_id, None if has_room)
    }

    async fn execute(&self) {
        let lock = get_rooms_lock(self.context).await;
        let room_storage = lock.read().await;

        if let Some(index) = room_storage
            .iter()
            .position(|e| e.discord_id == self.new.user_id.0)
        {
            if self.delete_rooms(&room_storage[index]).await.is_ok() {
                drop(room_storage);
                let mut room_storage = lock.write().await;
                room_storage.remove(index);
            }
        }
    }
}
