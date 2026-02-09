use anyhow::Result;
use serde::Deserialize;

use super::{ResponseError, HTTP_CLIENT};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub prefix: String,
    pub target: String,
}

pub async fn request(lrclib_instance: &str) -> Result<Response> {
    let api_endpoint = format!(
        "{}/api/request-challenge",
        lrclib_instance.trim_end_matches('/')
    );
    let url = reqwest::Url::parse(&api_endpoint)?;
    let res = HTTP_CLIENT.post(url).send().await?;

    match res.status() {
        reqwest::StatusCode::OK => {
            let response = res.json::<Response>().await?;
            Ok(response)
        }

        reqwest::StatusCode::BAD_REQUEST
        | reqwest::StatusCode::SERVICE_UNAVAILABLE
        | reqwest::StatusCode::INTERNAL_SERVER_ERROR => {
            let error = res.json::<ResponseError>().await?;
            Err(error.into())
        }

        _ => Err(ResponseError {
            status_code: None,
            error: "UnknownError".to_string(),
            message: "Unknown error happened".to_string(),
        }
        .into()),
    }
}
