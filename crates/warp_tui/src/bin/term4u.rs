use anyhow::Result;
use warp_core::channel::{Channel, ChannelConfig, ChannelState, ConnectivityMode};
use warp_core::product_identity::{self, TUI_LOG_FILE};

fn main() -> Result<()> {
    let mut state = ChannelState::new(
        Channel::Oss,
        ChannelConfig {
            app_id: product_identity::app_id(),
            logfile_name: TUI_LOG_FILE.into(),
            connectivity: ConnectivityMode::Offline {
                allow_loopback: true,
            },
        },
    );
    if cfg!(debug_assertions) {
        state = state.with_additional_features(warp_core::features::DEBUG_FLAGS);
    }
    ChannelState::set(state);
    warp_tui::run()
}
