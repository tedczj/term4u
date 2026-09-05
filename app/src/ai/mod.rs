//! Local workspace metadata, LSP discovery, and skill loading.

pub(crate) mod persisted_workspace;
pub(crate) mod skills;

pub(crate) use ai::paths;

pub fn init(_app: &mut warpui::AppContext) {}
