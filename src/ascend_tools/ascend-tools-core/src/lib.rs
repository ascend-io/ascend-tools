#![forbid(unsafe_code)]

pub mod auth;
pub mod client;
pub mod config;
pub mod models;

use ureq::Agent;

pub(crate) fn new_agent() -> Agent {
    Agent::new_with_config(
        ureq::config::Config::builder()
            .tls_config(
                ureq::tls::TlsConfig::builder()
                    .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                    .build(),
            )
            .http_status_as_error(false)
            .timeout_global(Some(std::time::Duration::from_secs(30)))
            .user_agent(concat!("ascend-tools/", env!("CARGO_PKG_VERSION")))
            .build(),
    )
}
