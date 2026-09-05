use std::sync::mpsc::SyncSender;

use warpui::{Entity, ModelHandle, SingletonEntity};

use crate::banner::BannerState;
use crate::persistence::ModelEvent;
use crate::resource_center::TipsCompleted;

#[derive(Clone)]
pub struct GlobalResourceHandles {
    pub model_event_sender: Option<SyncSender<ModelEvent>>,
    pub tips_completed: ModelHandle<TipsCompleted>,
    pub user_default_shell_unsupported_banner_model_handle: ModelHandle<BannerState>,
}

impl GlobalResourceHandles {
    #[cfg(any(test, feature = "integration_tests", feature = "test-util"))]
    pub fn mock(app: &mut warpui::App) -> Self {
        Self {
            model_event_sender: None,
            tips_completed: app.add_model(|_| TipsCompleted::default()),
            user_default_shell_unsupported_banner_model_handle: app.add_model(|_| BannerState::default()),
        }
    }
}

pub struct GlobalResourceHandlesProvider {
    global_resources: GlobalResourceHandles,
}

impl GlobalResourceHandlesProvider {
    pub fn get(&self) -> &GlobalResourceHandles {
        &self.global_resources
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn set_model_event_sender_for_test(&mut self, sender: SyncSender<ModelEvent>) {
        self.global_resources.model_event_sender = Some(sender);
    }

    pub(super) fn new(global_resources: GlobalResourceHandles) -> Self {
        Self { global_resources }
    }
}

impl Entity for GlobalResourceHandlesProvider { type Event = (); }
impl SingletonEntity for GlobalResourceHandlesProvider {}
