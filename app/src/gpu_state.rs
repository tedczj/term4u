use warpui::{Entity, SingletonEntity};

#[derive(Debug, Default, Clone)]
pub struct GPUState;

impl GPUState {
    pub fn new() -> Self {
        Self
    }

    pub fn is_low_power_gpu_available(&self) -> bool {
        false
    }
}

impl SingletonEntity for GPUState {}

impl Entity for GPUState {
    type Event = ();
}
