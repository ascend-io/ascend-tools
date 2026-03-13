#![cfg_attr(not(feature = "jsts"), forbid(unsafe_code))]
#![cfg_attr(feature = "jsts", deny(unsafe_code))]

pub mod auth;
pub mod client;
pub mod config;
pub mod error;
pub mod models;
pub mod sse;

use ureq::Agent;

pub use error::{Error, Result};

fn tls_config() -> ureq::tls::TlsConfig {
    ureq::tls::TlsConfig::builder()
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build()
}

fn user_agent() -> &'static str {
    concat!("ascend-tools/", env!("CARGO_PKG_VERSION"))
}

/// Agent for normal API requests (30-second global timeout).
pub(crate) fn new_agent() -> Agent {
    Agent::new_with_config(
        ureq::config::Config::builder()
            .tls_config(tls_config())
            .http_status_as_error(false)
            .timeout_global(Some(std::time::Duration::from_secs(30)))
            .user_agent(user_agent())
            .build(),
    )
}

/// Agent for SSE streaming requests (no global timeout).
pub(crate) fn new_streaming_agent() -> Agent {
    Agent::new_with_config(
        ureq::config::Config::builder()
            .tls_config(tls_config())
            .http_status_as_error(false)
            .user_agent(user_agent())
            .build(),
    )
}
