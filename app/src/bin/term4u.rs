use anyhow::Result;
use warp_core::channel::{Channel, ChannelConfig, ChannelState, ConnectivityMode};
use warp_core::product_identity::{self, GUI_LOG_FILE};

fn main() -> Result<()> {
    let mut state = ChannelState::new(
        Channel::Oss,
        ChannelConfig {
            app_id: product_identity::app_id(),
            logfile_name: GUI_LOG_FILE.into(),
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
    warp::run()
}

#[cfg(all(not(feature = "extern_plist"), target_os = "macos"))]
embed_plist::embed_info_plist!("../../Term4u.Info.plist");
