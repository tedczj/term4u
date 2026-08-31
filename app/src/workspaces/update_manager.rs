use std::sync::Arc;
use std::sync::mpsc::SyncSender;

use anyhow::{Context, Result};
use futures::channel::oneshot::{self, Receiver};
use futures::stream::AbortHandle;
use warp_errors::{report_error, report_if_error};
use warpui::{Entity, ModelContext, RequestState, SingletonEntity};

use super::user_workspaces::{
    CreateTeamResponse, UserWorkspaces, WorkspacesMetadataResponse, WorkspacesMetadataWithPricing,
};
use super::workspace::WorkspaceUid;
use crate::ai::request_usage_model::AIRequestUsageModel;
use crate::auth::AuthStateProvider;
use crate::cloud_object::CloudObjectEventEntrypoint;
use crate::persistence::ModelEvent;
use crate::pricing::PricingInfoModel;
use crate::server::cloud_objects::update_manager::UpdateManager;
use crate::server::ids::ServerId;
use crate::server::retry_strategies::OUT_OF_BAND_REQUEST_RETRY_STRATEGY;
use crate::server::server_api::ServerApiProvider;
use crate::server::server_api::team::TeamClient;

pub enum TeamUpdateManagerEvent {
    LeaveSuccess,
    LeaveError,
    RenameTeamSuccess,
    RenameTeamError,
}

/// TeamUpdateManager is a singleton model responsible for communicating with the server and local
/// database regarding teams' metadata.
/// It emits events that are later processed by UserWorkspaces model (which is an in-memory store for
/// the workspace metadata).
/// TeamUpdateManager is used when sending a team-related request to the server and processing the
/// response, but also controls the periodic polling from the server (also controlled by calling
/// `force_refresh` method).
pub struct TeamUpdateManager {
    team_client: Arc<dyn TeamClient>,
    model_event_sender: Option<SyncSender<ModelEvent>>,
    should_poll_for_workspace_metadata_updates: bool,

    /// The abort handle for the timer that waits a fixed duration
    /// before making an outbound request for workspace metadata, if any.
    next_poll_abort_handle: Option<AbortHandle>,

    /// The abort handle for the in flight request of workspace metadata,
    /// if any.
    in_flight_request_abort_handle: Option<AbortHandle>,
}

impl TeamUpdateManager {
    #[cfg(test)]
    pub fn new(
        team_client: Arc<dyn TeamClient>,
        model_event_sender: Option<SyncSender<ModelEvent>>,
        ctx: &mut ModelContext<Self>,
    ) -> Self {
        let _ = ctx;
        Self {
            team_client,
            model_event_sender,
            should_poll_for_workspace_metadata_updates: false,
            next_poll_abort_handle: None,
            in_flight_request_abort_handle: None,
        }
    }

    #[cfg(test)]
    pub fn mock(ctx: &mut ModelContext<Self>) -> Self {
        use crate::server::server_api::team::MockTeamClient;

        // This mock API is used in test contexts where we don't care which teams the user is on.
        // Since the mocked `TeamClient` is inaccessible to tests, stub the metadata polling to
        // avoid noisy `No matching expectation found` errors.
        let mut team_client = MockTeamClient::new();
        team_client.expect_workspaces_metadata().returning(|| {
            Ok(WorkspacesMetadataWithPricing {
                metadata: WorkspacesMetadataResponse {
                    workspaces: vec![],
                    joinable_teams: vec![],
                    experiments: None,
                    ai_credit_availability: None,
                    user_purchase_policy: None,
                },
                pricing_info: None,
            })
        });

        Self::new(Arc::new(team_client), Default::default(), ctx)
    }
    pub fn stop_polling_for_workspace_metadata_updates(&mut self) {
        self.should_poll_for_workspace_metadata_updates = false;
        self.abort_existing_poll();
    }

    /// Out-of-band (from the regular poll) refresh of workspace metadata.
    /// Returns a oneshot Receiver that resolves when the refresh completes (success or final failure).
    pub fn refresh_workspace_metadata(&mut self, ctx: &mut ModelContext<Self>) -> Receiver<()> {
        // Skip the refresh when logged out to avoid noisy auth errors.
        if !AuthStateProvider::as_ref(ctx).get().is_logged_in() {
            let (tx, rx) = oneshot::channel::<()>();
            let _ = tx.send(());
            return rx;
        }

        let team_client = self.team_client.clone();
        let (tx, rx) = oneshot::channel::<()>();
        let mut tx = Some(tx);
        ctx.spawn_with_retry_on_error(
            move || {
                let team_client = team_client.clone();
                async move { team_client.workspaces_metadata().await }
            },
            OUT_OF_BAND_REQUEST_RETRY_STRATEGY,
            move |update_manager, request_state, ctx| {
                // Only signal once there are no more retries left.
                let is_final = !request_state.has_pending_retries();
                update_manager.handle_workspace_metadata_with_request_state(request_state, ctx);
                if is_final && let Some(sender) = tx.take() {
                    let _ = sender.send(());
                }
            },
        );
        rx
    }

