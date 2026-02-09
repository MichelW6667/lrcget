use crate::db;
use crate::library;
use crate::persistent_entities::{PersistentAlbum, PersistentArtist, PersistentConfig, PersistentTrack};
use crate::state::AppState;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn get_directories(app_state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let conn_guard = app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;
    let directories = db::get_directories(conn);
    match directories {
        Ok(directories) => Ok(directories),
        Err(error) => Err(format!(
            "Cannot get existing directories from database. Error: {}",
            error
        )),
    }
}

#[tauri::command]
pub async fn set_directories(
    directories: Vec<String>,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    let conn_guard = app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;
    db::set_directories(directories, conn).map_err(|err| err.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_init(app_state: State<'_, AppState>) -> Result<bool, String> {
    let conn_guard = app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;
    let init = library::get_init(conn).map_err(|err| err.to_string())?;

    Ok(init)
}

#[tauri::command]
pub async fn get_config(app_state: State<'_, AppState>) -> Result<PersistentConfig, String> {
    let conn_guard = app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;
    let config = db::get_config(conn).map_err(|err| err.to_string())?;

    Ok(config)
}

#[tauri::command]
pub async fn set_config(
    skip_tracks_with_synced_lyrics: bool,
    skip_tracks_with_plain_lyrics: bool,
    show_line_count: bool,
    try_embed_lyrics: bool,
    theme_mode: &str,
    lrclib_instance: &str,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    let conn_guard = app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;
    db::set_config(
        skip_tracks_with_synced_lyrics,
        skip_tracks_with_plain_lyrics,
        show_line_count,
        try_embed_lyrics,
        theme_mode,
        lrclib_instance,
        conn,
    )
    .map_err(|err| err.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn initialize_library(
    app_state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let mut conn = app_state.db.lock()
        .map_err(|e| format!("Database lock error: {}", e))?
        .take()
        .ok_or("Database not initialized")?;

    let (conn, result) = tokio::task::spawn_blocking(move || {
        let result = library::initialize_library(&mut conn, app_handle);
        (conn, result)
    })
    .await
    .map_err(|err| err.to_string())?;

    *app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))? = Some(conn);
    result.map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn uninitialize_library(app_state: State<'_, AppState>) -> Result<(), String> {
    let conn_guard = app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;

    library::uninitialize_library(conn).map_err(|err| err.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn refresh_library(
    app_state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let mut conn = app_state.db.lock()
        .map_err(|e| format!("Database lock error: {}", e))?
        .take()
        .ok_or("Database not initialized")?;

    let (conn, result) = tokio::task::spawn_blocking(move || {
        library::uninitialize_library(&conn).ok();
        let result = library::initialize_library(&mut conn, app_handle);
        (conn, result)
    })
    .await
    .map_err(|err| err.to_string())?;

    *app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))? = Some(conn);
    result.map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_tracks(app_state: State<'_, AppState>) -> Result<Vec<PersistentTrack>, String> {
    let conn_guard = app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;
    let tracks = library::get_tracks(conn).map_err(|err| err.to_string())?;

    Ok(tracks)
}

#[tauri::command]
pub async fn get_track_ids(
    search_query: Option<String>,
    synced_lyrics_tracks: Option<bool>,
    plain_lyrics_tracks: Option<bool>,
    instrumental_tracks: Option<bool>,
    no_lyrics_tracks: Option<bool>,
    app_state: State<'_, AppState>,
) -> Result<Vec<i64>, String> {
    let conn_guard = app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;
    let search_query = search_query.filter(|s| !s.is_empty());
    let track_ids = library::get_track_ids(
        search_query,
        synced_lyrics_tracks.unwrap_or(true),
        plain_lyrics_tracks.unwrap_or(true),
        instrumental_tracks.unwrap_or(true),
        no_lyrics_tracks.unwrap_or(true),
        conn,
    )
    .map_err(|err| err.to_string())?;

    Ok(track_ids)
}

#[tauri::command]
pub async fn get_track(
    track_id: i64,
    app_state: State<'_, AppState>,
) -> Result<PersistentTrack, String> {
    let conn_guard = app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;
    let track = library::get_track(track_id, conn).map_err(|err| err.to_string())?;

    Ok(track)
}

#[tauri::command]
pub async fn get_albums(app_state: State<'_, AppState>) -> Result<Vec<PersistentAlbum>, String> {
    let conn_guard = app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;
    let albums = library::get_albums(conn).map_err(|err| err.to_string())?;

    Ok(albums)
}

#[tauri::command]
pub async fn get_album_ids(app_state: State<'_, AppState>) -> Result<Vec<i64>, String> {
    let conn_guard = app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;
    let album_ids = library::get_album_ids(conn).map_err(|err| err.to_string())?;

    Ok(album_ids)
}

#[tauri::command]
pub async fn get_album(
    album_id: i64,
    app_state: State<'_, AppState>,
) -> Result<PersistentAlbum, String> {
    let conn_guard = app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;
    let album = library::get_album(album_id, conn).map_err(|err| err.to_string())?;

    Ok(album)
}

#[tauri::command]
pub async fn get_artists(app_state: State<'_, AppState>) -> Result<Vec<PersistentArtist>, String> {
    let conn_guard = app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;
    let artists = library::get_artists(conn).map_err(|err| err.to_string())?;

    Ok(artists)
}

#[tauri::command]
pub async fn get_artist_ids(app_state: State<'_, AppState>) -> Result<Vec<i64>, String> {
    let conn_guard = app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;
    let artist_ids = library::get_artist_ids(conn).map_err(|err| err.to_string())?;

    Ok(artist_ids)
}

#[tauri::command]
pub async fn get_artist(
    artist_id: i64,
    app_state: State<'_, AppState>,
) -> Result<PersistentArtist, String> {
    let conn_guard = app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;
    let artist = library::get_artist(artist_id, conn).map_err(|err| err.to_string())?;

    Ok(artist)
}

#[tauri::command]
pub async fn get_album_tracks(
    album_id: i64,
    app_state: State<'_, AppState>,
) -> Result<Vec<PersistentTrack>, String> {
    let conn_guard = app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;
    let tracks = library::get_album_tracks(album_id, conn).map_err(|err| err.to_string())?;

    Ok(tracks)
}

#[tauri::command]
pub async fn get_artist_tracks(
    artist_id: i64,
    app_state: State<'_, AppState>,
) -> Result<Vec<PersistentTrack>, String> {
    let conn_guard = app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;
    let tracks = library::get_artist_tracks(artist_id, conn).map_err(|err| err.to_string())?;

    Ok(tracks)
}

#[tauri::command]
pub async fn get_album_track_ids(
    album_id: i64,
    without_plain_lyrics: Option<bool>,
    without_synced_lyrics: Option<bool>,
    app_state: State<'_, AppState>,
) -> Result<Vec<i64>, String> {
    let conn_guard = app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;
    let track_ids = library::get_album_track_ids(album_id, without_plain_lyrics.unwrap_or(false), without_synced_lyrics.unwrap_or(false), conn).map_err(|err| err.to_string())?;

    Ok(track_ids)
}

#[tauri::command]
pub async fn get_artist_track_ids(
    artist_id: i64,
    without_plain_lyrics: Option<bool>,
    without_synced_lyrics: Option<bool>,
    app_state: State<'_, AppState>,
) -> Result<Vec<i64>, String> {
    let conn_guard = app_state.db.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;
    let track_ids =
        library::get_artist_track_ids(artist_id, without_plain_lyrics.unwrap_or(false), without_synced_lyrics.unwrap_or(false), conn).map_err(|err| err.to_string())?;

    Ok(track_ids)
}
