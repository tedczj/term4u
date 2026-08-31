use serde::{Deserialize, Serialize};
use warp_terminal::session_sharing_types::common::{Role, SessionId};
use warp_terminal::session_sharing_types::sharer::SessionSourceType;
use warpui::id;
use warpui::keymap::ContextPredicate;

use super::TerminalView;
use super::model::terminal_model::BlockIndex;

#[derive(Debug, Clone)]
pub struct SharedSessionSource {
    pub source_type: SessionSourceType,
    pub source_task_id: Option<String>,
}

impl SharedSessionSource {
    pub fn user(source_task_id: Option<String>) -> Self {
        Self {
            source_type: SessionSourceType::User,
            source_task_id,
        }
    }

    pub fn ambient_agent(task_id: Option<String>) -> Self {
        Self {
            source_type: SessionSourceType::AmbientAgent {
                task_id: task_id.clone(),
            },
            source_task_id: task_id,
        }
    }

    pub fn orchestrator_task_id(&self) -> Option<&str> {
        self.source_task_id.as_deref().or(match &self.source_type {
            SessionSourceType::AmbientAgent { task_id } => task_id.as_deref(),
            SessionSourceType::User => None,
        })
    }
}

impl Default for SharedSessionSource {
    fn default() -> Self {
        Self::user(None)
    }
}

#[derive(Debug, Clone, Default)]
pub enum IsSharedSessionCreator {
    Yes {
        source: SharedSessionSource,
    },
    #[default]
    No,
}

#[derive(Debug, Clone, Default)]
pub enum SharedSessionStatus {
    #[default]
    NotShared,
    ViewPending,
    ActiveViewer {
        role: Role,
    },
    FinishedViewer,
    SharePendingPreBootstrap {
        source: SharedSessionSource,
    },
    SharePending,
    ActiveSharer,
}

impl SharedSessionStatus {
    pub fn reader() -> Self {
        Self::ActiveViewer { role: Role::Reader }
    }

    pub fn executor() -> Self {
        Self::ActiveViewer {
            role: Role::Executor,
        }
    }

    pub fn is_view_pending(&self) -> bool {
        matches!(self, Self::ViewPending)
    }

    pub fn is_active_viewer(&self) -> bool {
        matches!(self, Self::ActiveViewer { .. })
    }

    pub fn is_finished_viewer(&self) -> bool {
        matches!(self, Self::FinishedViewer)
    }

    pub fn is_viewer(&self) -> bool {
        self.is_view_pending() || self.is_active_viewer() || self.is_finished_viewer()
    }

    pub fn is_executor(&self) -> bool {
        matches!(self, Self::ActiveViewer { role } if role.can_execute())
    }

    pub fn is_reader(&self) -> bool {
        matches!(self, Self::ActiveViewer { role: Role::Reader })
    }

    pub fn is_share_pending(&self) -> bool {
        matches!(
            self,
            Self::SharePending | Self::SharePendingPreBootstrap { .. }
        )
    }

    pub fn is_active_sharer(&self) -> bool {
        matches!(self, Self::ActiveSharer)
    }

    pub fn is_sharer(&self) -> bool {
        self.is_share_pending() || self.is_active_sharer()
    }

    pub fn is_sharer_or_viewer(&self) -> bool {
        !matches!(self, Self::NotShared)
    }

