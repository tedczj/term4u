use ai::agent::action_result::{AIAgentActionResultType, RequestComputerUseResult};
use futures::FutureExt as _;
use futures::future::BoxFuture;
use warpui::{Entity, EntityId, ModelContext};

use super::{ActionExecution, AnyActionExecution, ExecuteActionInput, PreprocessActionInput};
use crate::ai::agent::{
    AIAgentActionType, StartRecordingResult, StopRecordingResult, UseComputerResult,
};
use crate::ai::agent_tasks::AmbientAgentTaskId;
use crate::workspaces::user_workspaces::TeamContext;

const UNSUPPORTED: &str = "Computer use and screen recording are unavailable in term4u";

pub struct UseComputerExecutor;

impl UseComputerExecutor {
    pub fn new() -> Self {
        Self
    }

    pub(super) fn should_autoexecute(
        &self,
        input: ExecuteActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> bool {
        let _ = ctx;
        matches!(input.action.action, AIAgentActionType::UseComputer(_))
    }

    pub(super) fn execute(
        &mut self,
        input: ExecuteActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> impl Into<AnyActionExecution> + use<> {
        let _ = ctx;
        if !matches!(input.action.action, AIAgentActionType::UseComputer(_)) {
            return ActionExecution::<()>::InvalidAction;
        }
        ActionExecution::Sync(AIAgentActionResultType::UseComputer(
            UseComputerResult::Error(UNSUPPORTED.to_string()),
        ))
    }

    pub(super) fn preprocess_action(
        &mut self,
        input: PreprocessActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> BoxFuture<'static, ()> {
        let _ = (input, ctx);
        futures::future::ready(()).boxed()
    }
}

impl Entity for UseComputerExecutor {
    type Event = ();
}

pub struct RequestComputerUseExecutor;

impl RequestComputerUseExecutor {
    pub fn new(terminal_view_id: EntityId) -> Self {
        let _ = terminal_view_id;
        Self
    }

    pub fn set_ambient_agent_task_id(&mut self, id: Option<AmbientAgentTaskId>) {
        let _ = id;
    }

    pub(super) fn should_autoexecute(
        &mut self,
        input: ExecuteActionInput,
        scope: &TeamContext<'_>,
        ctx: &ModelContext<Self>,
    ) -> bool {
        let _ = (scope, ctx);
        matches!(
            input.action.action,
            AIAgentActionType::RequestComputerUse(_)
        )
    }

    pub(super) fn execute(
        &mut self,
        input: ExecuteActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> impl Into<AnyActionExecution> + use<> {
        let _ = ctx;
        if !matches!(
            input.action.action,
            AIAgentActionType::RequestComputerUse(_)
        ) {
            return ActionExecution::<()>::InvalidAction;
        }
        ActionExecution::Sync(AIAgentActionResultType::RequestComputerUse(
            RequestComputerUseResult::Error(UNSUPPORTED.to_string()),
        ))
    }

    pub(super) fn preprocess_action(
        &mut self,
        input: PreprocessActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> BoxFuture<'static, ()> {
        let _ = (input, ctx);
        futures::future::ready(()).boxed()
    }
}

impl Entity for RequestComputerUseExecutor {
    type Event = ();
}

pub struct StartRecordingExecutor;

impl StartRecordingExecutor {
    pub fn new() -> Self {
        Self
    }

    pub(super) fn should_autoexecute(
        &self,
        input: ExecuteActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> bool {
        let _ = ctx;
        matches!(
            input.action.action,
            AIAgentActionType::StartRecording { .. }
        )
    }

    pub(super) fn execute(
        &mut self,
        input: ExecuteActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> impl Into<AnyActionExecution> + use<> {
        let _ = ctx;
        if !matches!(
            input.action.action,
            AIAgentActionType::StartRecording { .. }
        ) {
            return ActionExecution::<()>::InvalidAction;
        }
        ActionExecution::Sync(AIAgentActionResultType::StartRecording(
            StartRecordingResult::Error(UNSUPPORTED.to_string()),
        ))
    }

    pub(super) fn preprocess_action(
        &mut self,
        input: PreprocessActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> BoxFuture<'static, ()> {
        let _ = (input, ctx);
        futures::future::ready(()).boxed()
    }
}

impl Entity for StartRecordingExecutor {
    type Event = ();
}

pub struct StopRecordingExecutor;

impl StopRecordingExecutor {
    pub fn new() -> Self {
        Self
    }

    pub(super) fn should_autoexecute(
        &self,
        input: ExecuteActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> bool {
        let _ = ctx;
        matches!(input.action.action, AIAgentActionType::StopRecording { .. })
    }

    pub(super) fn execute(
        &mut self,
        input: ExecuteActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> AnyActionExecution {
        let _ = ctx;
        if !matches!(input.action.action, AIAgentActionType::StopRecording { .. }) {
            return ActionExecution::<()>::InvalidAction.into();
        }
        ActionExecution::<()>::Sync(AIAgentActionResultType::StopRecording(
            StopRecordingResult::Error(UNSUPPORTED.to_string()),
        ))
        .into()
    }

    pub(super) fn preprocess_action(
        &mut self,
        input: PreprocessActionInput,
        ctx: &mut ModelContext<Self>,
    ) -> BoxFuture<'static, ()> {
        let _ = (input, ctx);
        futures::future::ready(()).boxed()
    }
}

impl Entity for StopRecordingExecutor {
    type Event = ();
}