    fn abort_existing_poll(&mut self) {
        if let Some(abort_handle) = self.in_flight_request_abort_handle.take() {
            abort_handle.abort();
        }

        if let Some(abort_handle) = self.next_poll_abort_handle.take() {
            abort_handle.abort();
        }
    }
    fn save_to_db(&self, events: impl IntoIterator<Item = ModelEvent>) {
        let model_event_sender = self.model_event_sender.clone();
        if let Some(model_event_sender) = &model_event_sender {
            for event in events {
                report_if_error!(
                    model_event_sender
                        .send(event)
                        .context("Unable to save teams metadata to sqlite")
                );
            }
        }
    }

    pub fn create_team(
        &mut self,
        team_name: String,
        entrypoint: CloudObjectEventEntrypoint,
        discoverable: Option<bool>,
        ctx: &mut ModelContext<Self>,
    ) {
        let team_client = self.team_client.clone();
        let _ = ctx.spawn(
            async move {
                team_client
                    .create_team(team_name, entrypoint, discoverable)
                    .await
                    .context("Error creating team")
            },
            Self::on_team_created,
        );
    }

    fn on_team_created(
        &mut self,
        create_team_response: Result<CreateTeamResponse>,
        ctx: &mut ModelContext<Self>,
    ) {
        // TODO we should implement a similar mechanism to cloud objects with local team id
        report_if_error!(create_team_response);
        let Ok(create_team_response) = create_team_response else {
            return;
        };

        // Update sqlite
        self.save_to_db([ModelEvent::UpsertWorkspace {
            workspace: Box::new(create_team_response.workspace.clone()),
        }]);

        // Update UserWorkspaces
        UserWorkspaces::handle(ctx).update(ctx, |user_workspaces, ctx| {
            user_workspaces.team_created(&create_team_response, ctx);
        });
    }

    pub fn leave_team(
        &mut self,
        team_uid: ServerId,
        entrypoint: CloudObjectEventEntrypoint,
        ctx: &mut ModelContext<Self>,
    ) {
        // Handle server update
        let user_uid = AuthStateProvider::as_ref(ctx).get().user_id();
        if let Some(user_uid) = user_uid {
            let team_client = self.team_client.clone();
            let _ = ctx.spawn(
                async move {
                    team_client
                        .leave_team(user_uid, team_uid, entrypoint)
                        .await
                        .context("Error leaving team")
                },
                move |me, result, ctx| {
                    me.on_team_left(team_uid, result, ctx);
                },
            );
        } else {
            log::warn!("User is not authenticated, cannot leave team");
            ctx.emit(TeamUpdateManagerEvent::LeaveError);
        }
    }

    fn on_team_left(
        &mut self,
        left_team_uid: ServerId,
        result: Result<WorkspacesMetadataWithPricing>,
        ctx: &mut ModelContext<Self>,
    ) {
        match result {
            Ok(response) => {
                if let Some(pricing_info) = response.pricing_info {
                    PricingInfoModel::handle(ctx).update(ctx, |model, ctx| {
                        model.update_pricing_info(pricing_info, ctx);
                    });
                }

                if let Some(availability) = response.metadata.ai_credit_availability {
                    AIRequestUsageModel::handle(ctx).update(ctx, |usage_model, ctx| {
                        usage_model.apply_server_availability(Ok(availability), ctx);
                    });
                }

                let workspaces = response.metadata.workspaces;
                let joinable_teams = response.metadata.joinable_teams;
                let user_purchase_policy = response.metadata.user_purchase_policy;

                UserWorkspaces::handle(ctx).update(ctx, |user_workspaces, ctx| {
                    user_workspaces.set_user_purchase_policy(user_purchase_policy);
                    user_workspaces.update_workspaces(workspaces.clone(), ctx);
                    user_workspaces.update_joinable_teams(joinable_teams, ctx);
                });

                // Check if the current workspace is still in the list of workspaces.
                // If it's not, then set the current workspace to the first workspace in the list.
                if let Some(current_workspace) = UserWorkspaces::as_ref(ctx).current_workspace() {
                    if !workspaces.iter().any(|w| w.uid == current_workspace.uid)
                        && let Some(workspace_uid) = workspaces.first().map(|w| w.uid)
                    {
                        self.set_current_workspace_uid(workspace_uid, ctx);
                    };
                } else if let Some(workspace_uid) = workspaces.first().map(|w| w.uid) {
                    self.set_current_workspace_uid(workspace_uid, ctx);
                }

                // Update sqlite
                self.save_to_db([ModelEvent::UpsertWorkspaces { workspaces }]);

                // Remove objects owned by the team that was left.
                UpdateManager::handle(ctx).update(ctx, |update_manager, ctx| {
                    // We first remove team objects from local state so that they're not shown to the user.
                    // Then, refresh all objects to fetch any that were independently shared.
                    update_manager.remove_team_objects(left_team_uid, ctx);
                    update_manager.refresh_updated_objects(ctx);
                });

                ctx.emit(TeamUpdateManagerEvent::LeaveSuccess);
            }
            Err(e) => {
                report_error!(e);

                ctx.emit(TeamUpdateManagerEvent::LeaveError);
            }
        }
    }

