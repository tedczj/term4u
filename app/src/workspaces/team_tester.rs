use warpui::{Entity, ModelContext, SingletonEntity};

#[derive(Clone)]
pub struct TeamTesterStatus {}

impl TeamTesterStatus {
    #[cfg(test)]
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        let _ = ctx;
        Self {}
    }

    #[cfg(test)]
    pub fn mock(ctx: &mut ModelContext<Self>) -> Self {
        Self::new(ctx)
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
