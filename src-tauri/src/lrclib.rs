pub mod challenge_solver;
pub mod flag;
pub mod get;
pub mod get_by_id;
pub mod publish;
pub mod request_challenge;
pub mod search;

use std::sync::LazyLock;
use std::time::Duration;

use anyhow::Result;
use serde::Deserialize;
use thiserror::Error;

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 1000;

/// Shared HTTP client with connection pooling and TLS session caching.
pub static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    let version = env!("CARGO_PKG_VERSION");
    let user_agent = format!(
        "LRCGET v{} (https://github.com/MichelW6667/lrcget)",
        version
    );
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(user_agent)
        .build()
        .expect("Failed to create HTTP client")
});

/// Send a GET request with automatic retry on network errors.
pub async fn get_with_retry(url: reqwest::Url) -> Result<reqwest::Response> {
    let mut last_err = None;
    for attempt in 0..MAX_RETRIES {
        match HTTP_CLIENT.get(url.clone()).send().await {
            Ok(response) => return Ok(response),
            Err(e) => {
                // Only retry on network/timeout errors, not on HTTP status errors
                if e.is_connect() || e.is_timeout() || e.is_request() {
                    println!("Request failed (attempt {}/{}): {}", attempt + 1, MAX_RETRIES, e);
                    last_err = Some(e);
                    if attempt + 1 < MAX_RETRIES {
                        tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS * (attempt as u64 + 1))).await;
                    }
                } else {
                    return Err(e.into());
                }
            }
        }
    }
    Err(last_err.unwrap().into())
}

/// Send a POST request with automatic retry on network errors.
pub async fn post_with_retry(request: reqwest::RequestBuilder) -> Result<reqwest::Response> {
    let mut last_err = None;
    for attempt in 0..MAX_RETRIES {
        match request.try_clone().unwrap().send().await {
            Ok(response) => return Ok(response),
            Err(e) => {
                if e.is_connect() || e.is_timeout() || e.is_request() {
                    println!("Request failed (attempt {}/{}): {}", attempt + 1, MAX_RETRIES, e);
                    last_err = Some(e);
                    if attempt + 1 < MAX_RETRIES {
                        tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS * (attempt as u64 + 1))).await;
                    }
                } else {
                    return Err(e.into());
                }
            }
        }
    }
    Err(last_err.unwrap().into())
}

/// Shared error type for all LRCLIB API responses.
#[derive(Error, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[error("{error}: {message}")]
pub struct ResponseError {
    pub status_code: Option<u16>,
    pub error: String,
    pub message: String,
}
