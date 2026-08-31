use warpui::{AppContext, Entity, ModelContext, SingletonEntity};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum AutoupdateStage {
    #[default]
    NoUpdateAvailable,
}

impl AutoupdateStage {
    pub fn ready_for_update(&self) -> bool {
        false
    }

    pub fn available_new_version(&self) -> Option<&channel_versions::VersionInfo> {
        None
    }
}

pub struct AutoupdateState {
    stage: AutoupdateStage,
}

impl AutoupdateState {
    pub fn new<T>(ignored: T) -> Self {
        let _ = ignored;
        Self {
            stage: AutoupdateStage::NoUpdateAvailable,
        }
    }

    pub fn register<T: 'static>(ctx: &mut AppContext, ignored: T) {
        ctx.add_singleton_model(move |_| Self::new(ignored));
    }

    pub fn maybe_daily_check_for_update(&mut self, ctx: &mut ModelContext<Self>) {
        let _ = ctx;
    }

    pub fn manually_check_for_update(&mut self, ctx: &mut ModelContext<Self>) {
        let _ = ctx;
    }
}

impl Entity for AutoupdateState {
    type Event = ();
}

impl SingletonEntity for AutoupdateState {}

pub fn get_update_state(app: &AppContext) -> AutoupdateStage {
    AutoupdateState::as_ref(app).stage.clone()
}

pub fn initiate_relaunch_for_update(app: &mut AppContext) {
    let _ = app;
}

pub fn apply_pending_update<F>(app: &mut AppContext, on_update_complete: F) -> bool
where
    F: FnOnce(&mut AppContext) + Send + 'static,
{
    let _ = (app, on_update_complete);
    false
}

pub fn cancel_relaunch(app: &mut AppContext) {
    let _ = app;
}

pub fn spawn_child_if_necessary(app: &mut AppContext) {
    let _ = app;
}

pub fn manually_download_new_version(ctx: &mut AppContext) {
    let _ = ctx;
}

#[derive(Clone, Copy, Default)]
pub struct RelaunchModel;

impl RelaunchModel {
    pub fn new() -> Self {
        Self
    }
}

impl Entity for RelaunchModel {
    type Event = ();
}

impl SingletonEntity for RelaunchModel {}

pub fn is_incoming_version_past_current(version: Option<&str>) -> bool {
    let _ = version;
    false
}
