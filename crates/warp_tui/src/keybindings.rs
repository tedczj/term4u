use warpui_core::AppContext;

pub(crate) const TUI_BINDING_GROUP: &str = "tui";

pub(crate) fn init(app: &mut AppContext) {
    crate::root_view::init(app);
    crate::terminal_session_view::init(app);
}

#[cfg(test)]
#[path = "keybindings_tests.rs"]
mod tests;
