use std::sync::Arc;

pub struct OfflineApi {
    http_client: Arc<http_client::Client>,
}

impl OfflineApi {
    pub fn new() -> Self {
        Self {
            http_client: Arc::new(http_client::Client::new()),
        }
    }

    pub(crate) fn unavailable(capability: &'static str) -> anyhow::Error {
        warp_core::channel::OfflineError::new(capability).into()
    }

    pub(crate) fn http_client(&self) -> &http_client::Client {
        &self.http_client
    }

    pub(crate) fn owned_http_client(&self) -> Arc<http_client::Client> {
        self.http_client.clone()
    }
}
