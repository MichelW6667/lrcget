use crate::fs_track;
use crate::persistent_entities::{
    LibraryStats, PersistentAlbum, PersistentArtist, PersistentConfig, PersistentTrack,
};
use crate::utils::{prepare_input, RE_INSTRUMENTAL};
use anyhow::Result;
use indoc::indoc;
use rusqlite::{named_params, params, Connection};
use std::fs;
use tauri::{AppHandle, Manager};

const CURRENT_DB_VERSION: u32 = 13;

/// Initializes the database connection, creating the .sqlite file if needed, and upgrading the database
/// if it's out of date.
pub fn initialize_database(app_handle: &AppHandle) -> Result<Connection, rusqlite::Error> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .expect("The app data directory should exist.");
    fs::create_dir_all(&app_dir).expect("The app data directory should be created.");
    let sqlite_path = app_dir.join("db.sqlite3");

    println!("Database file path: {}", sqlite_path.display());

    let mut db = Connection::open(sqlite_path)?;

    let mut user_pragma = db.prepare("PRAGMA user_version")?;
    let existing_user_version: u32 = user_pragma.query_row([], |row| Ok(row.get(0)?))?;
    drop(user_pragma);

    upgrade_database_if_needed(&mut db, existing_user_version)?;

    Ok(db)
}

