use std::sync::Arc;

use parking_lot::FairMutex;
use warp::tui_export::{BlockPadding, BlockSpacing, TerminalModel};
use warp_core::semantic_selection::SemanticSelection;
use warpui::SingletonEntity;
use warpui_core::elements::tui::{
    TuiElement, TuiScrollable, TuiScrollableElement as _, TuiSelectable, TuiSelectionHandle,
    TuiViewportVerticalAlignment, TuiViewportedList, TuiViewportedListState,
};
use warpui_core::{AppContext, Entity, TuiView};

use crate::terminal_block::{block_content_rows, should_render_terminal_block};
use crate::tui_block_list_viewport_source::TuiBlockListViewportSource;
use crate::tui_builder::TuiUiBuilder;

pub(crate) const BLOCK_TOP_PADDING_ROWS: u16 = 1;
pub(crate) const TRANSCRIPT_BLOCK_SPACING: BlockSpacing = BlockSpacing {
    block_padding: BlockPadding {
        padding_top: BLOCK_TOP_PADDING_ROWS as f32,
        command_padding_top: 0.0,
        middle: 0.0,
        bottom: 0.0,
    },
    warp_prompt_height_lines: 0.0,
    show_memory_stats: false,
};

pub(super) struct TuiTranscriptView {
    model: Arc<FairMutex<TerminalModel>>,
    viewport: TuiViewportedListState,
    selection: TuiSelectionHandle,
}

impl TuiTranscriptView {
    pub(super) fn new(model: Arc<FairMutex<TerminalModel>>) -> Self {
        Self {
            model,
            viewport: TuiViewportedListState::new_at_end(),
            selection: TuiSelectionHandle::default(),
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        let model = self.model.lock();
        let block_list = model.block_list();
        !block_list.blocks().iter().any(|block| {
            should_render_terminal_block(block, block_list) && !block_content_rows(block).is_empty()
        })
    }
}

impl Entity for TuiTranscriptView {
    type Event = ();
}

impl TuiView for TuiTranscriptView {
    fn ui_name() -> &'static str {
        "TuiTranscriptView"
    }

    fn render(&self, app: &AppContext) -> Box<dyn TuiElement> {
        let viewport = TuiViewportedList::new(
            self.viewport.clone(),
            TuiBlockListViewportSource::new(self.model.clone()),
            TuiUiBuilder::from_app(app).selection_style(),
        )
        .with_vertical_alignment(TuiViewportVerticalAlignment::GrowFromBottom);
        let semantic_selection = SemanticSelection::as_ref(app);
        let selectable = TuiSelectable::new(self.selection.clone(), viewport)
            .with_word_boundaries_policy(semantic_selection.word_boundary_policy())
            .with_smart_select_fn(semantic_selection.smart_select_fn());
        TuiScrollable::new(selectable.finish_scrollable()).finish()
    }
}