    pub fn as_keymap_context(&self) -> &'static str {
        match self {
            Self::NotShared => "SharedSessionStatus_NotShared",
            Self::ViewPending => "SharedSessionStatus_ViewPending",
            Self::ActiveViewer { role: Role::Reader } => "SharedSessionStatus_Reader",
            Self::ActiveViewer {
                role: Role::Executor | Role::Full,
            } => "SharedSessionStatus_Executor",
            Self::FinishedViewer => "SharedSessionStatus_FinishedViewer",
            Self::SharePendingPreBootstrap { .. } => "SharedSessionStatus_SharePendingPreBootstrap",
            Self::SharePending => "SharedSessionStatus_SharePending",
            Self::ActiveSharer => "SharedSessionStatus_ActiveSharer",
        }
    }

    pub fn active_viewer_keymap_context() -> ContextPredicate {
        id!(Self::reader().as_keymap_context()) | id!(Self::executor().as_keymap_context())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SharedSessionScrollbackType {
    None,
    FromBlock { block_index: BlockIndex },
    All,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum SharedSessionActionSource {
    BlocklistContextMenu { block_index: Option<BlockIndex> },
    Tab,
    PaneHeader,
    CommandPalette,
    OnboardingBlock,
    Closed { is_confirm_close_session: bool },
    InactivityModal,
    NonUser,
    SharingDialog,
    RightClickMenu,
    FooterChip,
}

pub fn join_native_intent(session_id: &SessionId) -> String {
    let _ = session_id;
    String::new()
}

pub fn join_link(session_id: &SessionId) -> String {
    let _ = session_id;
    String::new()
}

pub mod manager {
    use std::collections::HashMap;

    use warp_terminal::session_sharing_types::common::SessionId;
    use warpui::{
        AppContext, Entity, EntityId, ModelContext, SingletonEntity, ViewHandle, WeakViewHandle,
        WindowId,
    };

    use super::{SharedSessionStatus, TerminalView};

    struct SessionState {
        session_id: SessionId,
        view: WeakViewHandle<TerminalView>,
    }

    pub struct Manager {
        shared: HashMap<EntityId, SessionState>,
        joined: HashMap<EntityId, SessionState>,
        ended: HashMap<EntityId, SessionId>,
    }

    impl Manager {
        pub fn new(ctx: &mut ModelContext<Self>) -> Self {
            let _ = ctx;
            Self {
                shared: HashMap::new(),
                joined: HashMap::new(),
                ended: HashMap::new(),
            }
        }

        pub fn is_some_session_being_shared(&self) -> bool {
            false
        }

        pub fn is_some_session_being_viewed(&self) -> bool {
            false
        }

        pub fn session_id(&self, terminal_view_id: &EntityId) -> Option<SessionId> {
            self.shared
                .get(terminal_view_id)
                .or(self.joined.get(terminal_view_id))
                .map(|state| state.session_id)
        }

        pub fn ended_session_id(&self, terminal_view_id: &EntityId) -> Option<SessionId> {
            self.ended.get(terminal_view_id).copied()
        }

        pub fn session_id_for_link(
            &self,
            terminal_view_id: &EntityId,
            status: &SharedSessionStatus,
        ) -> Option<SessionId> {
            let _ = status;
            self.session_id(terminal_view_id)
        }

        pub fn has_session_link(
            &self,
            terminal_view_id: &EntityId,
            status: &SharedSessionStatus,
        ) -> bool {
            self.session_id_for_link(terminal_view_id, status).is_some()
        }

        pub fn shared_view_by_id(
            &self,
            terminal_view_id: &EntityId,
            ctx: &AppContext,
        ) -> Option<ViewHandle<TerminalView>> {
            self.shared.get(terminal_view_id)?.view.upgrade(ctx)
        }

        pub fn shared_view_by_session_id(
            &self,
            session_id: &SessionId,
            ctx: &AppContext,
        ) -> Option<ViewHandle<TerminalView>> {
            self.shared
                .values()
                .find(|state| state.session_id == *session_id)?
                .view
                .upgrade(ctx)
        }

        pub fn joined_view_by_id(
            &self,
            terminal_view_id: &EntityId,
            ctx: &AppContext,
        ) -> Option<ViewHandle<TerminalView>> {
            self.joined.get(terminal_view_id)?.view.upgrade(ctx)
        }

        pub fn shared_view_ids(&self) -> impl Iterator<Item = EntityId> + '_ {
            self.shared.keys().copied()
        }

        pub fn joined_view_ids(&self) -> impl Iterator<Item = EntityId> + '_ {
            self.joined.keys().copied()
        }

        pub fn shared_views<'a>(
            &'a self,
            ctx: &'a AppContext,
        ) -> impl Iterator<Item = ViewHandle<TerminalView>> + 'a {
            self.shared
                .values()
                .filter_map(move |state| state.view.upgrade(ctx))
        }

        pub fn started_share(
            &mut self,
            terminal_view: WeakViewHandle<TerminalView>,
            session_id: SessionId,
            window_id: WindowId,
            ctx: &mut ModelContext<Self>,
        ) {
            let _ = (terminal_view, session_id, window_id, ctx);
        }

        pub fn joined_share(
            &mut self,
            terminal_view: WeakViewHandle<TerminalView>,
            session_id: SessionId,
            ctx: &mut ModelContext<Self>,
        ) {
            let _ = (terminal_view, session_id, ctx);
        }

        pub fn left_share(&mut self, terminal_view_id: EntityId) {
            self.joined.remove(&terminal_view_id);
        }

        pub fn stopped_share(&mut self, terminal_view_id: EntityId, ctx: &mut ModelContext<Self>) {
            let _ = ctx;
            self.shared.remove(&terminal_view_id);
        }

        pub fn share_failed(&mut self, window_id: WindowId, ctx: &mut ModelContext<Self>) {
            let _ = (window_id, ctx);
        }

        pub fn clear_joined(&mut self) {
            self.joined.clear();
        }

        pub fn stop_all_shared_sessions(&mut self, ctx: &mut ModelContext<Self>) {
            let _ = ctx;
            self.shared.clear();
        }

        pub fn rejoin_all_shared_sessions(&mut self, ctx: &mut ModelContext<Self>) {
            let _ = ctx;
            self.joined.clear();
        }
    }

    pub enum ManagerEvent {
        ShareAttempted,
        StartedShare {
            session_id: SessionId,
            window_id: WindowId,
        },
        JoinedSession {
            session_id: SessionId,
            view_id: EntityId,
        },
        StoppedShare,
        FailedToShare {
            window_id: WindowId,
        },
    }

    impl Entity for Manager {
        type Event = ManagerEvent;
    }

    impl SingletonEntity for Manager {}
}

