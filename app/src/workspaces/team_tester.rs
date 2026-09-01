use warpui::{Entity, ModelContext, SingletonEntity};

#[derive(Clone)]
pub struct TeamTesterStatus {}

impl TeamTesterStatus {
    pub fn new_local(_ctx: &mut ModelContext<Self>) -> Self {
        Self {}
    }

    #[cfg(test)]
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        Self::new_local(ctx)
    }

    #[cfg(test)]
    pub fn mock(ctx: &mut ModelContext<Self>) -> Self {
        Self::new_local(ctx)
    }

    pub fn initiate_data_pollers(&mut self, ctx: &mut ModelContext<Self>) {
        ctx.emit(TeamTesterStatusEvent::InitiateDataPollers)
    }
}

pub enum TeamTesterStatusEvent {
    InitiateDataPollers,
}

impl Entity for TeamTesterStatus {
    type Event = TeamTesterStatusEvent;
}

impl SingletonEntity for TeamTesterStatus {}
