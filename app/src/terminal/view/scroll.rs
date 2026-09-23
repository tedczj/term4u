use super::*;
use crate::terminal::model::block::BlockId;
use crate::terminal::model::grid::grid_handler::AbsolutePoint;

#[derive(Clone)]
pub(super) struct ScrollSnapshot {
    position: f32,
    max_position: f32,
    following: bool,
    anchor: Option<ReadingAnchor>,
}

#[derive(Clone)]
struct ReadingAnchor {
    block_id: BlockId,
    grid_type: GridType,
    point: AbsolutePoint,
    offset_rows: f64,
}

impl ScrollSnapshot {
    fn capture(view: &TerminalView, model: &TerminalModel) -> Self {
        let list = model.block_list();
        let tree = list.block_heights();
        let cell_height = view.size_info.cell_height_px().as_f32();
        let total_height = tree.summary().height.as_f64() as f32 * cell_height;
        let max_position = (total_height - view.size_info.pane_height_px).max(0.);
        let position = view
            .transcript_scroll
            .scroll_start()
            .as_f32()
            .min(max_position);
        let following = view.transcript_height == 0. || max_position - position <= 0.5;
        let row = position as f64 / cell_height as f64;
        let end_row = row + view.size_info.pane_height_px as f64 / cell_height as f64;
        let mut result = Self {
            position,
            max_position,
            following,
            anchor: None,
        };
        if following {
            return result;
        }
        let mut cursor = tree.cursor::<BlockHeight, BlockHeightSummary>();
        cursor.seek(&row.into(), SeekBias::Right);
        while let Some(item) = cursor.item() {
            let top = cursor.start().height.as_f64();
            if top >= end_row {
                break;
            }
            let index = BlockIndex(cursor.start().block_count);
            let bottom = top + item.height().as_f64();
            cursor.seek(&bottom.into(), SeekBias::Right);
            if !matches!(item, BlockHeightItem::Block(_)) {
                continue;
            }
            let block = &list.blocks()[index.0];
            for (hidden, grid_type) in [
                (block.should_hide_command_grid(), GridType::PromptAndCommand),
                (block.should_hide_output_grid(), GridType::Output),
            ] {
                let grid = block.grid_of_type(grid_type).unwrap();
                let first = BlockListPoint::from_within_block_point(
                    &WithinBlock::new(Point { row: 0, col: 0 }, index, grid_type),
                    list,
                )
                .row
                .as_f64();
                let rows = (grid.len_displayed() as f64).min((bottom - first).max(0.));
                if hidden || rows == 0. || first + rows <= row {
                    continue;
                }
                let point = Point {
                    row: (row - first).max(0.).floor() as usize,
                    col: 0,
                };
                result.anchor = Some(ReadingAnchor {
                    block_id: block.id().clone(),
                    grid_type,
                    point: AbsolutePoint::from_point(point, grid.grid_handler()),
                    offset_rows: first + point.row as f64 - row,
                });
                return result;
            }
        }
        result
    }

    pub(super) fn matches_scroll(&self, view: &TerminalView) -> bool {
        (view
            .transcript_scroll
            .scroll_start()
            .as_f32()
            .min(self.max_position)
            - self.position)
            .abs()
            <= 0.5
    }

    pub(super) fn prepare_resize(&self, model: &mut TerminalModel) {
        if let Some(anchor) = &self.anchor
            && let Some(block) = model.block_list_mut().mut_block_from_id(&anchor.block_id)
            && let Some(grid) = block.grid_of_type_mut(anchor.grid_type)
            && let Some(point) = anchor.point.to_point(grid.grid_handler())
        {
            grid.grid_handler_mut().set_resize_anchor(point);
        }
    }

    pub(super) fn finish_resize(&mut self, model: &mut TerminalModel) {
        self.anchor = self.anchor.take().and_then(|mut anchor| {
            let block = model.block_list_mut().mut_block_from_id(&anchor.block_id)?;
            let grid = block.grid_of_type_mut(anchor.grid_type)?;
            let point = grid.grid_handler_mut().take_resize_anchor()?;
            anchor.point = AbsolutePoint::from_point(point, grid.grid_handler());
            Some(anchor)
        });
    }

    fn anchor_row(&self, model: &TerminalModel) -> Option<f64> {
        let anchor = self.anchor.as_ref()?;
        let list = model.block_list();
        let index = list.block_index_for_id(&anchor.block_id)?;
        let block = list.block_at(index)?;
        if !block.is_visible()
            || (anchor.grid_type == GridType::Output && block.should_hide_output_grid())
        {
            let mut cursor = list
                .block_heights()
                .cursor::<BlockIndex, BlockHeightSummary>();
            cursor.seek(&index, SeekBias::Right);
            return Some(cursor.start().height.as_f64());
        }
        let grid = block.grid_of_type(anchor.grid_type)?;
        let mut point = anchor.point.to_point(grid.grid_handler())?;
        point.row = point.row.min(grid.len_displayed().saturating_sub(1));
        Some(
            BlockListPoint::from_within_block_point(
                &WithinBlock::new(point, index, anchor.grid_type),
                list,
            )
            .row
            .as_f64()
                - anchor.offset_rows,
        )
    }
}

impl TerminalView {
    pub(super) fn capture_scroll(&self, model: &TerminalModel) -> ScrollSnapshot {
        self.reading_position
            .borrow()
            .as_ref()
            .filter(|snapshot| snapshot.matches_scroll(self))
            .cloned()
            .unwrap_or_else(|| ScrollSnapshot::capture(self, model))
    }

    pub(super) fn remember_scroll(&self, model: &TerminalModel) {
        *self.reading_position.borrow_mut() = Some(ScrollSnapshot::capture(self, model));
    }

    pub(super) fn restore_scroll(&mut self, snapshot: ScrollSnapshot, model: &TerminalModel) {
        let cell_height = self.size_info.cell_height_px().as_f32();
        self.transcript_height =
            model.block_list().block_heights().summary().height.as_f64() as f32 * cell_height;
        let max_position = (self.transcript_height - self.size_info.pane_height_px).max(0.);
        let position = if snapshot.following {
            max_position
        } else {
            snapshot
                .anchor_row(model)
                .map(|row| row as f32 * cell_height)
                .unwrap_or(snapshot.position)
        };
        self.transcript_scroll
            .scroll_to(position.min(max_position).max(0.).into_pixels());
        self.remember_scroll(model);
    }
}
