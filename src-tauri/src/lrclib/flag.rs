use anyhow::Result;
use serde::Serialize;

use super::{ResponseError, HTTP_CLIENT};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    track_id: i64,
    reason: String,
}

pub async fn request(
    track_id: i64,
    reason: &str,
    publish_token: &str,
    lrclib_instance: &str,
) -> Result<()> {
    let data = Request {
        track_id,
        reason: reason.to_owned(),
    };

    let api_endpoint = format!("{}/api/flag", lrclib_instance.trim_end_matches('/'));
    let url = reqwest::Url::parse(&api_endpoint)?;
    let res = HTTP_CLIENT
        .post(url)
        .header("X-Publish-Token", publish_token)
        .json(&data)
        .send()
        .await?;

    match res.status() {
        reqwest::StatusCode::CREATED => Ok(()),

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
