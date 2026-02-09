use crate::db;
use crate::lrclib;
use crate::lyrics;
use crate::state::ServiceAccess;
use crate::utils::RE_INSTRUMENTAL;
use rusqlite::Connection;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublishLyricsProgress {
    request_challenge: String,
    solve_challenge: String,
    publish_lyrics: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FlagLyricsProgress {
    request_challenge: String,
    solve_challenge: String,
    flag_lyrics: String,
}

#[tauri::command]
pub async fn download_lyrics(track_id: i64, app_handle: AppHandle) -> Result<String, String> {
    let track = app_handle
        .db(|db| db::get_track_by_id(track_id, db))
        .map_err(|err| err.to_string())?;

    // Skip if track already has synced lyrics (already best quality)
    let has_synced = track.lrc_lyrics.as_ref().is_some_and(|l| l != "[au: instrumental]");
    if has_synced {
        return Ok("Skipped: already has synced lyrics".to_owned());
    }
    let has_plain = track.txt_lyrics.is_some();

    let config = app_handle
        .db(|db| db::get_config(db))
        .map_err(|err| err.to_string())?;
    let lyrics =
        lyrics::download_lyrics_for_track(track, config.try_embed_lyrics, &config.lrclib_instance)
            .await
            .map_err(|err| err.to_string())?;
    match lyrics {
        lrclib::get::Response::SyncedLyrics(synced_lyrics, plain_lyrics) => {
            app_handle
                .db(|db: &Connection| {
                    db::update_track_synced_lyrics(track_id, &synced_lyrics, &plain_lyrics, db)
                })
                .map_err(|err| err.to_string())?;
            let _ = app_handle.emit("reload-track-id", track_id);
            Ok("Synced lyrics downloaded".to_owned())
        }
        lrclib::get::Response::UnsyncedLyrics(plain_lyrics) => {
            if has_plain {
                // Skip: track already has plain lyrics and no synced upgrade is available
                return Ok("Skipped: already has plain lyrics, no synced available".to_owned());
            }
            app_handle
                .db(|db: &Connection| db::update_track_plain_lyrics(track_id, &plain_lyrics, db))
                .map_err(|err| err.to_string())?;
            let _ = app_handle.emit("reload-track-id", track_id);
            Ok("Plain lyrics downloaded".to_owned())
        }
        lrclib::get::Response::IsInstrumental => {
            app_handle
                .db(|db: &Connection| db::update_track_instrumental(track_id, db))
                .map_err(|err| err.to_string())?;
            Ok("Marked track as instrumental".to_owned())
        }
        lrclib::get::Response::None => Err(lyrics::GetLyricsError::NotFound.to_string()),
    }
}

#[tauri::command]
pub async fn apply_lyrics(
    track_id: i64,
    lrclib_response: lrclib::get::RawResponse,
    app_handle: AppHandle,
) -> Result<String, String> {
    let track = app_handle
        .db(|db| db::get_track_by_id(track_id, db))
        .map_err(|err| err.to_string())?;
    let is_try_embed_lyrics = app_handle
        .db(|db| db::get_config(db))
        .map_err(|err| err.to_string())?
        .try_embed_lyrics;

    let lyrics = lrclib::get::Response::from_raw_response(lrclib_response);
    let lyrics = lyrics::apply_lyrics_for_track(track, lyrics, is_try_embed_lyrics)
        .await
        .map_err(|err| err.to_string())?;

    match lyrics {
        lrclib::get::Response::SyncedLyrics(synced_lyrics, plain_lyrics) => {
            app_handle
                .db(|db: &Connection| {
                    db::update_track_synced_lyrics(track_id, &synced_lyrics, &plain_lyrics, db)
                })
                .map_err(|err| err.to_string())?;
            let _ = app_handle.emit("reload-track-id", track_id);
            Ok("Synced lyrics downloaded".to_owned())
        }
        lrclib::get::Response::UnsyncedLyrics(plain_lyrics) => {
            app_handle
                .db(|db: &Connection| db::update_track_plain_lyrics(track_id, &plain_lyrics, db))
                .map_err(|err| err.to_string())?;
            let _ = app_handle.emit("reload-track-id", track_id);
            Ok("Plain lyrics downloaded".to_owned())
        }
        lrclib::get::Response::IsInstrumental => {
            app_handle
                .db(|db: &Connection| db::update_track_instrumental(track_id, db))
                .map_err(|err| err.to_string())?;
            Ok("Marked track as instrumental".to_owned())
        }
        lrclib::get::Response::None => Err(lyrics::GetLyricsError::NotFound.to_string()),
    }
}

#[tauri::command]
pub async fn retrieve_lyrics(
    title: String,
    album_name: String,
    artist_name: String,
    duration: f64,
    app_handle: AppHandle,
) -> Result<lrclib::get::RawResponse, String> {
    let config = app_handle
        .db(|db: &Connection| db::get_config(db))
        .map_err(|err| err.to_string())?;

    let response = lrclib::get::request_raw(
        &title,
        &album_name,
        &artist_name,
        duration,
        &config.lrclib_instance,
    )
    .await
    .map_err(|err| err.to_string())?;

    Ok(response)
}

#[tauri::command]
pub async fn retrieve_lyrics_by_id(
    id: i64,
    app_handle: AppHandle,
) -> Result<lrclib::get_by_id::RawResponse, String> {
    let config = app_handle
        .db(|db: &Connection| db::get_config(db))
        .map_err(|err| err.to_string())?;

    let response = lrclib::get_by_id::request_raw(id, &config.lrclib_instance)
        .await
        .map_err(|err| err.to_string())?;

    Ok(response)
}

#[tauri::command]
pub async fn search_lyrics(
    title: String,
    album_name: String,
    artist_name: String,
    q: String,
    app_handle: AppHandle,
) -> Result<lrclib::search::Response, String> {
    let config = app_handle
        .db(|db: &Connection| db::get_config(db))
        .map_err(|err| err.to_string())?;
    let response = lrclib::search::request(
        &title,
        &album_name,
        &artist_name,
        &q,
        &config.lrclib_instance,
    )
    .await
    .map_err(|err| err.to_string())?;

    Ok(response)
}

#[tauri::command]
pub async fn save_lyrics(
    track_id: i64,
    plain_lyrics: String,
    synced_lyrics: String,
    app_handle: AppHandle,
) -> Result<String, String> {
    let track = app_handle
        .db(|db| db::get_track_by_id(track_id, db))
        .map_err(|err| err.to_string())?;
    let is_try_embed_lyrics = app_handle
        .db(|db| db::get_config(db))
        .map_err(|err| err.to_string())?
        .try_embed_lyrics;

    let is_instrumental = RE_INSTRUMENTAL.is_match(&synced_lyrics);

    lyrics::apply_string_lyrics_for_track(
        &track,
        &plain_lyrics,
        &synced_lyrics,
        is_try_embed_lyrics,
    )
    .await
    .map_err(|err| err.to_string())?;

    if is_instrumental {
        app_handle
            .db(|db: &Connection| db::update_track_instrumental(track.id, db))
            .map_err(|err| err.to_string())?;
    } else if !synced_lyrics.is_empty() {
        app_handle
            .db(|db: &Connection| {
                db::update_track_synced_lyrics(track.id, &synced_lyrics, &plain_lyrics, db)
            })
            .map_err(|err| err.to_string())?;
    } else if !plain_lyrics.is_empty() {
        app_handle
            .db(|db: &Connection| db::update_track_plain_lyrics(track.id, &plain_lyrics, db))
            .map_err(|err| err.to_string())?;
    } else {
        app_handle
            .db(|db: &Connection| db::update_track_null_lyrics(track.id, db))
            .map_err(|err| err.to_string())?;
    }

    let _ = app_handle.emit("reload-track-id", track_id);

    Ok("Lyrics saved successfully".to_owned())
}

#[tauri::command]
pub async fn publish_lyrics(
    title: String,
    album_name: String,
    artist_name: String,
    duration: f64,
    plain_lyrics: String,
    synced_lyrics: String,
    app_handle: AppHandle,
) -> Result<(), String> {
    let config = app_handle
        .db(|db: &Connection| db::get_config(db))
        .map_err(|err| err.to_string())?;

    let mut progress = PublishLyricsProgress {
        request_challenge: "Pending".to_owned(),
        solve_challenge: "Pending".to_owned(),
        publish_lyrics: "Pending".to_owned(),
    };
    progress.request_challenge = "In Progress".to_owned();
    app_handle
        .emit("publish-lyrics-progress", &progress)
        .ok();
    let challenge_response = lrclib::request_challenge::request(&config.lrclib_instance)
        .await
        .map_err(|err| err.to_string())?;
    progress.request_challenge = "Done".to_owned();
    progress.solve_challenge = "In Progress".to_owned();
    app_handle
        .emit("publish-lyrics-progress", &progress)
        .ok();
    let prefix = challenge_response.prefix.clone();
    let target = challenge_response.target.clone();
    let nonce = tokio::task::spawn_blocking(move || {
        lrclib::challenge_solver::solve_challenge(&prefix, &target)
    })
    .await
    .map_err(|err| err.to_string())?;
    progress.solve_challenge = "Done".to_owned();
    progress.publish_lyrics = "In Progress".to_owned();
    app_handle
        .emit("publish-lyrics-progress", &progress)
        .ok();
    let publish_token = format!("{}:{}", challenge_response.prefix, nonce);
    lrclib::publish::request(
        &title,
        &album_name,
        &artist_name,
        duration,
        &plain_lyrics,
        &synced_lyrics,
        &publish_token,
        &config.lrclib_instance,
    )
    .await
    .map_err(|err| err.to_string())?;
    progress.publish_lyrics = "Done".to_owned();
    app_handle
        .emit("publish-lyrics-progress", &progress)
        .ok();
    Ok(())
}

#[tauri::command]
pub async fn flag_lyrics(
    track_id: i64,
    flag_reason: String,
    app_handle: AppHandle,
) -> Result<(), String> {
    let config = app_handle
        .db(|db: &Connection| db::get_config(db))
        .map_err(|err| err.to_string())?;

    let mut progress = FlagLyricsProgress {
        request_challenge: "Pending".to_owned(),
        solve_challenge: "Pending".to_owned(),
        flag_lyrics: "Pending".to_owned(),
    };
    progress.request_challenge = "In Progress".to_owned();
    app_handle
        .emit("flag-lyrics-progress", &progress)
        .ok();
    let challenge_response = lrclib::request_challenge::request(&config.lrclib_instance)
        .await
        .map_err(|err| err.to_string())?;
    progress.request_challenge = "Done".to_owned();
    progress.solve_challenge = "In Progress".to_owned();
    app_handle
        .emit("flag-lyrics-progress", &progress)
        .ok();
    let prefix = challenge_response.prefix.clone();
    let target = challenge_response.target.clone();
    let nonce = tokio::task::spawn_blocking(move || {
        lrclib::challenge_solver::solve_challenge(&prefix, &target)
    })
    .await
    .map_err(|err| err.to_string())?;
    progress.solve_challenge = "Done".to_owned();
    progress.flag_lyrics = "In Progress".to_owned();
    app_handle
        .emit("flag-lyrics-progress", &progress)
        .ok();
    let publish_token = format!("{}:{}", challenge_response.prefix, nonce);
    lrclib::flag::request(
        track_id,
        &flag_reason,
        &publish_token,
        &config.lrclib_instance,
    )
    .await
    .map_err(|err| err.to_string())?;
    progress.flag_lyrics = "Done".to_owned();
    app_handle
        .emit("flag-lyrics-progress", &progress)
        .ok();
    Ok(())
}
