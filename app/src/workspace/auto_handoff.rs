use warpui::{AppContext, Entity, ModelContext, SingletonEntity, WindowId};

use super::AutoCloudHandoffTrigger;
use crate::ai::agent::conversation::AIConversationId;

pub(crate) struct AutoCloudHandoffController;

impl AutoCloudHandoffController {
    pub(crate) fn new(ctx: &mut ModelContext<Self>) -> Self {
        let _ = ctx;
        Self
    }

    #[cfg(all(feature = "local_fs", not(target_family = "wasm")))]
    pub(crate) fn record_handoff_succeeded(
        &mut self,
        conversation_id: AIConversationId,
        window_id: WindowId,
        ctx: &mut ModelContext<Self>,
    ) {
        let _ = (conversation_id, window_id, ctx);
    }

    #[cfg(all(feature = "local_fs", not(target_family = "wasm")))]
    pub(crate) fn record_handoff_failed(&mut self, conversation_id: AIConversationId) {
        let _ = conversation_id;
    }
}

impl Entity for AutoCloudHandoffController {
    type Event = ();
}

impl SingletonEntity for AutoCloudHandoffController {}

pub(crate) fn init(app: &mut AppContext) {
    app.add_singleton_model(AutoCloudHandoffController::new);
}

pub(crate) fn trigger_auto_handoff_to_cloud(
    trigger: AutoCloudHandoffTrigger,
    ctx: &mut AppContext,
) {
    let _ = (trigger, ctx);
}
