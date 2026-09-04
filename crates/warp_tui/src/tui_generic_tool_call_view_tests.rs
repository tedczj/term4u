use std::cell::RefCell;

use futures::channel::oneshot;
use warp::tui_export::{
    AIAgentAction, AIAgentActionId, AIAgentActionResultType, AIAgentActionType, AIConversationId,
    BlocklistAIActionEvent, SuggestNewConversationResult, TaskId, queue_tui_permission_action,
};
use warp_core::execution_mode::{AppExecutionMode, ExecutionMode};
use warpui::platform::WindowStyle;
use warpui::{AddWindowOptions, App};

use super::TuiGenericToolCallView;
use crate::test_fixtures::{TestHostView, add_test_action_model};

#[test]
fn accepting_new_conversation_suggestion_completes_the_executor() {
    App::test((), |mut app| async move {
        app.add_singleton_model(|ctx| AppExecutionMode::new(ExecutionMode::App, false, ctx));
        let action_model = add_test_action_model(&mut app);
        let conversation_id = AIConversationId::new();
        let action = AIAgentAction {
            id: AIAgentActionId::from("suggest-conversation".to_owned()),
            task_id: TaskId::new("task".to_owned()),
            action: AIAgentActionType::SuggestNewConversation {
                message_id: "next-step".to_owned(),
            },
            requires_result: true,
        };
        let action_for_queue = action.clone();
        let action_id = action.id.clone();
        let action_model_for_view = action_model.clone();
        let view = app.update(|ctx| {
            let (window_id, _) = ctx.add_tui_window(
                AddWindowOptions {
                    window_style: WindowStyle::NotStealFocus,
                    ..Default::default()
                },
                |_| TestHostView,
            );
            ctx.add_tui_view(window_id, |ctx| {
                TuiGenericToolCallView::new(
                    action,
                    false,
                    action_model_for_view,
                    conversation_id,
                    ctx,
                )
            })
        });
        let (finished_tx, finished_rx) = oneshot::channel();
        let finished_tx = RefCell::new(Some(finished_tx));
        app.update(|ctx| {
            ctx.subscribe_to_model(&action_model, move |_, event, _| {
                if matches!(
                    event,
                    BlocklistAIActionEvent::FinishedAction { action_id: id, .. } if id == &action_id
                ) && let Some(tx) = finished_tx.borrow_mut().take()
                {
                    let _ = tx.send(());
                }
            });
        });
        action_model.update(&mut app, |model, ctx| {
            queue_tui_permission_action(model, action_for_queue, conversation_id, ctx);
        });

        view.update(&mut app, |view, ctx| view.accept(ctx));
        finished_rx
            .await
            .expect("accepted suggestion should reach a terminal result");

        app.read(|ctx| {
            let result = action_model
                .as_ref(ctx)
                .get_action_result(&AIAgentActionId::from("suggest-conversation".to_owned()))
                .expect("suggestion result");
            assert!(matches!(
                &result.result,
                AIAgentActionResultType::SuggestNewConversation(
                    SuggestNewConversationResult::Accepted { message_id }
                ) if message_id == "next-step"
            ));
        });
    });
}
