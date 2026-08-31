pub use cloud_object_client::ObjectUpdateMessage;
#[cfg(any(test, feature = "test-util", feature = "integration_tests"))]
use warpui::{Entity, SingletonEntity};

#[cfg(any(test, feature = "test-util", feature = "integration_tests"))]
pub struct Listener;

#[cfg(any(test, feature = "test-util", feature = "integration_tests"))]
impl Listener {
    pub fn has_current_subscription_abort_handle(&self) -> bool {
        false
    }

    pub fn mock(_: &mut warpui::ModelContext<Self>) -> Self {
        Self
    }
}

#[cfg(any(test, feature = "test-util", feature = "integration_tests"))]
impl Entity for Listener {
    type Event = ();
}

#[cfg(any(test, feature = "test-util", feature = "integration_tests"))]
impl SingletonEntity for Listener {}