pub mod ai_agent {
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD_NO_PAD;
    use prost::Message as _;

    pub fn encode_agent_response_event(event: &warp_multi_agent_api::ResponseEvent) -> String {
        STANDARD_NO_PAD.encode(event.encode_to_vec())
    }
}

pub mod presence_manager {
    use std::collections::HashMap;

    use pathfinder_color::ColorU;
    use warp_terminal::session_sharing_types::common::{
        InputReplicaId, ParticipantId, ParticipantInfo, Role, RoleRequestId,
    };
    use warpui::{Entity, ModelContext};

    use crate::auth::UserUid;

    pub const MUTED_PARTICIPANT_COLOR: ColorU = ColorU {
        r: 176,
        g: 176,
        b: 176,
        a: 255,
    };

    pub fn text_selection_color(color: ColorU) -> ColorU {
        ColorU { a: 64, ..color }
    }

    #[derive(Clone)]
    pub struct Participant {
        pub info: ParticipantInfo,
        pub color: ColorU,
        pub role: Option<Role>,
    }

    impl Participant {
        pub fn id(&self) -> &ParticipantId {
            &self.info.id
        }

        pub fn input_replica_id(&self) -> &InputReplicaId {
            &self.info.profile_data.input_replica_id
        }
    }

    #[derive(Clone)]
    pub struct AbsentViewer {
        participant_info: ParticipantInfo,
    }

    impl AbsentViewer {
        pub fn id(&self) -> &ParticipantId {
            &self.participant_info.id
        }

        pub fn input_replica_id(&self) -> &InputReplicaId {
            &self.participant_info.profile_data.input_replica_id
        }
    }

    pub struct PresenceManager {
        id: ParticipantId,
        firebase_uid: UserUid,
        role: Option<Role>,
        sharer: Option<Participant>,
        participants: HashMap<ParticipantId, Participant>,
        role_requests: HashMap<ParticipantId, RoleRequestId>,
        reconnecting: bool,
    }

    impl PresenceManager {
        pub fn id(&self) -> ParticipantId {
            self.id.clone()
        }

        pub fn firebase_uid(&self) -> UserUid {
            self.firebase_uid
        }

        pub fn role(&self) -> Option<Role> {
            self.role
        }

        pub fn viewer_role(&self, id: &ParticipantId) -> Option<Role> {
            self.participants
                .get(id)
                .and_then(|participant| participant.role)
        }

        pub fn get_role_request(&self, id: &ParticipantId) -> Option<&RoleRequestId> {
            self.role_requests.get(id)
        }

        pub fn get_present_viewers(&self) -> impl Iterator<Item = &Participant> {
            self.participants.values()
        }

        pub fn get_sharer(&self) -> Option<&Participant> {
            self.sharer.as_ref()
        }

        pub fn all_present_participants(&self) -> impl Iterator<Item = &Participant> {
            self.sharer.iter().chain(self.participants.values())
        }

