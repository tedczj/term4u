use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::AppId;

#[derive(Debug, Deserialize, Serialize)]
pub enum ConnectivityMode {
    Offline {
        allow_loopback: bool,
    },
    #[cfg(not(feature = "offline_hard"))]
    Cloud {
        server: WarpServerConfig,
        oz: OzConfig,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChannelConfig {
    /// The application ID for this channel.
    pub app_id: AppId,

    /// The name of the file to which logs should be written.
    pub logfile_name: Cow<'static, str>,

    pub connectivity: ConnectivityMode,
}

/// Configuration for GCP Identity-Aware Proxy authentication, present only on staging builds.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IapConfig {
    /// The IAP OAuth2 client ID used as the audience for identity tokens.
    pub audiences: Cow<'static, str>,
    /// The service account email to impersonate when acquiring IAP credentials.
    pub service_account_email: Cow<'static, str>,
}

#[cfg(not(feature = "offline_hard"))]
#[derive(Debug, Deserialize, Serialize)]
pub struct WarpServerConfig {
    /// The root URL for the standard server pool.
    pub server_root_url: Cow<'static, str>,
    /// The URL for the RTC server, which serves real-time updates for Warp Drive objects.
    pub rtc_server_url: Cow<'static, str>,
    /// The URL for the session sharing server, or [`None`] if session sharing is not
    /// supported.
    pub session_sharing_server_url: Option<Cow<'static, str>>,
    /// The API key to use when making requests to Firebase Authentication endpoints.
    pub firebase_auth_api_key: Cow<'static, str>,
    /// Configuration for GCP Identity-Aware Proxy authentication, present only on
    /// staging builds. [`None`] on production builds.
    #[serde(default)]
    pub iap_config: Option<IapConfig>,
}

#[cfg(not(feature = "offline_hard"))]
impl WarpServerConfig {
    pub fn production() -> Self {
        Self {
            server_root_url: "https://app.warp.dev".into(),
            rtc_server_url: "wss://rtc.app.warp.dev/graphql/v2".into(),
            session_sharing_server_url: Some("wss://sessions.app.warp.dev".into()),
            firebase_auth_api_key: "AIzaSyBdy3O3S9hrdayLJxJ7mriBR4qgUaUygAs".into(),
            iap_config: None,
        }
    }
}

#[cfg(not(feature = "offline_hard"))]
#[derive(Debug, Deserialize, Serialize)]
pub struct OzConfig {
    /// Root URL for the Oz (ambient agent management) dashboard.
    pub oz_root_url: Cow<'static, str>,

    /// URL to use as the audience when issuing workload identity tokens. If [`None`], falls back
    /// to [`WarpServerConfig::server_root_url`]. This exists so the audience is not overridden
    /// when a custom server root URL is provided (e.g. an ngrok URL for local development).
    pub workload_audience_url: Option<Cow<'static, str>>,
}

#[cfg(not(feature = "offline_hard"))]
impl OzConfig {
    pub fn production() -> Self {
        Self {
            oz_root_url: "https://oz.warp.dev".into(),
            workload_audience_url: None,
        }
    }
}