/// Upgrades the database to the current version.
pub fn upgrade_database_if_needed(
    db: &mut Connection,
    existing_version: u32,
) -> Result<(), rusqlite::Error> {
    println!("Existing database version: {}", existing_version);

    if existing_version < CURRENT_DB_VERSION {
        if existing_version <= 0 {
            println!("Migrate database version 1...");
            db.pragma_update(None, "journal_mode", "WAL")?;

            let tx = db.transaction()?;

            tx.pragma_update(None, "user_version", 1)?;

            tx.execute_batch(indoc! {"
            CREATE TABLE directories (
                id INTEGER PRIMARY KEY,
                path TEXT
            );

            CREATE TABLE library_data (
                id INTEGER PRIMARY KEY,
                init BOOLEAN
            );

            CREATE TABLE config_data (
                id INTEGER PRIMARY KEY,
                skip_not_needed_tracks BOOLEAN,
                try_embed_lyrics BOOLEAN
            );

            CREATE TABLE artists (
                id INTEGER PRIMARY KEY,
                name TEXT
            );

            CREATE TABLE albums (
                id INTEGER PRIMARY KEY,
                name TEXT,
                artist_id INTEGER,
                image_path TEXT,
                FOREIGN KEY(artist_id) REFERENCES artists(id)
            );

            CREATE TABLE tracks (
                id INTEGER PRIMARY KEY,
                file_path TEXT,
                file_name TEXT,
                title TEXT,
                album_id INTEGER,
                artist_id INTEGER,
                duration FLOAT,
                lrc_lyrics TEXT,
                FOREIGN KEY(artist_id) REFERENCES artists(id),
                FOREIGN KEY(album_id) REFERENCES albums(id)
            );

            INSERT INTO library_data (init) VALUES (0);
            INSERT INTO config_data (skip_not_needed_tracks, try_embed_lyrics) VALUES (1, 0);
            "})?;

            tx.commit()?;
        }

        if existing_version <= 1 {
            println!("Migrate database version 2...");
            db.pragma_update(None, "journal_mode", "WAL")?;

            let tx = db.transaction()?;

            tx.pragma_update(None, "user_version", 2)?;

            tx.execute_batch(indoc! {"
            ALTER TABLE tracks ADD txt_lyrics TEXT;
            CREATE INDEX idx_tracks_title ON tracks(title);
            CREATE INDEX idx_albums_name ON albums(name);
            CREATE INDEX idx_artists_name ON artists(name);
            "})?;
            tx.commit()?;
        }

        if existing_version <= 2 {
            println!("Migrate database version 3...");
            let tx = db.transaction()?;

            tx.pragma_update(None, "user_version", 3)?;

            tx.execute_batch(indoc! {"
            ALTER TABLE tracks ADD instrumental BOOLEAN;
            "})?;
            tx.commit()?;
        }

        if existing_version <= 3 {
            println!("Migrate database version 4...");
            let tx = db.transaction()?;

            tx.pragma_update(None, "user_version", 4)?;

            tx.execute_batch(indoc! {"
            ALTER TABLE tracks ADD title_lower TEXT;
            ALTER TABLE albums ADD name_lower TEXT;
            ALTER TABLE artists ADD name_lower TEXT;
            CREATE INDEX idx_tracks_title_lower ON tracks(title_lower);
            CREATE INDEX idx_albums_name_lower ON albums(name_lower);
            CREATE INDEX idx_artists_name_lower ON artists(name_lower);
            "})?;

            tx.commit()?;
        }

        if existing_version <= 4 {
            println!("Migrate database version 5...");
            let tx = db.transaction()?;

            tx.pragma_update(None, "user_version", 5)?;

            tx.execute_batch(indoc! {"
            ALTER TABLE tracks ADD track_number INTEGER;
            ALTER TABLE albums ADD album_artist_name TEXT;
            ALTER TABLE albums ADD album_artist_name_lower TEXT;
            ALTER TABLE config_data ADD theme_mode TEXT DEFAULT 'auto';
            ALTER TABLE config_data ADD lrclib_instance TEXT DEFAULT 'https://lrclib.net';
            CREATE INDEX idx_albums_album_artist_name_lower ON albums(album_artist_name_lower);
            CREATE INDEX idx_tracks_track_number ON tracks(track_number);

            DELETE FROM tracks WHERE 1;
            DELETE FROM albums WHERE 1;
            DELETE FROM artists WHERE 1;
            UPDATE library_data SET init = 0 WHERE 1;
            "})?;

            tx.commit()?;
        }

        if existing_version <= 5 {
            println!("Migrate database version 6...");
            let tx = db.transaction()?;

            tx.pragma_update(None, "user_version", 6)?;

            tx.execute_batch(indoc! {"
            ALTER TABLE config_data ADD skip_tracks_with_synced_lyrics BOOLEAN DEFAULT 0;
            ALTER TABLE config_data ADD skip_tracks_with_plain_lyrics BOOLEAN DEFAULT 0;
            UPDATE config_data SET skip_tracks_with_synced_lyrics = skip_not_needed_tracks;
            ALTER TABLE config_data DROP COLUMN skip_not_needed_tracks;
            "})?;

            tx.commit()?;
        }

        if existing_version <= 6 {
            println!("Migrate database version 7...");
            let tx = db.transaction()?;

            tx.pragma_update(None, "user_version", 7)?;

            tx.execute_batch(indoc! {"
            ALTER TABLE config_data ADD show_line_count BOOLEAN DEFAULT 1;
            "})?;

            tx.commit()?;
        }

        if existing_version <= 7 {
            println!("Migrate database version 8...");
            let tx = db.transaction()?;

            tx.pragma_update(None, "user_version", 8)?;

            tx.execute_batch(indoc! {"
            ALTER TABLE config_data ADD lyrics_type_preference TEXT DEFAULT 'both';
            "})?;

            tx.commit()?;
        }

        if existing_version <= 8 {
            println!("Migrate database version 9...");
            let tx = db.transaction()?;

            tx.pragma_update(None, "user_version", 9)?;

            tx.execute_batch(indoc! {"
            ALTER TABLE config_data ADD duration_tolerance REAL DEFAULT 3.0;
            "})?;

            tx.commit()?;
        }

        if existing_version <= 9 {
            println!("Migrate database version 10...");
            let tx = db.transaction()?;

            tx.pragma_update(None, "user_version", 10)?;

            tx.execute_batch(indoc! {"
            ALTER TABLE config_data ADD fuzzy_search_enabled BOOLEAN DEFAULT 1;
            "})?;

            tx.commit()?;
        }

        if existing_version <= 10 {
            println!("Migrate database version 11...");
            let tx = db.transaction()?;

            tx.pragma_update(None, "user_version", 11)?;

            tx.execute_batch(indoc! {"
            ALTER TABLE tracks ADD bitrate INTEGER;
            "})?;

            tx.commit()?;
        }

        if existing_version <= 11 {
            println!("Migrate database version 12...");
            let tx = db.transaction()?;

            tx.pragma_update(None, "user_version", 12)?;

            tx.execute_batch(indoc! {"
            CREATE INDEX IF NOT EXISTS idx_tracks_album_id ON tracks(album_id);
            CREATE INDEX IF NOT EXISTS idx_tracks_artist_id ON tracks(artist_id);
            CREATE INDEX IF NOT EXISTS idx_albums_artist_id ON albums(artist_id);
            "})?;

            tx.commit()?;
        }

        if existing_version <= 12 {
            println!("Migrate database version 13...");
            let tx = db.transaction()?;

            tx.pragma_update(None, "user_version", 13)?;

            tx.execute_batch(indoc! {"
                ALTER TABLE tracks ADD lyrics_status TEXT DEFAULT 'missing';
                UPDATE tracks SET lyrics_status = CASE
                    WHEN instrumental = 1 THEN 'instrumental'
                    WHEN lrc_lyrics IS NOT NULL AND lrc_lyrics != '[au: instrumental]' THEN 'synced'
                    WHEN txt_lyrics IS NOT NULL THEN 'plain'
                    ELSE 'missing'
                END;
                CREATE INDEX idx_tracks_lyrics_status ON tracks(lyrics_status);
            "})?;

            tx.commit()?;
        }
    }

    Ok(())
}

pub fn get_directories(db: &Connection) -> Result<Vec<String>> {
    let mut statement = db.prepare("SELECT * FROM directories")?;
    let mut rows = statement.query([])?;
    let mut directories: Vec<String> = Vec::new();
    while let Some(row) = rows.next()? {
        let path: String = row.get("path")?;

        directories.push(path);
    }

    Ok(directories)
}

pub fn set_directories(directories: Vec<String>, db: &Connection) -> Result<()> {
    db.execute("DELETE FROM directories WHERE 1", ())?;
    let mut statement = db.prepare("INSERT INTO directories (path) VALUES (@path)")?;
    for directory in directories.iter() {
        statement.execute(named_params! { "@path": directory })?;
    }

    Ok(())
}

pub fn get_init(db: &Connection) -> Result<bool> {
    let mut statement = db.prepare("SELECT init FROM library_data LIMIT 1")?;
    let init: bool = statement.query_row([], |r| r.get(0))?;
    Ok(init)
}

pub fn set_init(init: bool, db: &Connection) -> Result<()> {
    let mut statement = db.prepare("UPDATE library_data SET init = ? WHERE 1")?;
    statement.execute([init])?;
    Ok(())
}

pub fn get_config(db: &Connection) -> Result<PersistentConfig> {
    let mut statement = db.prepare(indoc! {"
      SELECT
        skip_tracks_with_synced_lyrics,
        skip_tracks_with_plain_lyrics,
        show_line_count,
        try_embed_lyrics,
        theme_mode,
        lrclib_instance,
        lyrics_type_preference,
        duration_tolerance,
        fuzzy_search_enabled
      FROM config_data
      LIMIT 1
    "})?;
    let row = statement.query_row([], |r| {
        Ok(PersistentConfig {
            skip_tracks_with_synced_lyrics: r.get("skip_tracks_with_synced_lyrics")?,
            skip_tracks_with_plain_lyrics: r.get("skip_tracks_with_plain_lyrics")?,
            show_line_count: r.get("show_line_count")?,
            try_embed_lyrics: r.get("try_embed_lyrics")?,
            theme_mode: r.get("theme_mode")?,
            lrclib_instance: r.get("lrclib_instance")?,
            lyrics_type_preference: r.get("lyrics_type_preference")?,
            duration_tolerance: r.get("duration_tolerance")?,
            fuzzy_search_enabled: r.get("fuzzy_search_enabled")?,
        })
    })?;
    Ok(row)
}

pub fn set_config(
    skip_tracks_with_synced_lyrics: bool,
    skip_tracks_with_plain_lyrics: bool,
    show_line_count: bool,
    try_embed_lyrics: bool,
    theme_mode: &str,
    lrclib_instance: &str,
    lyrics_type_preference: &str,
    duration_tolerance: f64,
    fuzzy_search_enabled: bool,
    db: &Connection,
) -> Result<()> {
    let mut statement = db.prepare(indoc! {"
      UPDATE config_data
      SET
        skip_tracks_with_synced_lyrics = ?,
        skip_tracks_with_plain_lyrics = ?,
        show_line_count = ?,
        try_embed_lyrics = ?,
        theme_mode = ?,
        lrclib_instance = ?,
        lyrics_type_preference = ?,
        duration_tolerance = ?,
        fuzzy_search_enabled = ?
      WHERE 1
    "})?;
    statement.execute((
        skip_tracks_with_synced_lyrics,
        skip_tracks_with_plain_lyrics,
        show_line_count,
        try_embed_lyrics,
        theme_mode,
        lrclib_instance,
        lyrics_type_preference,
        duration_tolerance,
        fuzzy_search_enabled,
    ))?;
    Ok(())
}

fn get_order_clause(sort_by: &str, sort_order: &str) -> String {
    let column = match sort_by {
        "title" => "title_lower",
        "duration" => "duration",
        "track_number" => "track_number",
        "lyrics_status" => "CASE WHEN lrc_lyrics IS NOT NULL AND lrc_lyrics != '[au: instrumental]' THEN 0 WHEN txt_lyrics IS NOT NULL THEN 1 WHEN instrumental = 1 THEN 2 ELSE 3 END",
        _ => "title_lower",
    };
    let direction = if sort_order == "desc" { "DESC" } else { "ASC" };
    format!("ORDER BY {} {}", column, direction)
}

pub fn get_library_stats(db: &Connection) -> Result<LibraryStats> {
    let mut statement = db.prepare(indoc! {"
      SELECT
        COUNT(*) as total,
        SUM(CASE WHEN lyrics_status = 'instrumental' THEN 1 ELSE 0 END) as instrumental,
        SUM(CASE WHEN lyrics_status = 'synced' THEN 1 ELSE 0 END) as synced,
        SUM(CASE WHEN lyrics_status = 'plain' THEN 1 ELSE 0 END) as plain_only,
        SUM(CASE WHEN lyrics_status = 'missing' THEN 1 ELSE 0 END) as missing
      FROM tracks
    "})?;
    let row = statement.query_row([], |r| {
        Ok(LibraryStats {
            total: r.get("total")?,
            instrumental: r.get::<_, Option<i64>>("instrumental")?.unwrap_or(0),
            synced: r.get::<_, Option<i64>>("synced")?.unwrap_or(0),
            plain_only: r.get::<_, Option<i64>>("plain_only")?.unwrap_or(0),
            missing: r.get::<_, Option<i64>>("missing")?.unwrap_or(0),
        })
    })?;
    Ok(row)
}

pub fn find_artist(name: &str, db: &Connection) -> Result<i64> {
    let mut statement = db.prepare("SELECT id FROM artists WHERE name = ?")?;
    let id: i64 = statement.query_row([name], |r| r.get(0))?;
    Ok(id)
}

pub fn add_artist(name: &str, db: &Connection) -> Result<i64> {
    let mut statement = db.prepare("INSERT INTO artists (name, name_lower) VALUES (?, ?)")?;
    let row_id = statement.insert((name, prepare_input(name)))?;
    Ok(row_id)
}

pub fn find_album(name: &str, album_artist_name: &str, db: &Connection) -> Result<i64> {
    let mut statement =
        db.prepare("SELECT id FROM albums WHERE name = ? AND album_artist_name = ?")?;
    let id: i64 = statement.query_row((name, album_artist_name), |r| r.get(0))?;
    Ok(id)
}

pub fn add_album(name: &str, album_artist_name: &str, db: &Connection) -> Result<i64> {
    let mut statement = db.prepare("INSERT INTO albums (name, name_lower, album_artist_name, album_artist_name_lower) VALUES (?, ?, ?, ?)")?;
    let row_id = statement.insert((
        name,
        prepare_input(name),
        album_artist_name,
        prepare_input(album_artist_name),
    ))?;
    Ok(row_id)
}

pub fn get_track_by_id(id: i64, db: &Connection) -> Result<PersistentTrack> {
    let query = indoc! {"
    SELECT
      tracks.id,
      file_path,
      file_name,
      title,
      artists.name AS artist_name,
      tracks.artist_id,
      albums.name AS album_name,
      albums.album_artist_name,
      album_id,
      duration,
      track_number,
      albums.image_path,
      txt_lyrics,
      lrc_lyrics,
      instrumental,
      bitrate
    FROM tracks
    JOIN albums ON tracks.album_id = albums.id
    JOIN artists ON tracks.artist_id = artists.id
    WHERE tracks.id = ?
    LIMIT 1
  "};

    let mut statement = db.prepare(query)?;
    let row = statement.query_row([id], |row| {
        let is_instrumental: Option<bool> = row.get("instrumental")?;

        Ok(PersistentTrack {
            id: row.get("id")?,
            file_path: row.get("file_path")?,
            file_name: row.get("file_name")?,
            title: row.get("title")?,
            artist_name: row.get("artist_name")?,
            artist_id: row.get("artist_id")?,
            album_name: row.get("album_name")?,
            album_artist_name: row.get("album_artist_name")?,
            album_id: row.get("album_id")?,
            duration: row.get("duration")?,
            track_number: row.get("track_number")?,
            txt_lyrics: row.get("txt_lyrics")?,
            lrc_lyrics: row.get("lrc_lyrics")?,
            image_path: row.get("image_path")?,
            instrumental: is_instrumental.unwrap_or(false),
            bitrate: row.get("bitrate")?,
        })
    })?;
    Ok(row)
}

pub fn update_track_synced_lyrics(
    id: i64,
    synced_lyrics: &str,
    plain_lyrics: &str,
    db: &Connection,
) -> Result<PersistentTrack> {
    let mut statement = db.prepare(
        "UPDATE tracks SET lrc_lyrics = ?, txt_lyrics = ?, instrumental = false, lyrics_status = 'synced' WHERE id = ?",
    )?;
    statement.execute((synced_lyrics, plain_lyrics, id))?;

    Ok(get_track_by_id(id, db)?)
}

pub fn update_track_plain_lyrics(
    id: i64,
    plain_lyrics: &str,
    db: &Connection,
) -> Result<PersistentTrack> {
    let mut statement = db.prepare(
        "UPDATE tracks SET txt_lyrics = ?, lrc_lyrics = null, instrumental = false, lyrics_status = 'plain' WHERE id = ?",
    )?;
    statement.execute((plain_lyrics, id))?;

    Ok(get_track_by_id(id, db)?)
}

pub fn update_track_null_lyrics(id: i64, db: &Connection) -> Result<PersistentTrack> {
    let mut statement = db.prepare(
        "UPDATE tracks SET txt_lyrics = null, lrc_lyrics = null, instrumental = false, lyrics_status = 'missing' WHERE id = ?",
    )?;
    statement.execute([id])?;

    Ok(get_track_by_id(id, db)?)
}

pub fn update_track_instrumental(id: i64, db: &Connection) -> Result<PersistentTrack> {
    let mut statement = db.prepare(
        "UPDATE tracks SET txt_lyrics = null, lrc_lyrics = ?, instrumental = true, lyrics_status = 'instrumental' WHERE id = ?",
    )?;
    statement.execute(params!["[au: instrumental]", id])?;

    Ok(get_track_by_id(id, db)?)
}

pub fn add_tracks(
    tracks: &Vec<fs_track::FsTrack>,
    db: &mut Connection,
    artist_cache: &mut std::collections::HashMap<String, i64>,
    album_cache: &mut std::collections::HashMap<(String, String), i64>,
) -> Result<()> {
    let tx = db.transaction()?;

    // Prepare statement once, reuse for all tracks in the batch
    let mut insert_stmt = tx.prepare(indoc! {"
        INSERT INTO tracks (
            file_path, file_name, title, title_lower, album_id, artist_id,
            duration, track_number, txt_lyrics, lrc_lyrics, instrumental, bitrate, lyrics_status
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    "})?;

    for track in tracks.iter() {
        let artist_key = track.artist().to_owned();
        let artist_id = if let Some(&id) = artist_cache.get(&artist_key) {
            id
        } else {
            let id = match find_artist(track.artist(), &tx) {
                Ok(id) => id,
                Err(_) => add_artist(track.artist(), &tx)?,
            };
            artist_cache.insert(artist_key, id);
            id
        };

        let album_key = (track.album().to_owned(), track.album_artist().to_owned());
        let album_id = if let Some(&id) = album_cache.get(&album_key) {
            id
        } else {
            let id = match find_album(track.album(), track.album_artist(), &tx) {
                Ok(id) => id,
                Err(_) => add_album(track.album(), track.album_artist(), &tx)?,
            };
            album_cache.insert(album_key, id);
            id
        };

        let is_instrumental = track
            .lrc_lyrics()
            .map_or(false, |lyrics| RE_INSTRUMENTAL.is_match(lyrics));

        let lyrics_status = if is_instrumental {
            "instrumental"
        } else if track.lrc_lyrics().is_some() {
            "synced"
        } else if track.txt_lyrics().is_some() {
            "plain"
        } else {
            "missing"
        };

        insert_stmt.execute((
            track.file_path(),
            track.file_name(),
            track.title(),
            prepare_input(track.title()),
            album_id,
            artist_id,
            track.duration(),
            track.track_number(),
            track.txt_lyrics(),
            track.lrc_lyrics(),
            is_instrumental,
            track.bitrate(),
            lyrics_status,
        ))?;
    }

    drop(insert_stmt);
    tx.commit()?;

    Ok(())
}

pub fn get_tracks(db: &Connection) -> Result<Vec<PersistentTrack>> {
    let query = indoc! {"
      SELECT
          tracks.id, file_path, file_name, title,
          artists.name AS artist_name, tracks.artist_id,
          albums.name AS album_name, albums.album_artist_name, album_id, duration, track_number,
          albums.image_path, txt_lyrics, lrc_lyrics, instrumental, bitrate
      FROM tracks
      JOIN albums ON tracks.album_id = albums.id
      JOIN artists ON tracks.artist_id = artists.id
      ORDER BY title_lower ASC
  "};
    let mut statement = db.prepare(query)?;
    let mut rows = statement.query([])?;
    let mut tracks: Vec<PersistentTrack> = Vec::new();

    while let Some(row) = rows.next()? {
        let is_instrumental: Option<bool> = row.get("instrumental")?;

        let track = PersistentTrack {
            id: row.get("id")?,
            file_path: row.get("file_path")?,
            file_name: row.get("file_name")?,
            title: row.get("title")?,
            artist_name: row.get("artist_name")?,
            artist_id: row.get("artist_id")?,
            album_name: row.get("album_name")?,
            album_artist_name: row.get("album_artist_name")?,
            album_id: row.get("album_id")?,
            duration: row.get("duration")?,
            track_number: row.get("track_number")?,
            txt_lyrics: row.get("txt_lyrics")?,
            lrc_lyrics: row.get("lrc_lyrics")?,
            image_path: row.get("image_path")?,
            instrumental: is_instrumental.unwrap_or(false),
            bitrate: row.get("bitrate")?,
        };

        tracks.push(track);
    }

    Ok(tracks)
}

pub fn get_track_ids(
    synced_lyrics: bool,
    plain_lyrics: bool,
    instrumental: bool,
    no_lyrics: bool,
    sort_by: &str,
    sort_order: &str,
    db: &Connection
) -> Result<Vec<i64>> {
    let base_query = "SELECT id FROM tracks";

    let mut excluded = Vec::new();
    if !synced_lyrics { excluded.push("'synced'"); }
    if !plain_lyrics { excluded.push("'plain'"); }
    if !instrumental { excluded.push("'instrumental'"); }
    if !no_lyrics { excluded.push("'missing'"); }

    let where_clause = if !excluded.is_empty() {
        format!(" WHERE lyrics_status NOT IN ({})", excluded.join(", "))
    } else {
        String::new()
    };

    let order = get_order_clause(sort_by, sort_order);
    let full_query = format!("{}{} {}", base_query, where_clause, order);

    let mut statement = db.prepare(&full_query)?;
    let mut rows = statement.query([])?;
    let mut track_ids: Vec<i64> = Vec::new();

    while let Some(row) = rows.next()? {
        track_ids.push(row.get("id")?);
    }

    Ok(track_ids)
}

pub fn get_search_track_ids(
    query_str: &String,
    synced_lyrics: bool,
    plain_lyrics: bool,
    instrumental: bool,
    no_lyrics: bool,
    sort_by: &str,
    sort_order: &str,
    db: &Connection
) -> Result<Vec<i64>> {
    let base_query = indoc! {"
      SELECT tracks.id
      FROM tracks
      JOIN artists ON tracks.artist_id = artists.id
      JOIN albums ON tracks.album_id = albums.id
      WHERE (artists.name_lower LIKE ?
      OR albums.name_lower LIKE ?
      OR tracks.title_lower LIKE ?)
    "};

    let mut excluded = Vec::new();
    if !synced_lyrics { excluded.push("'synced'"); }
    if !plain_lyrics { excluded.push("'plain'"); }
    if !instrumental { excluded.push("'instrumental'"); }
    if !no_lyrics { excluded.push("'missing'"); }

    let where_clause = if !excluded.is_empty() {
        format!(" AND tracks.lyrics_status NOT IN ({})", excluded.join(", "))
    } else {
        String::new()
    };

    let order = get_order_clause(sort_by, sort_order);
    let full_query = format!("{}{} {}", base_query, where_clause, order);

    let mut statement = db.prepare(&full_query)?;
    let formatted_query_str = format!("%{}%", prepare_input(query_str));
    let mut rows = statement.query(params![
        formatted_query_str,
        formatted_query_str,
        formatted_query_str
    ])?;
    let mut track_ids: Vec<i64> = Vec::new();

    while let Some(row) = rows.next()? {
        track_ids.push(row.get("id")?);
    }

    Ok(track_ids)
}

pub fn get_albums(db: &Connection) -> Result<Vec<PersistentAlbum>> {
    let mut statement = db.prepare(indoc! {"
      SELECT albums.id, albums.name, albums.album_artist_name AS album_artist_name, albums.album_artist_name,
          albums.image_path, COUNT(tracks.id) AS tracks_count
      FROM albums
      JOIN tracks ON tracks.album_id = albums.id
      GROUP BY albums.id, albums.name, albums.album_artist_name
      ORDER BY albums.name_lower ASC
  "})?;
    let mut rows = statement.query([])?;
    let mut albums: Vec<PersistentAlbum> = Vec::new();

    while let Some(row) = rows.next()? {
        let album = PersistentAlbum {
            id: row.get("id")?,
            name: row.get("name")?,
            image_path: row.get("image_path")?,
            artist_name: row.get("album_artist_name")?,
            album_artist_name: row.get("album_artist_name")?,
            tracks_count: row.get("tracks_count")?,
        };

        albums.push(album);
    }

    Ok(albums)
}

pub fn get_album_by_id(id: i64, db: &Connection) -> Result<PersistentAlbum> {
    let mut statement = db.prepare(indoc! {"
    SELECT
      albums.id,
      albums.name,
      albums.album_artist_name,
      COUNT(tracks.id) AS tracks_count
    FROM albums
    JOIN tracks ON tracks.album_id = albums.id
    WHERE albums.id = ?
    GROUP BY
      albums.id,
      albums.name,
      albums.album_artist_name
    LIMIT 1
  "})?;
    let row = statement.query_row([id], |row| {
        Ok(PersistentAlbum {
            id: row.get("id")?,
            name: row.get("name")?,
            image_path: None,
            artist_name: row.get("album_artist_name")?,
            album_artist_name: row.get("album_artist_name")?,
            tracks_count: row.get("tracks_count")?,
        })
    })?;
    Ok(row)
}

pub fn get_album_ids(search_query: Option<&str>, db: &Connection) -> Result<Vec<i64>> {
    let album_ids = match search_query {
        Some(query) => {
            let like_query = format!("%{}%", prepare_input(query));
            let mut statement = db.prepare(
                "SELECT id FROM albums WHERE name_lower LIKE ?1 OR album_artist_name_lower LIKE ?1 ORDER BY name_lower ASC"
            )?;
            let mut rows = statement.query([&like_query])?;
            let mut ids: Vec<i64> = Vec::new();
            while let Some(row) = rows.next()? {
                ids.push(row.get("id")?);
            }
            ids
        }
        None => {
            let mut statement = db.prepare("SELECT id FROM albums ORDER BY name_lower ASC")?;
            let mut rows = statement.query([])?;
            let mut ids: Vec<i64> = Vec::new();
            while let Some(row) = rows.next()? {
                ids.push(row.get("id")?);
            }
            ids
        }
    };
    Ok(album_ids)
}

pub fn get_artists(db: &Connection) -> Result<Vec<PersistentArtist>> {
    let mut statement = db.prepare(indoc! {"
    SELECT artists.id, artists.name AS name, COUNT(tracks.id) AS tracks_count
    FROM artists
    JOIN tracks ON tracks.artist_id = artists.id
    GROUP BY artists.id, artists.name
    ORDER BY artists.name_lower ASC
  "})?;
    let mut rows = statement.query([])?;
    let mut artists: Vec<PersistentArtist> = Vec::new();

    while let Some(row) = rows.next()? {
        let artist = PersistentArtist {
            id: row.get("id")?,
            name: row.get("name")?,
            // albums_count: row.get("albums_count")?,
            tracks_count: row.get("tracks_count")?,
        };

        artists.push(artist);
    }

    Ok(artists)
}

pub fn get_artist_by_id(id: i64, db: &Connection) -> Result<PersistentArtist> {
    let mut statement = db.prepare(indoc! {"
    SELECT artists.id,
      artists.name AS name,
      COUNT(tracks.id) AS tracks_count
    FROM artists
    JOIN tracks ON tracks.artist_id = artists.id
    WHERE artists.id = ?
    GROUP BY artists.id, artists.name
    LIMIT 1
  "})?;
    let row = statement.query_row([id], |row| {
        Ok(PersistentArtist {
            id: row.get("id")?,
            name: row.get("name")?,
            // albums_count: row.get("albums_count")?,
            tracks_count: row.get("tracks_count")?,
        })
    })?;
    Ok(row)
}

pub fn get_artist_ids(search_query: Option<&str>, db: &Connection) -> Result<Vec<i64>> {
    let artist_ids = match search_query {
        Some(query) => {
            let like_query = format!("%{}%", prepare_input(query));
            let mut statement = db.prepare(
                "SELECT id FROM artists WHERE name_lower LIKE ?1 ORDER BY name_lower ASC"
            )?;
            let mut rows = statement.query([&like_query])?;
            let mut ids: Vec<i64> = Vec::new();
            while let Some(row) = rows.next()? {
                ids.push(row.get("id")?);
            }
            ids
        }
        None => {
            let mut statement = db.prepare("SELECT id FROM artists ORDER BY name_lower ASC")?;
            let mut rows = statement.query([])?;
            let mut ids: Vec<i64> = Vec::new();
            while let Some(row) = rows.next()? {
                ids.push(row.get("id")?);
            }
            ids
        }
    };
    Ok(artist_ids)
}

pub fn get_album_tracks(album_id: i64, db: &Connection) -> Result<Vec<PersistentTrack>> {
    let mut statement = db.prepare(indoc! {"
    SELECT
      tracks.id,
      file_path,
      file_name,
      title,
      artists.name AS artist_name,
      tracks.artist_id,
      albums.name AS album_name,
      albums.album_artist_name,
      album_id,
      duration,
      track_number,
      albums.image_path,
      txt_lyrics,
      lrc_lyrics,
      instrumental,
      bitrate
    FROM tracks
    JOIN albums ON tracks.album_id = albums.id
    JOIN artists ON tracks.artist_id = artists.id
    WHERE tracks.album_id = ?
    ORDER BY track_number ASC
  "})?;
    let mut rows = statement.query([album_id])?;
    let mut tracks: Vec<PersistentTrack> = Vec::new();

    while let Some(row) = rows.next()? {
        let is_instrumental: Option<bool> = row.get("instrumental")?;

        let track = PersistentTrack {
            id: row.get("id")?,
            file_path: row.get("file_path")?,
            file_name: row.get("file_name")?,
            title: row.get("title")?,
            artist_name: row.get("artist_name")?,
            album_artist_name: row.get("album_artist_name")?,
            album_name: row.get("album_name")?,
            album_id: row.get("album_id")?,
            artist_id: row.get("artist_id")?,
            duration: row.get("duration")?,
            track_number: row.get("track_number")?,
            txt_lyrics: row.get("txt_lyrics")?,
            lrc_lyrics: row.get("lrc_lyrics")?,
            image_path: row.get("image_path")?,
            instrumental: is_instrumental.unwrap_or(false),
            bitrate: row.get("bitrate")?,
        };

        tracks.push(track);
    }

    Ok(tracks)
}

pub fn get_album_track_ids(album_id: i64, without_plain_lyrics: bool, without_synced_lyrics: bool, sort_by: &str, sort_order: &str, db: &Connection) -> Result<Vec<i64>> {
    let base_query = indoc! {"
      SELECT tracks.id
      FROM tracks
      JOIN albums ON tracks.album_id = albums.id
      WHERE tracks.album_id = ?"};

    // without_plain = only tracks without txt_lyrics (= 'missing', since synced always has txt)
    // without_synced = only tracks without lrc_lyrics (= 'missing' + 'plain')
    let lyrics_conditions = match (without_plain_lyrics, without_synced_lyrics) {
        (true, true) => " AND tracks.lyrics_status = 'missing'",
        (true, false) => " AND tracks.lyrics_status = 'missing'",
        (false, true) => " AND tracks.lyrics_status IN ('missing', 'plain')",
        (false, false) => "",
    };

    let order = get_order_clause(sort_by, sort_order);
    let full_query = format!("{}{} {}",
        base_query, lyrics_conditions, order);

    let mut statement = db.prepare(&full_query)?;
    let mut rows = statement.query([album_id])?;
    let mut tracks: Vec<i64> = Vec::new();

    while let Some(row) = rows.next()? {
        tracks.push(row.get("id")?);
    }

    Ok(tracks)
}

pub fn get_artist_tracks(artist_id: i64, db: &Connection) -> Result<Vec<PersistentTrack>> {
    let mut statement = db.prepare(indoc! {"
      SELECT tracks.id, file_path, file_name, title, artists.name AS artist_name,
        tracks.artist_id, albums.name AS album_name, albums.album_artist_name, album_id, duration, track_number,
        albums.image_path, txt_lyrics, lrc_lyrics, instrumental, bitrate
      FROM tracks
      JOIN albums ON tracks.album_id = albums.id
      JOIN artists ON tracks.artist_id = artists.id
      WHERE tracks.artist_id = ?
      ORDER BY album_name_lower ASC, track_number ASC
  "})?;
    let mut rows = statement.query([artist_id])?;
    let mut tracks: Vec<PersistentTrack> = Vec::new();

    while let Some(row) = rows.next()? {
        let is_instrumental: Option<bool> = row.get("instrumental")?;

        let track = PersistentTrack {
            id: row.get("id")?,
            file_path: row.get("file_path")?,
            file_name: row.get("file_name")?,
            title: row.get("title")?,
            artist_name: row.get("artist_name")?,
            artist_id: row.get("artist_id")?,
            album_name: row.get("album_name")?,
            album_artist_name: row.get("album_artist_name")?,
            album_id: row.get("album_id")?,
            duration: row.get("duration")?,
            track_number: row.get("track_number")?,
            txt_lyrics: row.get("txt_lyrics")?,
            lrc_lyrics: row.get("lrc_lyrics")?,
            image_path: row.get("image_path")?,
            instrumental: is_instrumental.unwrap_or(false),
            bitrate: row.get("bitrate")?,
        };

        tracks.push(track);
    }

    Ok(tracks)
}

pub fn get_artist_track_ids(artist_id: i64, without_plain_lyrics: bool, without_synced_lyrics: bool, sort_by: &str, sort_order: &str, db: &Connection) -> Result<Vec<i64>> {
    let base_query = indoc! {"
      SELECT tracks.id
      FROM tracks
      JOIN albums ON tracks.album_id = albums.id
      JOIN artists ON tracks.artist_id = artists.id
      WHERE tracks.artist_id = ?"};

    let lyrics_conditions = match (without_plain_lyrics, without_synced_lyrics) {
        (true, true) => " AND tracks.lyrics_status = 'missing'",
        (true, false) => " AND tracks.lyrics_status = 'missing'",
        (false, true) => " AND tracks.lyrics_status IN ('missing', 'plain')",
        (false, false) => "",
    };

    let order = get_order_clause(sort_by, sort_order);
    let full_query = format!("{}{} {}",
        base_query, lyrics_conditions, order);

    let mut statement = db.prepare(&full_query)?;
    let mut rows = statement.query([artist_id])?;
    let mut tracks: Vec<i64> = Vec::new();

    while let Some(row) = rows.next()? {
        tracks.push(row.get("id")?);
    }

    Ok(tracks)
}

pub fn clean_library(db: &Connection) -> Result<()> {
    db.execute("DELETE FROM tracks WHERE 1", ())?;
    db.execute("DELETE FROM albums WHERE 1", ())?;
    db.execute("DELETE FROM artists WHERE 1", ())?;
    Ok(())
}

pub fn get_existing_file_paths(db: &Connection) -> Result<std::collections::HashSet<String>> {
    let mut statement = db.prepare("SELECT file_path FROM tracks")?;
    let mut rows = statement.query([])?;
    let mut paths = std::collections::HashSet::new();
    while let Some(row) = rows.next()? {
        paths.insert(row.get(0)?);
    }
    Ok(paths)
}

pub fn delete_tracks_not_in(file_paths: &std::collections::HashSet<String>, db: &Connection) -> Result<usize> {
    let all_db_paths = get_existing_file_paths(db)?;
    let to_delete: Vec<&String> = all_db_paths.iter().filter(|p| !file_paths.contains(*p)).collect();
    let count = to_delete.len();

    if count > 0 {
        for chunk in to_delete.chunks(500) {
            let placeholders: Vec<&str> = chunk.iter().map(|_| "?").collect();
            let query = format!("DELETE FROM tracks WHERE file_path IN ({})", placeholders.join(", "));
            let mut stmt = db.prepare(&query)?;
            let params: Vec<&dyn rusqlite::types::ToSql> = chunk.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
            stmt.execute(params.as_slice())?;
        }
    }

    Ok(count)
}

pub fn delete_orphan_albums(db: &Connection) -> Result<usize> {
    let count = db.execute(
        "DELETE FROM albums WHERE id NOT IN (SELECT DISTINCT album_id FROM tracks)",
        (),
    )?;
    Ok(count)
}

pub fn delete_orphan_artists(db: &Connection) -> Result<usize> {
    let count = db.execute(
        "DELETE FROM artists WHERE id NOT IN (SELECT DISTINCT artist_id FROM tracks)",
        (),
    )?;
    Ok(count)
}
