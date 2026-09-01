use std::borrow::Cow;
use std::collections::HashSet;

use lazy_static::lazy_static;
use parking_lot::Mutex;
#[cfg(not(feature = "offline_hard"))]
use url::ParseError;
use url::{Origin, Url};

use super::Channel;
use crate::AppId;
use crate::channel::config::{ChannelConfig, ConnectivityMode, IapConfig, McpOAuthProviderConfig};
#[cfg(not(feature = "offline_hard"))]
use crate::channel::config::{OzConfig, WarpServerConfig};
use crate::features::FeatureFlag;
use crate::product_identity::{self, GUI_LOG_FILE, URL_SCHEME};

lazy_static! {
    static ref CHANNEL_STATE: Mutex<ChannelState> = Mutex::new(ChannelState::init());
}

#[cfg(feature = "test-util")]
lazy_static! {
    static ref MOCK_SERVER: Mutex<mockito::ServerGuard> = Mutex::new(mockito::Server::new());
    static ref MOCK_SERVER_URL: String = MOCK_SERVER.lock().url();
    static ref APP_VERSION: Mutex<Option<&'static str>> = Mutex::new(None);
}

#[derive(Debug, thiserror::Error)]
#[error("term4u is built offline-only; there is no {0}")]
pub struct OfflineError(&'static str);

impl OfflineError {
    pub const fn new(capability: &'static str) -> Self {
        Self(capability)
    }
}

#[derive(Debug)]
pub struct ChannelState {
    channel: Channel,

    /// The set of additional features to enable (on top of default-enabled ones).
    additional_features: HashSet<FeatureFlag>,

    config: ChannelConfig,
}

impl ChannelState {
    pub fn init() -> Self {
        let channel = Channel::Oss;
        let app_id = product_identity::app_id();
        cfg_if::cfg_if! {
            if #[cfg(feature = "offline_hard")] {
                let connectivity = ConnectivityMode::Offline { allow_loopback: true };
            } else {
                let connectivity = ConnectivityMode::Cloud {
                    server: WarpServerConfig::production(),
                    oz: OzConfig::production(),
                };
            }
        }
        Self {
            channel,
            additional_features: Default::default(),
            config: ChannelConfig {
                app_id,
                logfile_name: GUI_LOG_FILE.into(),
                connectivity,
                mcp_static_config: None,
            },
        }
    }

    /// Returns the server used by test-only URL routing so downstream tests can install mocks.
    #[cfg(feature = "test-util")]
    pub fn mock_server() -> parking_lot::MutexGuard<'static, mockito::ServerGuard> {
        lazy_static::initialize(&MOCK_SERVER_URL);
        MOCK_SERVER.lock()
    }

    pub fn new(channel: Channel, config: ChannelConfig) -> Self {
        Self {
            channel,
            additional_features: Default::default(),
            config,
        }
    }

    pub fn with_additional_features(mut self, overrides: &[FeatureFlag]) -> Self {
        self.additional_features.extend(overrides);
        self
    }

    pub fn set(state: ChannelState) {
        *CHANNEL_STATE.lock() = state;
    }

    pub fn is_release_bundle() -> bool {
        cfg!(feature = "release_bundle")
    }

    pub fn enable_debug_features() -> bool {
        cfg!(debug_assertions) || matches!(Self::channel(), Channel::Local | Channel::Dev)
    }

    #[cfg(not(feature = "offline_hard"))]
    pub fn override_server_root_url(url: impl Into<Cow<'static, str>>) -> Result<(), ParseError> {
        let url = url.into();
        Url::parse(&url)?;
        let mut state = CHANNEL_STATE.lock();
        let ConnectivityMode::Cloud { server, .. } = &mut state.config.connectivity else {
            return Ok(());
        };
        server.server_root_url = url;
        Ok(())
    }

    #[cfg(not(feature = "offline_hard"))]
    pub fn override_ws_server_url(url: impl Into<Cow<'static, str>>) -> Result<(), ParseError> {
        let url = url.into();
        Url::parse(&url)?;
        let mut state = CHANNEL_STATE.lock();
        let ConnectivityMode::Cloud { server, .. } = &mut state.config.connectivity else {
            return Ok(());
        };
        server.rtc_server_url = url;
        Ok(())
    }

    #[cfg(not(feature = "offline_hard"))]
    pub fn override_session_sharing_server_url(
        url: impl Into<Cow<'static, str>>,
    ) -> Result<(), ParseError> {
        let url = url.into();
        Url::parse(&url)?;
        let mut state = CHANNEL_STATE.lock();
        let ConnectivityMode::Cloud { server, .. } = &mut state.config.connectivity else {
            return Ok(());
        };
        server.session_sharing_server_url = Some(url);
        Ok(())
    }

    pub fn uses_staging_server() -> bool {
        Self::server_root_url()
            .ok()
            .and_then(|url| Url::parse(&url).ok())
            .is_some_and(|url| url.host_str() == Some("staging.warp.dev"))
    }

    /// Returns the canonical identifier for the application.
    ///
    /// This should not be used for namespacing persisted data - such use cases
    /// should make use of [`Self::data_domain`] instead.
    pub fn app_id() -> AppId {
        CHANNEL_STATE.lock().config.app_id.clone()
    }

    /// Returns a profile name for isolating user data. This should be used to
    /// sandbox how user data is stored.
    ///
    /// This is a debugging tool for isolating development instances of Warp, and is not
    /// supported in release builds.
    pub fn data_profile() -> Option<String> {
        if cfg!(debug_assertions) {
            std::env::var("WARP_DATA_PROFILE").ok()
        } else {
            None
        }
    }

    /// Returns a value that should be used for namespacing persisted data.
    ///
    /// In release builds, this is identical to the app ID; in debug builds,
    /// it optionally includes a suffix derived from the `WARP_DATA_PROFILE`
    /// environment variable.
    pub fn data_domain() -> String {
        match Self::data_profile() {
            Some(profile) => format!("{}-{profile}", Self::app_id()),
            None => Self::app_id().to_string(),
        }
    }

    /// Returns the data domain if overridden from the default, otherwise None.
    pub fn data_domain_if_not_default() -> Option<String> {
        Self::data_profile().map(|_| Self::data_domain())
    }

    pub fn additional_features() -> HashSet<FeatureFlag> {
        CHANNEL_STATE
            .lock()
            .additional_features
            .iter()
            .cloned()
            .collect()
    }

    pub fn debug_str() -> String {
        format!("{:?}", *CHANNEL_STATE.lock())
    }

    pub fn logfile_name() -> Cow<'static, str> {
        CHANNEL_STATE.lock().config.logfile_name.clone()
    }

    pub const fn is_telemetry_available() -> bool {
        false
    }

    pub const fn is_crash_reporting_available() -> bool {
        false
    }

    pub const fn show_autoupdate_menu_items() -> bool {
        false
    }

    pub fn firebase_api_key() -> Result<Cow<'static, str>, OfflineError> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "offline_hard")] {
                Err(OfflineError("Firebase API key"))
            } else {
                let state = CHANNEL_STATE.lock();
                match &state.config.connectivity {
                    ConnectivityMode::Offline { .. } => Err(OfflineError("Firebase API key")),
                    ConnectivityMode::Cloud { server, .. } => Ok(server.firebase_auth_api_key.clone()),
                }
            }
        }
    }

    pub fn iap_config() -> Result<Option<IapConfig>, OfflineError> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "offline_hard")] {
                Err(OfflineError("IAP configuration"))
            } else {
                let state = CHANNEL_STATE.lock();
                match &state.config.connectivity {
                    ConnectivityMode::Offline { .. } => Err(OfflineError("IAP configuration")),
                    ConnectivityMode::Cloud { server, .. } => Ok(server.iap_config.clone()),
                }
            }
        }
    }

    pub fn ws_server_url() -> Result<Cow<'static, str>, OfflineError> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "offline_hard")] {
                Err(OfflineError("WebSocket server URL"))
            } else {
                let state = CHANNEL_STATE.lock();
                match &state.config.connectivity {
                    ConnectivityMode::Offline { .. } => Err(OfflineError("WebSocket server URL")),
                    ConnectivityMode::Cloud { server, .. } => Ok(server.rtc_server_url.clone()),
                }
            }
        }
    }

    /// Returns the HTTP(S) root URL for the RTC server. Used for HTTP endpoints
    /// served by warp-server-rtc (e.g. the agent event SSE stream).
    pub fn rtc_http_url() -> Result<Cow<'static, str>, OfflineError> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "test-util")] {
                Ok(Cow::Owned(MOCK_SERVER_URL.clone()))
            } else if #[cfg(feature = "offline_hard")] {
                Err(OfflineError("RTC HTTP URL"))
            } else {
                let ws_url = Self::ws_server_url()?;
                match derive_http_origin_from_ws_url(&ws_url) {
                    Some(origin) => Ok(Cow::Owned(origin)),
                    None => Self::server_root_url(),
                }
            }
        }
    }

    pub fn session_sharing_server_url() -> Result<Option<Cow<'static, str>>, OfflineError> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "test-util")] {
                Ok(Some(Cow::Borrowed("fake_session_sharing_url")))
            } else if #[cfg(feature = "offline_hard")] {
                Err(OfflineError("session-sharing server URL"))
            } else {
                let state = CHANNEL_STATE.lock();
                match &state.config.connectivity {
                    ConnectivityMode::Offline { .. } => Err(OfflineError("session-sharing server URL")),
                    ConnectivityMode::Cloud { server, .. } => Ok(server.session_sharing_server_url.clone()),
                }
            }
        }
    }

    pub fn oz_root_url() -> Result<Cow<'static, str>, OfflineError> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "offline_hard")] {
                Err(OfflineError("Oz root URL"))
            } else {
                let state = CHANNEL_STATE.lock();
                match &state.config.connectivity {
                    ConnectivityMode::Offline { .. } => Err(OfflineError("Oz root URL")),
                    ConnectivityMode::Cloud { oz, .. } => Ok(oz.oz_root_url.clone()),
                }
            }
        }
    }

    pub fn is_offline() -> bool {
        cfg_if::cfg_if! {
            if #[cfg(feature = "offline_hard")] {
                true
            } else {
                matches!(
                    &CHANNEL_STATE.lock().config.connectivity,
                    ConnectivityMode::Offline { .. }
                )
            }
        }
    }

    pub fn server_root_url() -> Result<Cow<'static, str>, OfflineError> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "test-util")] {
                Ok(Cow::Owned(MOCK_SERVER_URL.clone()))
            } else if #[cfg(feature = "offline_hard")] {
                Err(OfflineError("server root URL"))
            } else {
                let state = CHANNEL_STATE.lock();
                match &state.config.connectivity {
                    ConnectivityMode::Offline { .. } => Err(OfflineError("server root URL")),
                    ConnectivityMode::Cloud { server, .. } => Ok(server.server_root_url.clone()),
                }
            }
        }
    }

    pub fn workload_audience_url() -> Result<Cow<'static, str>, OfflineError> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "offline_hard")] {
                Err(OfflineError("workload audience URL"))
            } else {
                let state = CHANNEL_STATE.lock();
                match &state.config.connectivity {
                    ConnectivityMode::Offline { .. } => Err(OfflineError("workload audience URL")),
                    ConnectivityMode::Cloud { server, oz } => Ok(oz
                        .workload_audience_url
                        .clone()
                        .unwrap_or_else(|| server.server_root_url.clone())),
                }
            }
        }
    }

    pub fn server_root_domain() -> Result<Origin, OfflineError> {
        let url = Self::server_root_url()?;
        Url::parse(&url)
            .map(|url| url.origin())
            .map_err(|_| OfflineError("valid server root domain"))
    }

    pub fn channel() -> Channel {
        CHANNEL_STATE.lock().channel
    }

    #[cfg(feature = "test-util")]
    pub fn app_version() -> Option<&'static str> {
        let version = APP_VERSION.lock();

        version.or_else(|| option_env!("GIT_RELEASE_TAG"))
    }

    #[cfg(feature = "test-util")]
    pub fn set_app_version(version: Option<&'static str>) {
        *APP_VERSION.lock() = version;
    }

    #[cfg(not(feature = "test-util"))]
    pub fn app_version() -> Option<&'static str> {
        option_env!("GIT_RELEASE_TAG")
    }

    /// Returns the MCP OAuth provider config matching the given client ID, if any.
    pub fn mcp_oauth_provider_by_client_id(client_id: &str) -> Option<McpOAuthProviderConfig> {
        CHANNEL_STATE
            .lock()
            .config
            .mcp_static_config
            .as_ref()
            .and_then(|c| c.providers.iter().find(|p| p.client_id == client_id))
            .cloned()
    }

    /// Returns the MCP OAuth provider config matching the given issuer URL, if any.
    pub fn mcp_oauth_provider_by_issuer(issuer: &str) -> Option<McpOAuthProviderConfig> {
        CHANNEL_STATE
            .lock()
            .config
            .mcp_static_config
            .as_ref()
            .and_then(|c| c.providers.iter().find(|p| p.issuer == issuer))
            .cloned()
    }

    pub fn url_scheme() -> &'static str {
        match Self::channel() {
            Channel::Stable => "warp",
            Channel::Preview => "warppreview",
            Channel::Dev => "warpdev",
            // Dummy value--integration tests shouldn't support URL schemes.
            Channel::Integration => "warpintegration",
            Channel::Local => "warplocal",
            Channel::Oss => URL_SCHEME,
        }
    }
}

/// Derives an HTTP(S) origin URL from a WebSocket URL by rewriting the scheme
/// (`wss`→`https`, `ws`→`http`) and stripping the path, query, and fragment.
/// Returns [`None`] when the input cannot be parsed as a URL or uses a scheme
/// other than `ws` or `wss`.
#[cfg(all(not(feature = "test-util"), not(feature = "offline_hard")))]
fn derive_http_origin_from_ws_url(ws_url: &str) -> Option<String> {
    let url = Url::parse(ws_url).ok()?;
    let http_scheme = match url.scheme() {
        "wss" => "https",
        "ws" => "http",
        _ => return None,
    };
    let host = url.host_str()?;
    let mut origin = format!("{http_scheme}://{host}");
    if let Some(port) = url.port() {
        origin.push_str(&format!(":{port}"));
    }
    Some(origin)
}

#[cfg(all(test, not(feature = "test-util")))]
#[path = "state_tests.rs"]
mod tests;
