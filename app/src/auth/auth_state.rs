use std::sync::Arc;

use parking_lot::RwLock;
use warpui::{AppContext, Entity, SingletonEntity};

use super::UserUid;

#[derive(Clone)]
struct LocalTestIdentity {
    uid: UserUid,
    email: String,
}

/// Process-local identity state. Production instances never contain an account or credential.
pub struct AuthState {
    test_identity: RwLock<Option<LocalTestIdentity>>,
}

impl AuthState {
    pub fn new_local(_ctx: &AppContext) -> Self {
        Self {
            test_identity: RwLock::new(None),
        }
    }

    pub fn new_offline() -> Self {
        Self {
            test_identity: RwLock::new(None),
        }
    }

    #[cfg(any(test, feature = "test-util", feature = "integration_tests"))]
    pub fn new_for_test() -> Self {
        Self {
            test_identity: RwLock::new(Some(LocalTestIdentity {
                uid: UserUid::new("local-test-user"),
                email: "local-test@example.invalid".to_owned(),
            })),
        }
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn new_logged_out_for_test() -> Self {
        Self::new_offline()
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn new_anonymous_for_test() -> Self {
        Self::new_for_test()
    }

    pub fn is_logged_in(&self) -> bool {
        false
    }

    pub fn user_id(&self) -> Option<UserUid> {
        self.test_identity.read().as_ref().map(|identity| identity.uid)
    }

    pub fn user_email(&self) -> Option<String> {
        self.test_identity
            .read()
            .as_ref()
            .map(|identity| identity.email.clone())
    }

    pub fn username_for_display(&self) -> Option<String> {
        self.user_email()
    }

    pub fn display_name(&self) -> Option<String> {
        None
    }

    pub fn is_anonymous_or_logged_out(&self) -> bool {
        true
    }

    pub fn is_user_anonymous(&self) -> Option<bool> {
        self.test_identity.read().as_ref().map(|_| false)
    }

    pub fn is_user_web_anonymous_user(&self) -> Option<bool> {
        self.test_identity.read().as_ref().map(|_| false)
    }

    pub fn is_anonymous_user_feature_gated(&self) -> Option<bool> {
        self.test_identity.read().as_ref().map(|_| false)
    }

    pub fn user_photo_url(&self) -> Option<String> {
        None
    }

    pub fn global_skills(&self) -> Vec<String> {
        Vec::new()
    }

    pub fn needs_reauth(&self) -> bool {
        false
    }

    pub fn anonymous_id(&self) -> String {
        "local".to_owned()
    }
}

pub struct AuthStateProvider {
    auth_state: Arc<AuthState>,
}

impl AuthStateProvider {
    pub fn new(auth_state: Arc<AuthState>) -> Self {
        Self { auth_state }
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn new_for_test() -> Self {
        Self::new(Arc::new(AuthState::new_for_test()))
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn new_logged_out_for_test() -> Self {
        Self::new(Arc::new(AuthState::new_logged_out_for_test()))
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn new_anonymous_for_test() -> Self {
        Self::new(Arc::new(AuthState::new_anonymous_for_test()))
    }

    pub fn get(&self) -> &Arc<AuthState> {
        &self.auth_state
    }
}

impl Entity for AuthStateProvider {
    type Event = ();
}

impl SingletonEntity for AuthStateProvider {}

pub struct LocalAuthStateProvider {
    auth_state: Arc<AuthState>,
}

impl LocalAuthStateProvider {
    pub fn new(auth_state: Arc<AuthState>) -> Self {
        Self { auth_state }
    }

    pub fn get(&self) -> &Arc<AuthState> {
        &self.auth_state
    }
}

impl Entity for LocalAuthStateProvider {
    type Event = ();
}

impl SingletonEntity for LocalAuthStateProvider {}