        pub fn get_participant(&self, id: &ParticipantId) -> Option<&Participant> {
            self.sharer
                .iter()
                .chain(self.participants.values())
                .find(|participant| participant.id() == id)
        }

        pub fn is_reconnecting(&self) -> bool {
            self.reconnecting
        }

        pub fn make_all_participants_readers(&mut self, ctx: &mut ModelContext<Self>) {
            let _ = ctx;
            for participant in self.participants.values_mut() {
                participant.role = Some(Role::Reader);
            }
        }

        pub fn single_distinct_present_viewer_uid(&self) -> Option<&str> {
            let mut uids = self
                .participants
                .values()
                .map(|participant| participant.info.profile_data.firebase_uid.as_str());
            let first = uids.next()?;
            uids.all(|uid| uid == first).then_some(first)
        }
    }

    pub enum Event {
        ParticipantListUpdated,
    }

    impl Entity for PresenceManager {
        type Event = Event;
    }
}

pub mod render_util {
    use pathfinder_color::ColorU;
    use warpui::elements::{ChildAnchor, ParentAnchor};

    use super::presence_manager::Participant;

    pub const SHARED_SESSION_AVATAR_DIAMETER: f32 = 20.;

    pub struct ParticipantAvatarParams {
        pub display_name: String,
        pub image_url: Option<String>,
        pub participant_color: ColorU,
        pub is_muted: bool,
        pub tooltip_parent_anchor: ParentAnchor,
        pub tooltip_child_anchor: ChildAnchor,
    }

    impl ParticipantAvatarParams {
        pub fn new(participant: &Participant, is_muted: bool) -> Self {
            Self {
                display_name: participant.info.profile_data.display_name.clone(),
                image_url: participant.info.profile_data.photo_url.clone(),
                participant_color: participant.color,
                is_muted,
                tooltip_parent_anchor: ParentAnchor::BottomMiddle,
                tooltip_child_anchor: ChildAnchor::TopMiddle,
            }
        }
    }
}

pub mod role_change_modal {
    use warp_terminal::session_sharing_types::common::{ParticipantId, Role, RoleRequestId};
    use warpui::elements::Empty;
    use warpui::{AppContext, Element, Entity, View, ViewContext};

    use super::render_util::ParticipantAvatarParams;
    use crate::pane_group::TerminalPaneId;

    #[derive(Debug, Clone)]
    pub enum RoleChangeOpenSource {
        ViewerRequest {
            role: Role,
        },
        SharerResponse {
            participant_id: ParticipantId,
            role_request_id: RoleRequestId,
            role: Role,
        },
        SharerGrant {
            participant_id: ParticipantId,
        },
    }

    #[derive(Debug, Clone, Copy)]
    pub enum RoleChangeCloseSource {
        ViewerRequest,
        SharerResponse,
        SharerGrant,
    }

    #[derive(Debug, Clone)]
    pub enum RoleChangeModalEvent {
        CancelRequest {
            terminal_pane_id: TerminalPaneId,
            role_request_id: RoleRequestId,
        },
        ApproveRequest {
            terminal_pane_id: TerminalPaneId,
            participant_id: ParticipantId,
            role_request_id: RoleRequestId,
            role: Role,
        },
        DenyRequest {
            terminal_pane_id: TerminalPaneId,
            participant_id: ParticipantId,
            role_request_id: RoleRequestId,
        },
        Close {
            source: RoleChangeCloseSource,
        },
        CancelGrant,
        GrantRole {
            terminal_pane_id: TerminalPaneId,
            participant_id: ParticipantId,
            dont_show_again: bool,
        },
    }

    pub struct RoleChangeModal;

    impl RoleChangeModal {
        pub fn new(ctx: &mut ViewContext<Self>) -> Self {
            let _ = ctx;
            Self
        }

        pub fn set_role_request_id(&mut self, role_request_id: RoleRequestId) {
            let _ = role_request_id;
        }

        pub fn open_for_viewer_request(
            &mut self,
            terminal_pane_id: TerminalPaneId,
            display_name: String,
            role: Role,
            ctx: &mut ViewContext<Self>,
        ) {
            let _ = (terminal_pane_id, display_name, role, ctx);
        }

        pub fn open_for_sharer_response(
            &mut self,
            request: (
                TerminalPaneId,
                ParticipantId,
                String,
                RoleRequestId,
                ParticipantAvatarParams,
                Role,
            ),
            ctx: &mut ViewContext<Self>,
        ) {
            let _ = (request, ctx);
        }

