use crate::ai::agent::conversation::{AIConversation, ConversationStatus};
use crate::ai::agent::{
    AIAgentOutputStatus, CancellationReason, FinishedAIAgentOutput, RenderableAIError,
};

pub mod spawn {
    use std::sync::Arc;
    use std::time::Duration;

    use futures::{Stream, stream};
    use warp_terminal::session_sharing_types::common::SessionId;

    use super::{AmbientAgentTask, AmbientAgentTaskId};
    use crate::server::server_api::ai::{AIClient, SpawnAgentRequest};

    pub const TASK_STATUS_POLLING_DURATION: Duration = Duration::from_secs(80);

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct SessionJoinInfo {
        pub session_id: Option<SessionId>,
        pub session_link: String,
    }

    impl SessionJoinInfo {
        pub fn from_task(task: &AmbientAgentTask) -> Option<Self> {
            let _ = task;
            None
        }
    }

    #[derive(Debug)]
    pub struct AmbientAgentEvent;

    fn unavailable_stream() -> impl Stream<Item = Result<AmbientAgentEvent, anyhow::Error>> {
        stream::once(async { Err(anyhow::anyhow!("Cloud agents are unavailable in term4u")) })
    }

    pub fn spawn_task(
        request: SpawnAgentRequest,
        ai_client: Arc<dyn AIClient>,
        timeout: Option<Duration>,
    ) -> impl Stream<Item = Result<AmbientAgentEvent, anyhow::Error>> {
        let _ = (request, ai_client, timeout);
        unavailable_stream()
    }

    pub fn monitor_spawned_task(
        task_id: AmbientAgentTaskId,
        run_id: String,
        at_capacity: bool,
        ai_client: Arc<dyn AIClient>,
        timeout: Option<Duration>,
    ) -> impl Stream<Item = Result<AmbientAgentEvent, anyhow::Error>> {
        let _ = (task_id, run_id, at_capacity, ai_client, timeout);
        unavailable_stream()
    }

    pub fn submit_run_followup(
        message: String,
        run_id: AmbientAgentTaskId,
        previous_session_id: Option<SessionId>,
        ai_client: Arc<dyn AIClient>,
        timeout: Option<Duration>,
    ) -> impl Stream<Item = Result<AmbientAgentEvent, anyhow::Error>> {
        let _ = (message, run_id, previous_session_id, ai_client, timeout);
        unavailable_stream()
    }
}
#[path = "task_types.rs"]
pub mod task;
pub mod telemetry {
    pub use crate::server::telemetry::cloud_agent_events::*;
}

pub use task::{
    AgentConfigSnapshot, AgentSource, AmbientAgentLiveSessionState, AmbientAgentTask,
    AmbientAgentTaskState, ExecutionLocation, cancel_task_silently, cancel_task_with_toast,
};
pub const OUT_OF_CREDITS_TASK_FAILURE_MESSAGE: &str =
    "Out of credits. Upgrade your Warp plan to continue running cloud agents.";
pub const SERVER_OVERLOADED_TASK_FAILURE_MESSAGE: &str =
    "Warp is temporarily overloaded. Please try again shortly.";

pub use ai_types::AmbientAgentTaskId;

/// High-level outcome of an ambient agent conversation.
#[derive(Clone, Debug)]
pub enum AmbientConversationStatus {
    Success,
    Error {
        error: RenderableAIError,
    },
    #[allow(dead_code)]
    Cancelled {
        reason: CancellationReason,
    },
    #[allow(dead_code)]
    Blocked {
        blocked_action: String,
    },
}

/// Derive an [`AmbientConversationStatus`] from the given conversation, if it has
/// reached a terminal state that we care about for ambient agents.
pub fn conversation_output_status_from_conversation(
    conversation: &AIConversation,
) -> Option<AmbientConversationStatus> {
    match conversation.status() {
        // A pending recovery is not a terminal outcome.
        ConversationStatus::TransientError => None,

        ConversationStatus::Blocked { blocked_action } => {
            Some(AmbientConversationStatus::Blocked {
                blocked_action: blocked_action.clone(),
            })
        }

        ConversationStatus::Error => {
            // Prefer the structured error on the last exchange: it carries the precise
            // error variant and rendering hints. Fall back to the conversation-level
            // `status_error` for out-of-band failures (e.g. shell exit) recorded
            // without an exchange. Both drive FAILED-vs-ERROR classification downstream.
            if let Some(AIAgentOutputStatus::Finished {
                finished_output: FinishedAIAgentOutput::Error { error, .. },
            }) = conversation
                .root_task_exchanges()
                .last()
                .map(|exchange| &exchange.output_status)
            {
                return Some(AmbientConversationStatus::Error {
                    error: error.clone(),
                });
            }
            if let Some(error) = conversation.status_error() {
                return Some(AmbientConversationStatus::Error {
                    error: error.clone(),
                });
            }
            // No structured error anywhere; fall back to whatever terminal outcome
            // the last exchange carries.
            terminal_status_from_last_exchange(conversation)
        }

        // `InProgress` and `WaitingForEvents` are not terminal, but we preserve the
        // existing behavior of reporting a terminal outcome whenever the last exchange
        // has already finished.
        ConversationStatus::InProgress
        | ConversationStatus::Success
        | ConversationStatus::Cancelled
        | ConversationStatus::WaitingForEvents => terminal_status_from_last_exchange(conversation),
    }
}

/// Derive a terminal [`AmbientConversationStatus`] from the conversation's last
/// exchange, if that exchange has finished.
fn terminal_status_from_last_exchange(
    conversation: &AIConversation,
) -> Option<AmbientConversationStatus> {
    let AIAgentOutputStatus::Finished { finished_output } =
        &conversation.root_task_exchanges().last()?.output_status
    else {
        return None;
    };
    Some(match finished_output {
        FinishedAIAgentOutput::Cancelled { reason, .. } => {
            AmbientConversationStatus::Cancelled { reason: *reason }
        }
        FinishedAIAgentOutput::Error { error, .. } => AmbientConversationStatus::Error {
            error: error.clone(),
        },
        FinishedAIAgentOutput::Success { .. } => AmbientConversationStatus::Success,
    })
}
