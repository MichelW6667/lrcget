use anyhow::Result;

pub use super::get::RawResponse;
pub use super::get::Response;
use super::{ResponseError, get_with_retry};

async fn make_request(id: i64, lrclib_instance: &str) -> Result<reqwest::Response> {
    let api_endpoint = format!("{}/api/get/{}", lrclib_instance.trim_end_matches('/'), id);
    let url = reqwest::Url::parse(&api_endpoint)?;
    Ok(get_with_retry(url).await?)
}

pub async fn request_raw(id: i64, lrclib_instance: &str) -> Result<RawResponse> {
    let res = make_request(id, lrclib_instance).await?;

    match res.status() {
        reqwest::StatusCode::OK => {
            let lrclib_response = res.json::<RawResponse>().await?;

            if lrclib_response.synced_lyrics.is_some()
                || lrclib_response.plain_lyrics.is_some()
                || lrclib_response.instrumental
            {
                Ok(lrclib_response)
            } else {
                Err(ResponseError {
                    status_code: Some(404),
                    error: "NotFound".to_string(),
                    message: "There is no lyrics for this track".to_string(),
                }
                .into())
            }
        }

        reqwest::StatusCode::NOT_FOUND => Err(ResponseError {
            status_code: Some(404),
            error: "NotFound".to_string(),
            message: "There is no lyrics for this track".to_string(),
        }
        .into()),

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

pub async fn request(id: i64, lrclib_instance: &str) -> Result<Response> {
    let res = make_request(id, lrclib_instance).await?;

    match res.status() {
        reqwest::StatusCode::OK => {
            let lrclib_response = res.json::<RawResponse>().await?;

            Ok(Response::from_raw_response(lrclib_response))
        }

        reqwest::StatusCode::NOT_FOUND => Ok(Response::None),

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