    pub fn rename_team(
        &mut self,
        new_name: String,
        team_uid: ServerId,
        ctx: &mut ModelContext<Self>,
    ) {
        let team_client = self.team_client.clone();
        let _ = ctx.spawn(
            async move { team_client.rename_team(new_name, team_uid).await },
            Self::on_team_renamed,
        );
    }

    fn on_team_renamed(
        &mut self,
        result: Result<WorkspacesMetadataWithPricing>,
        ctx: &mut ModelContext<Self>,
    ) {
        match result {
            Err(_) => ctx.emit(TeamUpdateManagerEvent::RenameTeamError),
            Ok(response) => {
                if let Some(pricing_info) = response.pricing_info.clone() {
                    PricingInfoModel::handle(ctx).update(ctx, |model, ctx| {
                        model.update_pricing_info(pricing_info, ctx);
                    });
                }

                self.on_workspaces_updated(Ok(response.metadata.clone()), ctx);

                // Update sqlite
                self.save_to_db([ModelEvent::UpsertWorkspaces {
                    workspaces: response.metadata.workspaces,
                }]);

                ctx.emit(TeamUpdateManagerEvent::RenameTeamSuccess);
            }
        };
        ctx.notify();
    }

    fn handle_workspace_metadata_with_request_state(
        &mut self,
        request_state: RequestState<WorkspacesMetadataWithPricing>,
        ctx: &mut ModelContext<Self>,
    ) {
        match request_state {
            RequestState::RequestSucceeded(response) => {
                if let Some(pricing_info) = response.pricing_info.clone() {
                    PricingInfoModel::handle(ctx).update(ctx, |model, ctx| {
                        model.update_pricing_info(pricing_info, ctx);
                    });
                }

                // Right now, this function is coupled with how we handle leaving a team.
                // TODO(zheng) refactor so we can separate these two cases and have clearer logic.
                self.on_workspaces_updated(Ok(response.metadata), ctx);
            }
            RequestState::RequestFailedRetryPending(err) => {
                log::info!(
                    "get_workspaces_metadata_for_user: request failed with error {err:#}. Trying again."
                );
            }
            RequestState::RequestFailed(err) => {
                log::info!(
                    "get_workspaces_metadata_for_user: request failed with error {err:#}. Retries exhausted."
                );
            }
        }
    }

    fn on_workspaces_updated(
        &mut self,
        result: Result<WorkspacesMetadataResponse>,
        ctx: &mut ModelContext<Self>,
    ) {
        match result {
            Ok(user_workspaces_access) => {
                let workspaces = user_workspaces_access.workspaces;
                let joinable_teams = user_workspaces_access.joinable_teams;
                let experiments = user_workspaces_access.experiments;
                let user_purchase_policy = user_workspaces_access.user_purchase_policy;

                if let Some(availability) = user_workspaces_access.ai_credit_availability {
                    AIRequestUsageModel::handle(ctx).update(ctx, |usage_model, ctx| {
                        usage_model.apply_server_availability(Ok(availability), ctx);
                    });
                }

                UserWorkspaces::handle(ctx).update(ctx, |user_workspaces, ctx| {
                    user_workspaces.set_user_purchase_policy(user_purchase_policy);
                    user_workspaces.update_workspaces(workspaces.clone(), ctx);
                    user_workspaces.update_joinable_teams(joinable_teams.clone(), ctx);
                });

                // Check if the current workspace is still in the list of workspaces.
                // If it's not, then set the current workspace to the first workspace in the list.
                if let Some(current_workspace) = UserWorkspaces::as_ref(ctx).current_workspace() {
                    if !workspaces.iter().any(|w| w.uid == current_workspace.uid)
                        && let Some(workspace_uid) = workspaces.first().map(|w| w.uid)
                    {
                        self.set_current_workspace_uid(workspace_uid, ctx);
                    };
                } else if let Some(workspace_uid) = workspaces.first().map(|w| w.uid) {
                    self.set_current_workspace_uid(workspace_uid, ctx);
                }

                if let Some(experiments) = experiments {
                    ServerApiProvider::handle(ctx).update(ctx, |provider, ctx| {
                        provider.handle_experiments_fetched(experiments, ctx);
                    });
                }

                // Update sqlite
                self.save_to_db([ModelEvent::UpsertWorkspaces { workspaces }]);
            }
            Err(e) => {
                report_error!(e);
            }
        }
    }

    pub fn set_current_workspace_uid(
        &mut self,
        workspace_uid: WorkspaceUid,
        ctx: &mut ModelContext<Self>,
    ) {
        UserWorkspaces::handle(ctx).update(ctx, |user_workspaces, ctx| {
            user_workspaces.set_current_workspace_uid(workspace_uid, ctx);
        });

        // Update sqlite
        self.save_to_db([ModelEvent::SetCurrentWorkspace { workspace_uid }]);
    }
}

impl Entity for TeamUpdateManager {
    type Event = TeamUpdateManagerEvent;
}

impl SingletonEntity for TeamUpdateManager {}
