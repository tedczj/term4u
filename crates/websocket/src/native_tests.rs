use async_tungstenite::tungstenite::client::IntoClientRequest as _;

use super::*;

#[tokio::test]
async fn guard_refuses_non_loopback_websocket() {
    let request = "wss://198.51.100.1/".into_client_request().unwrap();

    let error = connect_direct_offline(request, None)
        .await
        .expect_err("non-loopback WebSocket must be refused");

    assert!(error.chain().any(|source| {
        source
            .downcast_ref::<offline_guard::OutboundRefused>()
            .is_some()
    }));
}
