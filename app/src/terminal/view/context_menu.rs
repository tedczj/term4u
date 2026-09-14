//! Local context menus for the terminal transcript and the alt screen.

use pathfinder_geometry::vector::Vector2F;
use warp_core::context_flag::ContextFlag;
use warpui::{AppContext, ViewContext};

use super::{ContextMenuState, TerminalAction, TerminalView};
use crate::menu::{MenuItem, MenuItemFields};
use crate::pane_group::SplitPaneState;
use crate::terminal::available_shells::AvailableShell;
use crate::terminal::model::terminal_model::BlockIndex;
use crate::util::bindings::{
    CustomAction, custom_tag_to_keystroke, keybinding_name_to_display_string,
};

/// Width of the terminal context menu, matching the pre-rewrite value.
pub(super) const CONTEXT_MENU_WIDTH: f32 = 280.;

impl TerminalView {
    /// Items for a right-click inside the block transcript. `block_index` is the block under
    /// the cursor, or `None` when the click landed on empty space below the last block.
    pub(super) fn block_context_menu_items(
        &self,
        block_index: Option<BlockIndex>,
        ctx: &mut ViewContext<Self>,
    ) -> Vec<MenuItem<TerminalAction>> {
        let has_selection = self.selected_text(ctx).is_some_and(|text| !text.is_empty());

        let mut items = vec![
            MenuItemFields::new("Copy")
                .with_on_select_action(TerminalAction::Copy)
                .with_key_shortcut_label(keybinding_name_to_display_string("terminal:copy", ctx))
                .with_disabled(!has_selection)
                .into_item(),
            MenuItemFields::new("Paste")
                .with_on_select_action(TerminalAction::Paste)
                .with_key_shortcut_label(keybinding_name_to_display_string("terminal:paste", ctx))
                .into_item(),
        ];

        if has_selection {
            items.push(
                MenuItemFields::new("Insert into input")
                    .with_on_select_action(TerminalAction::InsertSelectedTextIntoInput)
                    .into_item(),
            );
        }

        if let Some(block_index) = block_index {
            let model = self.model.lock();
            if let Some(block) = model.block_list().block_at(block_index) {
                let command_is_empty = block.command_to_string().trim().is_empty();
                let output_is_empty = block.output_to_string().trim().is_empty();
                drop(model);

                items.push(MenuItem::Separator);
                items.push(
                    MenuItemFields::new("Copy command")
                        .with_on_select_action(TerminalAction::CopyBlockCommand(block_index))
                        .with_disabled(command_is_empty)
                        .into_item(),
                );
                items.push(
                    MenuItemFields::new("Copy output")
                        .with_on_select_action(TerminalAction::CopyBlockOutput(block_index))
                        .with_disabled(output_is_empty)
                        .into_item(),
                );
                items.push(
                    MenuItemFields::new("Copy block")
                        .with_on_select_action(TerminalAction::CopyBlock(block_index))
                        .with_disabled(command_is_empty && output_is_empty)
                        .into_item(),
                );
            }
        }

        items.push(MenuItem::Separator);
        items.push(
            MenuItemFields::new("Find in terminal")
                .with_on_select_action(TerminalAction::ShowFindBar)
                .with_key_shortcut_label(keybinding_name_to_display_string("terminal:find", ctx))
                .into_item(),
        );
        items.push(
            MenuItemFields::new("Clear buffer")
                .with_on_select_action(TerminalAction::ClearBuffer)
                .into_item(),
        );

        let shell = self.model.lock().shell_launch_state().available_shell();
        let mut pane_items = self.pane_context_menu_items(shell, ctx);
        if !pane_items.is_empty() {
            items.push(MenuItem::Separator);
            items.append(&mut pane_items);
        }

        items
    }

