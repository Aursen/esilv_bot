use crate::{actions::action::Action, get_config_lock, models::SubjectsMessage};
use async_trait::async_trait;
use serenity::{
    client::Context,
    model::{
        channel::{PermissionOverwrite, PermissionOverwriteType, Reaction},
        id::{ChannelId, UserId},
        Permissions,
    },
};

/// Action to open and close subject's channel
pub(crate) struct SubjectAction<'a> {
    context: &'a Context,
    reaction: &'a Reaction,
    open: bool,
}

/// Implement utility functions for action
impl<'a> SubjectAction<'a> {
    pub(crate) fn new(context: &'a Context, reaction: &'a Reaction, open: bool) -> Self {
        SubjectAction {
            context,
            reaction,
            open,
        }
    }

    async fn get_channel_and_user(&self, subject: &SubjectsMessage) -> Option<(u64, UserId)> {
        let emoji = self.reaction.emoji.as_data();
        let channel = subject.channels.get(&emoji)?;
        let user = self.reaction.user_id?;

        Some((*channel, user))
    }

    async fn close_channel(&self, channel: u64, user: UserId) {
        ChannelId(channel)
            .delete_permission(self.context, PermissionOverwriteType::Member(user))
            .await
            .ok();
    }

    async fn open_channel(&self, channel: u64, user: UserId) {
        let allow = Permissions::READ_MESSAGES;
        let overwrite = PermissionOverwrite {
            allow,
            deny: Permissions::default(),
            kind: PermissionOverwriteType::Member(user),
        };

        ChannelId(channel)
            .create_permission(self.context, &overwrite)
            .await
            .ok();
    }
}

/// Implement the action trait
#[async_trait]
impl Action for SubjectAction<'_> {
    async fn can_execute(&self) -> bool {
        let config_lock = get_config_lock(self.context).await;
        let config = config_lock.read().await;

        config
            .subjects
            .iter()
            .any(|s| s.id == self.reaction.message_id.0)
    }

    async fn execute(&self) {
        let config_lock = get_config_lock(self.context).await;
        let config = config_lock.read().await;

        if let Some(s) = config
            .subjects
            .iter()
            .find(|e| e.id == self.reaction.message_id.0)
        {
            if let Some((channel, user)) = self.get_channel_and_user(s).await {
                match self.open {
                    true => self.open_channel(channel, user).await,
                    false => self.close_channel(channel, user).await,
                }
            }
        }
    }
}
