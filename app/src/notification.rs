use std::sync::LazyLock;

/// At the app level, we have structs for representing the UI framework level
/// notification structs, but with the data parsed to our liking.
/// The similar structs at the UI framework layer are lower-level (mostly strings).
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use warpui::notification::NotificationResponse;
use warpui::{AppContext, EntityId, WindowId};

use crate::pane_group::PaneId;
use crate::workspace::{PaneViewLocator, Workspace, WorkspaceAction};

/// This data is passed along to the MacOS notification delegate and returned
/// to us when the notification is interacted with.
#[derive(Debug, Deserialize, Serialize)]
pub enum NotificationContext {
    /// A terminal identity scoped to the process that sent the notification.
    TerminalOrigin {
        run_id: Uuid,
        terminal_view_id: EntityId,
    },
    /// Legacy data retained for decoding, without assuming its numeric IDs are still valid.
    BlockOrigin {
        window_id: WindowId,
        pane_group_id: EntityId,
        pane_id: PaneId,
    },
}

static NOTIFICATION_RUN_ID: LazyLock<Uuid> = LazyLock::new(Uuid::new_v4);
impl NotificationContext {
    pub fn for_terminal(terminal_view_id: EntityId) -> Self {
        Self::TerminalOrigin {
            run_id: *NOTIFICATION_RUN_ID,
            terminal_view_id,
        }
    }
}

/// Resolves an explicit notification click to a live terminal in this process.
pub fn handle_notification_response(response: &NotificationResponse, ctx: &mut AppContext) {
    let Some(data) = response.data() else {
        return;
    };
    let Ok(context) = serde_json::from_str::<NotificationContext>(data) else {
        return;
    };
    let terminal_view_id = match context {
        NotificationContext::TerminalOrigin {
            run_id,
            terminal_view_id,
        } if run_id == *NOTIFICATION_RUN_ID => terminal_view_id,
        // Numeric view IDs can be reused after restarting; legacy data cannot prove its origin.
        NotificationContext::BlockOrigin { .. } | NotificationContext::TerminalOrigin { .. } => {
            return;
        }
    };
    let target = ctx.window_ids().into_iter().find_map(|window| {
        ctx.views_of_type::<Workspace>(window)?
            .into_iter()
            .find_map(|workspace| {
                workspace.read(ctx, |view, ctx| {
                    view.tab_views().find_map(|group| {
                        group
                            .read(ctx, |view, ctx| {
                                view.find_pane_id_for_terminal_view(terminal_view_id, ctx)
                            })
                            .map(|pane_id| {
                                (
                                    window,
                                    workspace.id(),
                                    PaneViewLocator {
                                        pane_group_id: group.id(),
                                        pane_id,
                                    },
                                )
                            })
                    })
                })
            })
    });
    if let Some((window, workspace, locator)) = target
        && ctx.dispatch_typed_action(
            window,
            &[workspace],
            &WorkspaceAction::FocusPane(locator),
            log::Level::Debug,
        )
    {
        ctx.windows().show_window_and_focus_app(window);
    }
}
