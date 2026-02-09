use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::{ResponseError, get_with_retry};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchItem {
    pub id: i64,
    pub name: Option<String>,
    pub artist_name: Option<String>,
    pub album_name: Option<String>,
    pub duration: Option<f64>,
    pub instrumental: bool,
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Response(pub Vec<SearchItem>);

pub async fn request(
    title: &str,
    album_name: &str,
    artist_name: &str,
    q: &str,
    lrclib_instance: &str,
) -> Result<Response> {
    let mut params: Vec<(String, String)> = Vec::new();
    if !title.is_empty() {
        params.push(("track_name".to_owned(), title.to_owned()));
    }
    if !artist_name.is_empty() {
        params.push(("artist_name".to_owned(), artist_name.to_owned()));
    }
    if !album_name.is_empty() {
        params.push(("album_name".to_owned(), album_name.to_owned()));
    }
    if !q.is_empty() {
        params.push(("q".to_owned(), q.to_owned()));
    }

    let api_endpoint = format!("{}/api/search", lrclib_instance.trim_end_matches('/'));
    let url = reqwest::Url::parse_with_params(&api_endpoint, &params)?;
    let res = get_with_retry(url).await?;

    match res.status() {
        reqwest::StatusCode::OK => {
            let lrclib_response = res.json::<Response>().await?;
            Ok(lrclib_response)
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
