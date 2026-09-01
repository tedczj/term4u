use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use offline_guard::OutboundRefused;
use offline_guard::loopback_addrs;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};

#[derive(Default)]
pub struct OfflineResolver {
    resolutions: AtomicUsize,
}

impl OfflineResolver {
    #[cfg(test)]
    pub fn resolution_count(&self) -> usize {
        self.resolutions.load(Ordering::Relaxed)
    }
}

impl Resolve for OfflineResolver {
    fn resolve(&self, name: Name) -> Resolving {
        self.resolutions.fetch_add(1, Ordering::Relaxed);
        let host = name.as_str().to_string();
        Box::pin(async move {
            let addresses = loopback_addrs(&host, 0)
                .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)?;
            let addresses: Addrs = Box::new(addresses.into_iter());
            Ok(addresses)
        })
    }
}

pub fn configure(
    builder: reqwest::ClientBuilder,
    resolver: Arc<OfflineResolver>,
) -> reqwest::ClientBuilder {
    builder
        .no_proxy()
        .dns_resolver(resolver)
        .proxy(reqwest::Proxy::custom(|url| {
            let host = url.host_str()?;
            if is_loopback_host(host) {
                return None;
            }
            let label: String = host
                .chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() {
                        character
                    } else {
                        '-'
                    }
                })
                .collect();
            reqwest::Url::parse(&format!("http://{label}.term4u-refused.invalid")).ok()
        }))
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

pub fn is_outbound_refused(error: &(dyn std::error::Error + 'static)) -> bool {
    let mut source = Some(error);
    while let Some(error) = source {
        if error.downcast_ref::<OutboundRefused>().is_some() {
            return true;
        }
        source = error.source();
    }
    false
}
