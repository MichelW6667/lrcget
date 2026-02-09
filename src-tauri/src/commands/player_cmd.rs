use crate::db;
use crate::state::{AppState, ServiceAccess};
use tauri::AppHandle;

#[tauri::command]
pub fn play_track(
    track_id: i64,
    app_state: tauri::State<AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let track = app_handle
        .db(|db| db::get_track_by_id(track_id, db))
        .map_err(|err| err.to_string())?;

    let mut player_guard = app_state.player.lock().map_err(|e| e.to_string())?;

    if let Some(ref mut player) = *player_guard {
        player.play(track).map_err(|err| err.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn pause_track(app_state: tauri::State<AppState>) -> Result<(), String> {
    let mut player_guard = app_state.player.lock().map_err(|e| e.to_string())?;

    if let Some(ref mut player) = *player_guard {
        player.pause();
    }

    Ok(())
}

#[tauri::command]
pub fn resume_track(app_state: tauri::State<AppState>) -> Result<(), String> {
    let mut player_guard = app_state.player.lock().map_err(|e| e.to_string())?;

    if let Some(ref mut player) = *player_guard {
        player.resume();
    }

    Ok(())
}

#[tauri::command]
pub fn seek_track(position: f64, app_state: tauri::State<AppState>) -> Result<(), String> {
    let mut player_guard = app_state.player.lock().map_err(|e| e.to_string())?;

    if let Some(ref mut player) = *player_guard {
        player.seek(position);
    }

    Ok(())
}

#[tauri::command]
pub fn stop_track(app_state: tauri::State<AppState>) -> Result<(), String> {
    let mut player_guard = app_state.player.lock().map_err(|e| e.to_string())?;

    if let Some(ref mut player) = *player_guard {
        player.stop();
    }

    Ok(())
}

#[tauri::command]
pub fn set_volume(volume: f64, app_state: tauri::State<AppState>) -> Result<(), String> {
    let mut player_guard = app_state.player.lock().map_err(|e| e.to_string())?;

    if let Some(ref mut player) = *player_guard {
        player.set_volume(volume);
    }

    Ok(())
}
