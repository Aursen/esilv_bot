use crate::models::config::SubjectsMessage;
use serenity::model::prelude::UserId;
use serenity::{
    client::Context,
    model::{
        prelude::{
            ChannelId, PermissionOverwrite, PermissionOverwriteType, Reaction, ReactionType,
        },
        Permissions,
    },
};

pub async fn open_subject_channel(
    context: &Context,
    reaction: Reaction,
    subject: &SubjectsMessage,
) {
    if let Some((channel, user)) = get_channel_and_user(reaction, subject).await {
        let allow = Permissions::READ_MESSAGES;
        let overwrite = PermissionOverwrite {
            allow,
            deny: Permissions::default(),
            kind: PermissionOverwriteType::Member(user),
        };
        let _ = ChannelId(channel)
            .create_permission(&context, &overwrite)
            .await;
    }
}

pub async fn close_subject_channel(
    context: &Context,
    reaction: Reaction,
    subject: &SubjectsMessage,
) {
    if let Some((channel, user)) = get_channel_and_user(reaction, subject).await {
        let _ = ChannelId(channel)
            .delete_permission(&context, PermissionOverwriteType::Member(user))
            .await;
    }
}

async fn get_channel_and_user(
    reaction: Reaction,
    subject: &SubjectsMessage,
) -> Option<(u64, UserId)> {
    let emoji = match reaction.emoji {
        ReactionType::Custom { name, .. } => name,
        ReactionType::Unicode(c) => Some(c),
        _ => None,
    }?;
    let channel = subject.channels.get(&emoji)?;
    let user = reaction.user_id?;

    Some((*channel, user))
}
