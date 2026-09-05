pub mod accept_autosuggestion_keybinding_view;
pub mod autosuggestion_ignore_view;
mod soft_wrap;
mod view;

use std::cmp;
use std::ops::Range;

/// Consumers of the editor should only interface with the view.
/// They should _not_ be able to interface with the internal
/// details of the editor (e.g. the [`Buffer`]).
pub use view::*;
use warp_core::semantic_selection::SemanticSelection;
use warp_editor::selection::TextUnit;
use warpui::{AppContext, SingletonEntity as _};
pub use warpui::text::point::Point;

// Re-exported for use by the `warp_tui` TUI front-end, which needs to
// construct and subscribe to `CodeEditorModel` in char-cell mode.
pub use crate::code::editor::model::{CodeEditorModel, CodeEditorModelEvent, LineBound};

pub fn init(app: &mut AppContext) {
    view::init(app);
}

pub(crate) fn word_unit(ctx: &AppContext) -> TextUnit {
    TextUnit::Word(SemanticSelection::as_ref(ctx).word_boundary_policy())
}

trait RangeExt<T> {
    fn sorted(&self) -> (T, T);
}

impl<T: Ord + Clone> RangeExt<T> for Range<T> {
    fn sorted(&self) -> (T, T) {
        (
            cmp::min(&self.start, &self.end).clone(),
            cmp::max(&self.start, &self.end).clone(),
        )
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
pub mod tests;
