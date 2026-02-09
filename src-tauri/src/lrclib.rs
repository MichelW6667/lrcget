pub mod challenge_solver;
pub mod flag;
pub mod get;
pub mod get_by_id;
pub mod publish;
pub mod request_challenge;
pub mod search;

use std::sync::LazyLock;
use std::time::Duration;

use serde::Deserialize;
use thiserror::Error;

/// Shared HTTP client with connection pooling and TLS session caching.
pub static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    let version = env!("CARGO_PKG_VERSION");
    let user_agent = format!(
        "LRCGET v{} (https://github.com/MichelW6667/lrcget)",
        version
    );
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(user_agent)
        .build()
        .expect("Failed to create HTTP client")
});

/// Shared error type for all LRCLIB API responses.
#[derive(Error, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[error("{error}: {message}")]
pub struct ResponseError {
    pub status_code: Option<u16>,
    pub error: String,
    pub message: String,
}
