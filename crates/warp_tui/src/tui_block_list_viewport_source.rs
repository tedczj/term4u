use std::ops::Range;
use std::sync::Arc;

use parking_lot::FairMutex;
use sum_tree::SeekBias;
use warp::tui_export::{BlockHeight, BlockHeightItem, BlockHeightSummary, BlockId, TerminalModel};
use warpui_core::AppContext;
use warpui_core::elements::tui::{
    TuiElement, TuiLayoutContext, TuiViewportContent, TuiViewportWindow, TuiViewportedElement,
    TuiVisibleViewportItem,
};

use crate::terminal_block::{TerminalBlockElement, should_render_terminal_block};

pub(super) struct TuiBlockListViewportSource {
    model: Arc<FairMutex<TerminalModel>>,
}

impl TuiBlockListViewportSource {
    pub(super) fn new(model: Arc<FairMutex<TerminalModel>>) -> Self {
        Self { model }
    }

    fn content(&self, window: TuiViewportWindow, available_width: u16) -> TuiViewportContent {
        let model = self.model.lock();
        let block_list = model.block_list();
        let content_height = block_list
            .block_heights()
            .summary()
            .height
            .as_f64()
            .ceil()
            .max(0.0) as usize;
        let viewport_bottom = window
            .scroll_top
            .saturating_add(usize::from(window.viewport_height));
        let mut cursor = block_list
            .block_heights()
            .cursor::<BlockHeight, BlockHeightSummary>();
        cursor.seek_clamped(&BlockHeight::from(window.scroll_top as f64), SeekBias::Left);
        let mut items = Vec::new();

        while let Some(item) = cursor.item() {
            let item_top = cursor.start().height.as_f64().floor().max(0.0) as usize;
            let height = item.height().as_f64().ceil().max(0.0) as usize;
            let item_bottom = item_top.saturating_add(height);
            if item_top >= viewport_bottom {
                break;
            }
            if item_bottom > window.scroll_top
                && let BlockHeightItem::Block(_) = item
                && let Some(block) = block_list.block_at(cursor.start().block_count.into())
                && height > 0
                && should_render_terminal_block(block, block_list)
            {
                items.push(render_terminal_item(
                    self.model.clone(),
                    block.id().clone(),
                    item_top,
                    height,
                    window,
                    available_width,
                ));
            }
            cursor.next();
        }

        TuiViewportContent {
            content_height,
            items,
        }
    }
}

impl TuiViewportedElement for TuiBlockListViewportSource {
    fn visible_items(
        &self,
        window: TuiViewportWindow,
        available_width: u16,
        _ctx: &mut TuiLayoutContext,
        _app: &AppContext,
    ) -> TuiViewportContent {
        self.content(window, available_width)
    }

    fn selection_content(
        &self,
        window: TuiViewportWindow,
        available_width: u16,
        _app: &AppContext,
    ) -> Option<TuiViewportContent> {
        Some(self.content(window, available_width))
    }
}

fn render_terminal_item(
    model: Arc<FairMutex<TerminalModel>>,
    block_id: BlockId,
    origin_y: usize,
    height: usize,
    window: TuiViewportWindow,
    width: u16,
) -> TuiVisibleViewportItem {
    let visible_rows = visible_rows(origin_y, height, window);
    TuiVisibleViewportItem {
        origin_y: origin_y.saturating_add(visible_rows.start),
        element: TerminalBlockElement::visible_rows(model, block_id, visible_rows, width).finish(),
    }
}

fn visible_rows(origin_y: usize, height: usize, window: TuiViewportWindow) -> Range<usize> {
    let bottom = origin_y.saturating_add(height);
    let visible_top = origin_y.max(window.scroll_top);
    let visible_bottom = bottom.min(
        window
            .scroll_top
            .saturating_add(usize::from(window.viewport_height)),
    );
    visible_top.saturating_sub(origin_y)..visible_bottom.saturating_sub(origin_y)
}
