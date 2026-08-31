use anyhow::Context as _;
use uuid::Uuid;

fn revision_file_path() -> std::path::PathBuf {
    warp_core::paths::tui_config_local_dir().join("api_keys.revision")
}

#[cfg_attr(not(feature = "tui"), allow(dead_code))]
pub fn notify_tui_api_keys_changed() -> anyhow::Result<()> {
    let path = revision_file_path();
    let parent = path
        .parent()
        .context("TUI API-key revision path has no parent directory")?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create TUI config directory {}", parent.display()))?;
    std::fs::write(&path, Uuid::new_v4().to_string())
        .with_context(|| format!("Failed to update TUI API-key revision {}", path.display()))
}
