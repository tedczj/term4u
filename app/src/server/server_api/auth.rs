use std::result::Result as StdResult;

use anyhow::Result;
use async_trait::async_trait;
use instant::Duration;
use thiserror::Error;
use warp_graphql::mutations::create_anonymous_user::{
    AnonymousUserType, CreateAnonymousUserResult,
};
use warp_graphql::mutations::expire_api_key::ExpireApiKeyResult;
use warp_graphql::mutations::generate_api_key::GenerateApiKeyResult;
use warp_graphql::mutations::mint_custom_token::MintCustomTokenResult;
use warp_graphql::mutations::update_user_settings::UpdateUserSettingsInput;
use warp_graphql::queries::api_keys::ApiKeyProperties;
use warp_graphql::queries::get_user::UserOutput as GqlUserOutput;
use warp_server_auth::credentials::{AuthToken, FirebaseToken, LoginToken};
use warp_server_client::auth::AgentIdentity;
#[cfg(any(test, feature = "test-util"))]
pub use warp_server_client::auth::MockAuthClient;
pub use warp_server_client::auth::{
    AuthClient, FetchUserResult, MintCustomTokenError, SyncedUserSettings, UserAuthenticationError,
};
use warp_server_client::ids::ApiKeyUid;

#[derive(Error, Debug)]
/// Error type when creating anonymous users.
pub enum AnonymousUserCreationError {
    #[error("The network request to create the anonymous user failed")]
    CreationFailed,

    #[error("Received a user facing error: {0}")]
    UserFacingError(String),

    /// Failure that occurs after the user is created, but the ID token could not be fetched.
    #[error("The user was created, but the ID token could not be fetched")]
    UserAuthenticationFailed(#[from] UserAuthenticationError),

    #[error("Failed to create anonymous user with unknown error")]
    Unknown,
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl AuthClient for crate::server::offline_api::OfflineApi {
    async fn create_anonymous_user(
        &self,
        referral_code: Option<String>,
        anonymous_user_type: AnonymousUserType,
    ) -> Result<CreateAnonymousUserResult> {
        let _ = (referral_code, anonymous_user_type);
        Err(Self::unavailable("authentication API"))
    }

    async fn get_or_refresh_access_token(&self) -> Result<AuthToken> {
        Err(Self::unavailable("authentication API"))
    }

    async fn fetch_user(
        &self,
        token: LoginToken,
        for_refresh: bool,
    ) -> StdResult<FetchUserResult, UserAuthenticationError> {
        let _ = (token, for_refresh);
        Err(UserAuthenticationError::Unexpected(Self::unavailable(
            "authentication API",
        )))
    }

    async fn fetch_new_custom_token(&self) -> Result<MintCustomTokenResult> {
        Err(Self::unavailable("authentication API"))
    }

    fn on_custom_token_fetched(
        &self,
        response: Result<MintCustomTokenResult>,
    ) -> Result<String, MintCustomTokenError> {
        let _ = response;
        Err(MintCustomTokenError::Unknown)
    }

    async fn fetch_user_properties<'a>(
        &self,
        auth_token: Option<&'a str>,
    ) -> Result<GqlUserOutput> {
        let _ = auth_token;
        Err(Self::unavailable("authentication API"))
    }

    async fn get_user_settings(&self) -> Result<Option<SyncedUserSettings>> {
        Ok(None)
    }

    async fn set_is_telemetry_enabled(&self, value: bool) -> Result<()> {
        let _ = value;
        Err(Self::unavailable("authentication API"))
    }

    async fn set_is_crash_reporting_enabled(&self, value: bool) -> Result<()> {
        let _ = value;
        Err(Self::unavailable("authentication API"))
    }

    async fn set_is_cloud_conversation_storage_enabled(&self, value: bool) -> Result<()> {
        let _ = value;
        Err(Self::unavailable("authentication API"))
    }

    async fn update_user_settings(&self, input: UpdateUserSettingsInput) -> Result<()> {
        let _ = input;
        Err(Self::unavailable("authentication API"))
    }

    async fn set_user_is_onboarded(&self) -> Result<bool> {
        Err(Self::unavailable("authentication API"))
    }

    async fn request_device_code(
        &self,
    ) -> StdResult<oauth2::StandardDeviceAuthorizationResponse, UserAuthenticationError> {
        Err(UserAuthenticationError::Unexpected(Self::unavailable(
            "authentication API",
        )))
    }

    async fn exchange_device_access_token(
        &self,
        details: &oauth2::StandardDeviceAuthorizationResponse,
        timeout: Duration,
    ) -> StdResult<FirebaseToken, UserAuthenticationError> {
        let _ = (details, timeout);
        Err(UserAuthenticationError::Unexpected(Self::unavailable(
            "authentication API",
        )))
    }

    async fn list_api_keys(&self) -> Result<Vec<ApiKeyProperties>> {
        Ok(Vec::new())
    }

    async fn create_api_key(
        &self,
        name: String,
        team_id: Option<cynic::Id>,
        agent_uid: Option<cynic::Id>,
        expires_at: Option<warp_graphql::scalars::Time>,
    ) -> Result<GenerateApiKeyResult> {
        let _ = (name, team_id, agent_uid, expires_at);
        Err(Self::unavailable("API key management"))
    }

    async fn expire_api_key(&self, key_uid: &ApiKeyUid) -> Result<ExpireApiKeyResult> {
        let _ = key_uid;
        Err(Self::unavailable("API key management"))
    }

    async fn list_agent_identities(&self) -> Result<Vec<AgentIdentity>> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod tests;
