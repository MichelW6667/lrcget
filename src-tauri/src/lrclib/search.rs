use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::{ResponseError, HTTP_CLIENT};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchItem {
    id: i64,
    name: Option<String>,
    artist_name: Option<String>,
    album_name: Option<String>,
    duration: Option<f64>,
    instrumental: bool,
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Response(Vec<SearchItem>);

pub async fn request(
    title: &str,
    album_name: &str,
    artist_name: &str,
    q: &str,
    lrclib_instance: &str,
) -> Result<Response> {
    let params: Vec<(String, String)> = vec![
        ("track_name".to_owned(), title.to_owned()),
        ("artist_name".to_owned(), artist_name.to_owned()),
        ("album_name".to_owned(), album_name.to_owned()),
        ("q".to_owned(), q.to_owned()),
    ];

    let api_endpoint = format!("{}/api/search", lrclib_instance.trim_end_matches('/'));
    let url = reqwest::Url::parse_with_params(&api_endpoint, &params)?;
    let res = HTTP_CLIENT.get(url).send().await?;

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
