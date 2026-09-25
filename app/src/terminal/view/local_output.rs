//! Transcript-only actions. Native programs keep ownership of their control keys.

use std::sync::atomic::Ordering;

use warp_core::semantic_selection::SemanticSelection;
use warpui::text::SelectionType;
use warpui::units::IntoPixels;
use warpui::{SingletonEntity, ViewContext};

use super::TerminalView;
use crate::terminal::GridType;
use crate::terminal::model::block::BlockId;
use crate::terminal::model::blocks::BlockListPoint;
use crate::terminal::model::index::Side;
use crate::terminal::model::terminal_model::WithinBlock;
use crate::terminal::model_events::ModelEvent;

#[derive(Clone, Debug)]
pub enum LocalOutputAction {
    ClearVisible,
    PreviousBlock,
    NextBlock,
    ToggleBlockOutput(BlockId),
}

impl TerminalView {
    pub(super) fn handle_local_output_action(
        &mut self,
        action: &LocalOutputAction,
        ctx: &mut ViewContext<Self>,
    ) {
        let handle = self.model.clone();
        let mut model = handle.lock();
        // A menu can outlive a screen switch. Recheck here as well as in the keymap context.
        if model.is_alt_screen_active() {
            return;
        }
        match action {
            LocalOutputAction::ClearVisible => {
                model.block_list_mut().clear_selection();
                model.clear_visible_screen();
                drop(model);
                self.output_dragging.store(false, Ordering::Relaxed);
                self.is_selecting = false;
                self.clear_link(ctx);
                self.handle_model_event(&ModelEvent::TerminalClear, ctx);
            }
            LocalOutputAction::ToggleBlockOutput(id) => {
                let Some(block) = model.block_list().block_with_id(id) else {
                    return;
                };
                let collapsed = !block.should_hide_output_grid();
                let scroll = self.capture_scroll(&model);
                if !model
                    .block_list_mut()
                    .set_block_output_collapsed(id, collapsed)
                {
                    return;
                }
                // A selection stores display coordinates; changing heights invalidates it.
                model.block_list_mut().clear_selection();
                self.restore_scroll(scroll, &model);
                drop(model);
                self.output_dragging.store(false, Ordering::Relaxed);
                self.is_selecting = false;
                self.clear_link(ctx);
                ctx.notify();
            }
            LocalOutputAction::PreviousBlock | LocalOutputAction::NextBlock => {
                let list = model.block_list();
                let selected = list
                    .text_selection_range(SemanticSelection::as_ref(ctx), false)
                    .map(|(start, end, _)| {
                        if matches!(action, LocalOutputAction::PreviousBlock) {
                            start.block_index
                        } else {
                            end.block_index
                        }
                    });
                let target = if matches!(action, LocalOutputAction::PreviousBlock) {
                    list.prev_non_hidden_block_from_index(
                        selected.unwrap_or_else(|| list.active_block_index()),
                    )
                } else if let Some(selected) = selected {
                    list.next_non_hidden_block_from_index(selected)
                } else {
                    list.first_non_hidden_block_by_index()
                };
                let Some(index) = target else { return };
                let block = list
                    .block_at(index)
                    .expect("index came from the block list");
                let start = block.start_point().to_within_block_point(index);
                let end = if block.should_hide_output_grid() {
                    WithinBlock::new(
                        block.prompt_and_command_grid().end_point(),
                        index,
                        GridType::PromptAndCommand,
                    )
                } else {
                    block.end_point().to_within_block_point(index)
                };
                let start = BlockListPoint::from_within_block_point(&start, list);
                let end = BlockListPoint::from_within_block_point(&end, list);
                let list = model.block_list_mut();
                list.start_selection(start, SelectionType::Simple, Side::Left);
                list.update_selection(end, Side::Right);
                self.transcript_scroll.scroll_to(
                    (start.row.as_f64() as f32 * self.size_info.cell_height_px().as_f32())
                        .into_pixels(),
                );
                self.remember_scroll(&model);
                drop(model);
                self.clear_link(ctx);
                ctx.focus_self();
                ctx.notify();
            }
        }
    }
}

#[cfg(test)]
#[path = "local_output_tests.rs"]
mod tests;
