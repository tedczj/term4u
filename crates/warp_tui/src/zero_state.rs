use warpui_core::AppContext;
use warpui_core::elements::CrossAxisAlignment;
use warpui_core::elements::tui::{Modifier, TuiElement, TuiFlex, TuiText};

use crate::tui_builder::TuiUiBuilder;

pub(crate) fn local_zero_state(ctx: &AppContext) -> Box<dyn TuiElement> {
    let builder = TuiUiBuilder::from_app(ctx);
    TuiFlex::column()
        .child(
            TuiText::new("Term4u local terminal")
                .with_style(builder.primary_text_style().add_modifier(Modifier::BOLD))
                .truncate()
                .finish(),
        )
        .child(
            TuiText::new("Type a command to begin.")
                .with_style(builder.muted_text_style())
                .truncate()
                .finish(),
        )
        .child(
            TuiText::new("ctrl-t new tab  ctrl-tab switch  ctrl-q exit")
                .with_style(builder.dim_text_style())
                .truncate()
                .finish(),
        )
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .finish()
}

#[cfg(test)]
#[path = "zero_state_tests.rs"]
mod tests;
