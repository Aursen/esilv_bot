use async_trait::async_trait;

/// Give some structure to each feature
/// It's necessary to put #[async_trait] for each implementation
#[async_trait]
pub(crate) trait Action {
    async fn can_execute(&self) -> bool;
    async fn execute(&self);
}

//TODO error handling
pub(crate) async fn schedule_action<A: Action>(action: A) {
    if action.can_execute().await {
        action.execute().await;
    }
}