    /// Items for a right-click while a full-screen (alt screen) application is running.
    /// The transcript is not addressable here, so only selection and pane actions apply.
    pub(super) fn alt_screen_context_menu_items(
        &self,
        ctx: &mut ViewContext<Self>,
    ) -> Vec<MenuItem<TerminalAction>> {
        let mut items = Vec::new();

        if self.selected_text(ctx).is_some_and(|text| !text.is_empty()) {
            items.push(
                MenuItemFields::new("Copy")
                    .with_on_select_action(TerminalAction::Copy)
                    .with_key_shortcut_label(keybinding_name_to_display_string(
                        "terminal:copy",
                        ctx,
                    ))
                    .into_item(),
            );
        }
        items.push(
            MenuItemFields::new("Paste")
                .with_on_select_action(TerminalAction::Paste)
                .with_key_shortcut_label(keybinding_name_to_display_string("terminal:paste", ctx))
                .into_item(),
        );

        let shell = self.model.lock().shell_launch_state().available_shell();
        let mut pane_items = self.pane_context_menu_items(shell, ctx);
        if !pane_items.is_empty() {
            items.push(MenuItem::Separator);
            items.append(&mut pane_items);
        }

        items
    }

    /// Split/maximize/close entries, shared by both menus. Restored from the pre-rewrite
    /// implementation; every entry here is local.
    pub(super) fn pane_context_menu_items(
        &self,
        shell: Option<AvailableShell>,
        ctx: &mut ViewContext<Self>,
    ) -> Vec<MenuItem<TerminalAction>> {
        let mut items = vec![];

        if ContextFlag::CreateNewSession.is_enabled() {
            items.extend([
                MenuItemFields::new("Split pane right")
                    .with_on_select_action(TerminalAction::SplitRight(shell.clone()))
                    .with_key_shortcut_label(keybinding_name_to_display_string(
                        "pane_group:add_right",
                        ctx,
                    ))
                    .into_item(),
                MenuItemFields::new("Split pane left")
                    .with_on_select_action(TerminalAction::SplitLeft(shell.clone()))
                    .with_key_shortcut_label(keybinding_name_to_display_string(
                        "pane_group:add_left",
                        ctx,
                    ))
                    .into_item(),
                MenuItemFields::new("Split pane down")
                    .with_on_select_action(TerminalAction::SplitDown(shell.clone()))
                    .with_key_shortcut_label(keybinding_name_to_display_string(
                        "pane_group:add_down",
                        ctx,
                    ))
                    .into_item(),
                MenuItemFields::new("Split pane up")
                    .with_on_select_action(TerminalAction::SplitUp(shell))
                    .with_key_shortcut_label(keybinding_name_to_display_string(
                        "pane_group:add_up",
                        ctx,
                    ))
                    .into_item(),
            ]);
        }

        let pane_state = self.split_pane_state(ctx);
        if pane_state.is_in_split_pane() {
            items.push(
                MenuItemFields::toggle_pane_action(pane_state.is_maximized())
                    .with_on_select_action(TerminalAction::ToggleMaximizePane)
                    .with_key_shortcut_label(keybinding_name_to_display_string(
                        "pane_group:toggle_maximize_pane",
                        ctx,
                    ))
                    .into_item(),
            );
            items.push(
                MenuItemFields::new("Close pane")
                    .with_on_select_action(TerminalAction::Close)
                    .with_key_shortcut_label(
                        custom_tag_to_keystroke(CustomAction::CloseCurrentSession.into())
                            .map(|keystroke| keystroke.displayed()),
                    )
                    .into_item(),
            );
        }

        items
    }

    pub(super) fn split_pane_state(&self, app: &AppContext) -> SplitPaneState {
        self.focus_handle
            .as_ref()
            .map_or(SplitPaneState::NotInSplitPane, |handle| {
                handle.split_pane_state(app)
            })
    }

    pub(super) fn show_context_menu(
        &mut self,
        position: Vector2F,
        items: Vec<MenuItem<TerminalAction>>,
        ctx: &mut ViewContext<Self>,
    ) {
        if items.is_empty() {
            return;
        }
        self.context_menu_state = Some(ContextMenuState { position });
        self.context_menu.update(ctx, move |menu, ctx| {
            menu.set_width(CONTEXT_MENU_WIDTH);
            menu.set_items(items, ctx);
            ctx.notify();
        });
        ctx.focus(&self.context_menu);
        ctx.notify();
    }

    pub(super) fn close_context_menu(&mut self, ctx: &mut ViewContext<Self>) {
        if self.context_menu_state.take().is_some() {
            // Find and other menu actions may have deliberately moved focus elsewhere.
            if self.context_menu.is_focused(ctx) {
                self.focus(ctx);
            }
            ctx.notify();
        }
    }
}
