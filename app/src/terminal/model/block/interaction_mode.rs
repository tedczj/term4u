use warp_terminal::model::Point;
use warp_terminal::model::grid::Dimensions;

use crate::terminal::model::RespectObfuscatedSecrets;
use crate::terminal::model::grid::RespectDisplayedOutput;
use crate::terminal::model::grid::grid_handler::GridHandler;

#[derive(Debug, Clone, Copy, Default)]
pub enum InteractionMode {
    #[default]
    User,
}

impl InteractionMode {
    pub fn should_hide_block(&self) -> bool {
        false
    }
}

pub const CURSOR_MARKER: &str = "<|cursor|>";

pub fn formatted_terminal_contents_for_input(
    grid_handler: &GridHandler,
    max_row_count: Option<usize>,
    cursor_pattern: &'static str,
) -> String {
    let cursor_point = grid_handler.cursor_point();
    let max_column_index = grid_handler.columns().saturating_sub(1);
    let (context_start_point, context_end_point) = match max_row_count {
        Some(max_count) => {
            let end_point = Point::new(grid_handler.max_content_row(), max_column_index)
                .min(Point::new(cursor_point.row + max_count / 2, max_column_index));
            (
                Point::new(end_point.row.saturating_sub(max_count), 0),
                end_point,
            )
        }
        None => (
            Point::new(0, 0),
            Point::new(
                grid_handler.total_rows().saturating_sub(1),
                grid_handler.columns().saturating_sub(1),
            ),
        ),
    };

    format!(
        "{}{}{cursor_pattern}{}",
        grid_handler.bounds_to_string(
            context_start_point,
            if cursor_point.col == 0 {
                Point::new(
                    cursor_point.row.saturating_sub(1),
                    grid_handler.columns().saturating_sub(1),
                )
            } else {
                Point::new(cursor_point.row, cursor_point.col.saturating_sub(1))
            },
            false,
            RespectObfuscatedSecrets::Yes,
            true,
            RespectDisplayedOutput::No,
        ),
        if cursor_point.col == 0 { "\n" } else { "" },
        grid_handler.bounds_to_string(
            cursor_point,
            context_end_point,
            false,
            RespectObfuscatedSecrets::Yes,
            true,
            RespectDisplayedOutput::No,
        )
    )
}