        pub fn open_for_sharer_grant(
            &mut self,
            terminal_pane_id: TerminalPaneId,
            participant_id: ParticipantId,
            ctx: &mut ViewContext<Self>,
        ) {
            let _ = (terminal_pane_id, participant_id, ctx);
        }

        pub fn all_child_modals_are_closed(&self) -> bool {
            true
        }

        pub fn close_for_viewer_request(&mut self, ctx: &mut ViewContext<Self>) {
            let _ = ctx;
        }

        pub fn close_for_sharer_response(&mut self, ctx: &mut ViewContext<Self>) {
            let _ = ctx;
        }

        pub fn close_for_sharer_grant(&mut self, ctx: &mut ViewContext<Self>) {
            let _ = ctx;
        }

        pub fn remove_role_request(
            &mut self,
            role_request_id: RoleRequestId,
            ctx: &mut ViewContext<Self>,
        ) {
            let _ = (role_request_id, ctx);
        }
    }

    impl Entity for RoleChangeModal {
        type Event = RoleChangeModalEvent;
    }

    impl View for RoleChangeModal {
        fn ui_name() -> &'static str {
            "RoleChangeModalDisabled"
        }

        fn render(&self, app: &AppContext) -> Box<dyn Element> {
            let _ = app;
            Empty::new().finish()
        }
    }
}

pub mod share_modal {
    use std::sync::Arc;

    use parking_lot::FairMutex;
    use warpui::elements::Empty;
    use warpui::{AppContext, Element, Entity, EntityId, TypedActionView, View, ViewContext};

    use super::{SharedSessionActionSource, SharedSessionScrollbackType};
    use crate::pane_group::TerminalPaneId;
    use crate::terminal::TerminalModel;

    pub struct ShareSessionModal;

    #[derive(Debug)]
    pub enum ShareSessionModalAction {
        Cancel,
    }

    pub enum ShareSessionModalEvent {
        Close,
        StartSharing {
            terminal_pane_id: TerminalPaneId,
            scrollback_type: SharedSessionScrollbackType,
            source: SharedSessionActionSource,
        },
        Upgrade,
    }

    impl ShareSessionModal {
        pub fn new(ctx: &mut ViewContext<Self>) -> Self {
            let _ = ctx;
            Self
        }

        pub fn open(
            &mut self,
            terminal_pane_id: TerminalPaneId,
            open_source: SharedSessionActionSource,
            model: Arc<FairMutex<TerminalModel>>,
            terminal_view_id: EntityId,
            ctx: &mut ViewContext<Self>,
        ) {
            let _ = (terminal_pane_id, open_source, model, terminal_view_id, ctx);
        }

        pub fn open_denied(
            &mut self,
            terminal_pane_id: TerminalPaneId,
            ctx: &mut ViewContext<Self>,
        ) {
            let _ = (terminal_pane_id, ctx);
        }
    }

    impl Entity for ShareSessionModal {
        type Event = ShareSessionModalEvent;
    }

    impl TypedActionView for ShareSessionModal {
        type Action = ShareSessionModalAction;

        fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
            let _ = (action, ctx);
        }
    }

    impl View for ShareSessionModal {
        fn ui_name() -> &'static str {
            "ShareSessionModalDisabled"
        }

        fn render(&self, app: &AppContext) -> Box<dyn Element> {
            let _ = app;
            Empty::new().finish()
        }
    }
}

impl From<&Role> for crate::editor::InteractionState {
    fn from(role: &Role) -> Self {
        match role {
            Role::Reader => Self::Selectable,
            Role::Executor | Role::Full => Self::Editable,
        }
    }
}

pub mod viewer {
    pub mod history_model {
        use warpui::Entity;

        use crate::terminal::HistoryEntry;

        #[derive(Default)]
        pub struct SharedSessionHistoryModel {
            entries: Vec<HistoryEntry>,
        }

        impl SharedSessionHistoryModel {
            pub fn new() -> Self {
                Self::default()
            }

            pub fn entries(&self) -> impl Iterator<Item = &HistoryEntry> {
                self.entries.iter()
            }

            pub fn push(&mut self, entry: HistoryEntry) {
                self.entries.push(entry);
            }
        }

        impl Entity for SharedSessionHistoryModel {
            type Event = ();
        }
    }
}
