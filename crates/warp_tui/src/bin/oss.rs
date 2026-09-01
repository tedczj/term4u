//! OSS-channel `warp-tui` binary and `default-run` target.
//!
//! This is what bare `cargo run -p warp_tui` builds, so it hand-builds an
//! offline config and needs no internal `warp-channel-config` generator. It is a console application (no GUI window,
//! no app bundle), so unlike the GUI binaries it sets no `windows_subsystem`
//! attribute and embeds no `Info.plist`.

#[cfg(not(feature = "offline_hard"))]
use anyhow::Result;
#[cfg(not(feature = "offline_hard"))]
use warp_core::AppId;
#[cfg(not(feature = "offline_hard"))]
use warp_core::channel::{Channel, ChannelConfig, ChannelState, ConnectivityMode};

#[cfg(not(feature = "offline_hard"))]
fn main() -> Result<()> {
    let mut state = ChannelState::new(
        Channel::Oss,
        ChannelConfig {
            app_id: AppId::new("dev", "warp", "WarpTui"),
            logfile_name: "warp-tui.log".into(),
            connectivity: ConnectivityMode::Offline {
                allow_loopback: true,
            },
            mcp_static_config: None,
        },
    );
    if cfg!(debug_assertions) {
        state = state.with_additional_features(warp_core::features::DEBUG_FLAGS);
    }
    ChannelState::set(state);

    warp_tui::run()
}

#[cfg(feature = "offline_hard")]
fn main() {
    eprintln!("warp-tui-oss is not available in offline builds; use term4u-tui");
    std::process::exit(2);
}
