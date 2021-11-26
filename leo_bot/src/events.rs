use crate::actions::{
    action::schedule_action,
    office::{CloseRoomAction, OpenRoomAction},
    subject::SubjectAction,
};
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{channel::Reaction, id::GuildId, prelude::VoiceState},
};

pub(crate) struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn voice_state_update(
        &self,
        context: Context,
        guil_id: Option<GuildId>,
        _: Option<VoiceState>,
        new: VoiceState,
    ) {
        let open_room = OpenRoomAction::new(&context, &guil_id, &new);
        let close_room = CloseRoomAction::new(&context, &new);

        let a1 = schedule_action(open_room);
        let a2 = schedule_action(close_room);

        futures::join!(a1, a2);
    }

    async fn reaction_add(&self, context: Context, reaction: Reaction) {
        let open_subject = SubjectAction::new(&context, &reaction, true);

        schedule_action(open_subject).await;
    }

    async fn reaction_remove(&self, context: Context, reaction: Reaction) {
        let close_subject = SubjectAction::new(&context, &reaction, false);

        schedule_action(close_subject).await;
    }
}
