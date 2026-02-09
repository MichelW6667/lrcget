#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

pub mod commands;
pub mod db;
pub mod fs_track;
pub mod library;
pub mod lrclib;
pub mod lyrics;
pub mod persistent_entities;
pub mod player;
pub mod state;
pub mod utils;

use commands::{library_cmd, lyrics_cmd, player_cmd};
use player::Player;
use state::{AppState, Notify, NotifyType};
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
fn open_devtools(app_handle: AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        window.open_devtools();
    }
}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_os::init())
        .manage(AppState {
            db: Default::default(),
            player: Default::default(),
        })
        .setup(|app| {
            let handle = app.handle();

            let app_state: State<AppState> = handle.state();
            let db = db::initialize_database(&handle).expect("Database initialize should succeed");
            *app_state.db.lock().expect("Database mutex poisoned during setup") = Some(db);

            let maybe_player = Player::new();
            match maybe_player {
                Ok(player) => {
                    *app_state.player.lock().expect("Player mutex poisoned during setup") = Some(player);
                }
                Err(e) => {
                    eprintln!("Failed to initialize audio player: {}", e);
                    let handle_for_notify = handle.clone();
                    let msg = format!("Failed to initialize audio player: {}", e);
                    tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        let _ = handle_for_notify.emit("app-notification", Notify {
                            message: msg,
                            notify_type: NotifyType::Error,
                        });
                    });
                }
            }

            let handle_clone = handle.clone();

            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_millis(40));
                loop {
                    interval.tick().await;
                    {
                        let app_state: State<AppState> = handle_clone.state();
                        let player_guard = app_state.player.lock();

                        match player_guard {
                            Ok(mut player_guard) => {
                                if let Some(ref mut player) = *player_guard {
                                    player.renew_state();

                                    let emit_player_state =
                                        handle_clone.emit("player-state", &player);

                                    if let Err(e) = emit_player_state {
                                        eprintln!("Failed to emit player state: {}", e);
                                    }
                                }
                            }
                            Err(e) => eprintln!("Failed to lock player: {}", e),
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            library_cmd::get_directories,
            library_cmd::set_directories,
            library_cmd::get_init,
            library_cmd::get_config,
            library_cmd::set_config,
            library_cmd::initialize_library,
            library_cmd::uninitialize_library,
            library_cmd::refresh_library,
            library_cmd::get_tracks,
            library_cmd::get_track_ids,
            library_cmd::get_track,
            library_cmd::get_albums,
            library_cmd::get_album_ids,
            library_cmd::get_album,
            library_cmd::get_artists,
            library_cmd::get_artist_ids,
            library_cmd::get_artist,
            library_cmd::get_album_tracks,
            library_cmd::get_artist_tracks,
            library_cmd::get_album_track_ids,
            library_cmd::get_artist_track_ids,
            library_cmd::get_library_stats,
            lyrics_cmd::download_lyrics,
            lyrics_cmd::apply_lyrics,
            lyrics_cmd::retrieve_lyrics,
            lyrics_cmd::retrieve_lyrics_by_id,
            lyrics_cmd::search_lyrics,
            lyrics_cmd::save_lyrics,
            lyrics_cmd::publish_lyrics,
            lyrics_cmd::flag_lyrics,
            player_cmd::play_track,
            player_cmd::pause_track,
            player_cmd::resume_track,
            player_cmd::seek_track,
            player_cmd::stop_track,
            player_cmd::set_volume,
            open_devtools,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
