use serenity::{
    client::Context,
    model::{
        prelude::{PermissionOverwrite, PermissionOverwriteType, Reaction, ChannelId, ReactionType},
        Permissions,
    },
};
use crate::SubjectsMessage;

pub async fn open_subject_channel(context: &Context, reaction: Reaction, subject: &SubjectsMessage) {
    let emoji = match reaction.emoji {
        ReactionType::Custom { name, .. } => name,
        ReactionType::Unicode(c) => Some(c),
        _ => None,
    };
    if let Some(e) = emoji {
        if let Some(c) = subject.channels.get(&e) {
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

pub async fn close_subject_channel(context: &Context, reaction: Reaction, subject: &SubjectsMessage) {
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
        if let Some(c) = subject.channels.get(&e) {
            if let Some(u) = reaction.user_id {
                let _ = ChannelId(*c)
                    .delete_permission(&context, PermissionOverwriteType::Member(u))
                    .await;
            }
        }
    }
}
