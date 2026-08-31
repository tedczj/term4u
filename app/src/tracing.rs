use tracing::subscriber;

pub fn init() -> anyhow::Result<Initialization> {
    install_no_subscriber()?;
    Ok(Initialization)
}

fn install_no_subscriber() -> anyhow::Result<()> {
    subscriber::set_global_default(subscriber::NoSubscriber::new())?;
    Ok(())
}

#[derive(Default)]
pub struct Initialization;

impl Initialization {
    pub fn log_initialization_warning(&mut self) {}

    pub(crate) fn shutdown(&mut self) {}
}
