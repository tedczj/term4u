use std::sync::Arc;

use futures::StreamExt as _;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::*;

fn offline_client() -> (Client, Arc<offline::OfflineResolver>) {
    let resolver = Arc::new(offline::OfflineResolver::default());
    let builder = offline::configure(reqwest::ClientBuilder::new().no_proxy(), resolver.clone());
    let client = Client {
        wrapped: builder.build().unwrap(),
        before_request_sent: None,
        after_response_received: None,
        iap_token_provider: None,
    };
    (client, resolver)
}

async fn serve_once(content_type: &str, body: &'static str) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let content_type = content_type.to_string();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0; 1024];
        let _ = stream.read(&mut request).await.unwrap();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
    });
    address
}

#[tokio::test]
async fn guard_refuses_non_loopback_http() {
    let (client, _) = offline_client();

    let error = match client.get("http://198.51.100.1/").send().await {
        Ok(_) => panic!("non-loopback request unexpectedly succeeded"),
        Err(error) => error,
    };

    assert!(offline::is_outbound_refused(&error));
}

#[tokio::test]
async fn guard_allows_loopback_http() {
    let address = serve_once("text/plain", "local").await;
    let (client, _) = offline_client();

    let body = client
        .get(format!("http://{address}/"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    assert_eq!(body, "local");
}

#[tokio::test]
async fn guard_refuses_non_loopback_eventsource() {
    let (client, _) = offline_client();
    let mut stream = client.get("http://198.51.100.1/events").eventsource();

    let error = stream.next().await.unwrap().unwrap_err();

    assert!(offline::is_outbound_refused(&error));
}

#[tokio::test]
async fn guard_allows_loopback_eventsource() {
    let address = serve_once("text/event-stream", "data: local\n\n").await;
    let (client, _) = offline_client();
    let mut stream = client.get(format!("http://{address}/events")).eventsource();

    assert_eq!(
        stream.next().await.unwrap().unwrap(),
        reqwest_eventsource::Event::Open
    );
    let event = stream.next().await.unwrap().unwrap();

    match event {
        reqwest_eventsource::Event::Message(message) => assert_eq!(message.data, "local"),
        reqwest_eventsource::Event::Open => panic!("expected an SSE message"),
    }
}

#[tokio::test]
async fn guard_makes_no_dns_query() {
    let (client, resolver) = offline_client();

    let error = match client.get("http://does-not-exist.invalid/").send().await {
        Ok(_) => panic!("non-loopback request unexpectedly succeeded"),
        Err(error) => error,
    };

    assert!(offline::is_outbound_refused(&error));
    assert_eq!(resolver.resolution_count(), 1);
}
