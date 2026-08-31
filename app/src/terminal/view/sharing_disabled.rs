use warp_terminal::session_sharing_types::common::{
    ParticipantId, Role, RoleRequestId, RoleRequestResponse,
};
use warp_terminal::session_sharing_types::sharer::RoleUpdateReason;
use warpui::{AppContext, ModelHandle, ViewContext};

use super::{TerminalAction, TerminalView};
use crate::ai::agent_tasks::AmbientAgentTaskId;
use crate::auth::UserUid;
use crate::menu::MenuItem;
use crate::terminal::TerminalModel;
use crate::terminal::session_sharing::presence_manager::PresenceManager;
use crate::terminal::session_sharing::{
    SharedSessionActionSource, SharedSessionScrollbackType, SharedSessionSource,
};

pub struct DisabledSessionKind;

impl DisabledSessionKind {
    pub fn is_sharer(&self) -> bool {
        false
    }
}

impl TerminalView {
    pub fn sharer_session_kind(&self) -> Option<&DisabledSessionKind> {
        None
    }

    pub fn shared_session_presence_manager(&self) -> Option<ModelHandle<PresenceManager>> {
        None
    }

    pub(in crate::terminal::view) fn blocks_cloud_followups_for_ambient_agent_session_from_model(
        &self,
        model: &TerminalModel,
        ctx: &AppContext,
    ) -> bool {
        let _ = (model, ctx);
        false
    }

    pub(crate) fn owned_ambient_agent_task_id(
        &self,
        ctx: &AppContext,
    ) -> Option<AmbientAgentTaskId> {
        let _ = ctx;
        None
    }
    pub fn update_session_link_permissions(
        &mut self,
        role: Option<Role>,
        ctx: &mut ViewContext<Self>,
    ) {
        let _ = (role, ctx);
    }

    pub fn update_session_team_permissions(
        &mut self,
        role: Option<Role>,
        team_uid: String,
        ctx: &mut ViewContext<Self>,
    ) {
        let _ = (role, team_uid, ctx);
    }

    pub fn update_role(
        &mut self,
        participant_id: ParticipantId,
        role: Role,
        ctx: &mut ViewContext<Self>,
    ) {
        let _ = (participant_id, role, ctx);
    }

    pub fn update_role_for_user(
        &mut self,
        user_uid: UserUid,
        role: Role,
        ctx: &mut ViewContext<Self>,
    ) {
        let _ = (user_uid, role, ctx);
    }

    pub fn update_role_for_pending_user(
        &mut self,
        email: String,
        role: Role,
        ctx: &mut ViewContext<Self>,
    ) {
        let _ = (email, role, ctx);
    }

    pub fn add_guests(&mut self, emails: Vec<String>, role: Role, ctx: &mut ViewContext<Self>) {
        let _ = (emails, role, ctx);
    }

    pub fn remove_guest(&mut self, user_uid: UserUid, ctx: &mut ViewContext<Self>) {
        let _ = (user_uid, ctx);
    }

    pub fn remove_pending_guest(&mut self, email: String, ctx: &mut ViewContext<Self>) {
        let _ = (email, ctx);
    }

    pub fn attempt_to_share_session(
        &mut self,
        scrollback_type: SharedSessionScrollbackType,
        action_source: Option<SharedSessionActionSource>,
        source: SharedSessionSource,
        bypass_conversation_guard: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        let _ = (
            scrollback_type,
            action_source,
            source,
            bypass_conversation_guard,
            ctx,
        );
    }

    pub fn stop_sharing_session(
        &mut self,
        source: SharedSessionActionSource,
        ctx: &mut ViewContext<Self>,
    ) {
        let _ = (source, ctx);
    }

    pub fn open_share_session_modal(
        &mut self,
        source: SharedSessionActionSource,
        ctx: &mut ViewContext<Self>,
    ) {
        let _ = (source, ctx);
    }

    pub fn pane_header_overflow_menu_toggled(
        &mut self,
        is_open: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        let _ = (is_open, ctx);
    }

    pub fn open_shared_session_viewer_role_menu(&mut self, ctx: &mut ViewContext<Self>) {
        let _ = ctx;
    }

    pub fn make_all_shared_session_participants_readers(
        &mut self,
        reason: RoleUpdateReason,
        ctx: &mut ViewContext<Self>,
    ) {
        let _ = (reason, ctx);
    }

    pub fn request_shared_session_role(&mut self, role: Role, ctx: &mut ViewContext<Self>) {
        let _ = (role, ctx);
    }

    pub fn cancel_shared_session_role_request(
        &mut self,
        role_request_id: RoleRequestId,
        ctx: &mut ViewContext<Self>,
    ) {
        let _ = (role_request_id, ctx);
    }

    pub fn respond_to_shared_session_role_request(
        &mut self,
        participant_id: ParticipantId,
        role_request_id: RoleRequestId,
        response: RoleRequestResponse,
        ctx: &mut ViewContext<Self>,
    ) {
        let _ = (participant_id, role_request_id, response, ctx);
    }

    pub fn copy_shared_session_link(
        &mut self,
        source: SharedSessionActionSource,
        ctx: &mut ViewContext<Self>,
    ) {
        let _ = (source, ctx);
    }

    pub fn open_shared_session_qr_code(&mut self, ctx: &mut ViewContext<Self>) {
        let _ = ctx;
    }

    pub fn open_shared_session_on_desktop(
        &mut self,
        source: SharedSessionActionSource,
        ctx: &mut ViewContext<Self>,
    ) {
        let _ = (source, ctx);
    }

    pub(crate) fn insert_conversation_ended_tombstone_with_cta(
        &mut self,
        cta: Option<()>,
        ctx: &mut ViewContext<Self>,
    ) {
        let _ = (cta, ctx);
    }

    pub(crate) fn insert_conversation_ended_tombstone_with_resolved_cta(
        &mut self,
        ctx: &mut ViewContext<Self>,
    ) {
        let _ = ctx;
    }

    pub(in crate::terminal::view) fn remove_conversation_ended_tombstone(
        &mut self,
        ctx: &mut ViewContext<Self>,
    ) {
        let _ = ctx;
    }

    pub(in crate::terminal::view) fn force_report_viewer_terminal_size(
        &mut self,
        ctx: &mut ViewContext<Self>,
    ) {
        let _ = ctx;
    }

    pub fn session_sharing_context_menu_items(
        &self,
        model: &TerminalModel,
        is_share_session_disabled: bool,
        has_session_link: bool,
    ) -> Vec<MenuItem<TerminalAction>> {
        let _ = (model, is_share_session_disabled, has_session_link);
        Vec::new()
    }
}
