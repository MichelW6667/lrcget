use anyhow::Result;
use serde::Serialize;

use super::{ResponseError, HTTP_CLIENT};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    track_name: String,
    album_name: String,
    artist_name: String,
    duration: f64,
    plain_lyrics: String,
    synced_lyrics: String,
}

pub async fn request(
    title: &str,
    album_name: &str,
    artist_name: &str,
    duration: f64,
    plain_lyrics: &str,
    synced_lyrics: &str,
    publish_token: &str,
    lrclib_instance: &str,
) -> Result<()> {
    let data = Request {
        artist_name: artist_name.to_owned(),
        track_name: title.to_owned(),
        album_name: album_name.to_owned(),
        duration: duration.round(),
        plain_lyrics: plain_lyrics.to_owned(),
        synced_lyrics: synced_lyrics.to_owned(),
    };

    let api_endpoint = format!("{}/api/publish", lrclib_instance.trim_end_matches('/'));
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
