use crate::db;
use anyhow::Result;
use globwalk::{glob, DirEntry};
use id3::TagLike;
use lofty::config::{ParseOptions, ParsingMode};
use lofty::error::LoftyError;
use lofty::file::AudioFile;
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use lofty::tag::Accessor;
use rayon::prelude::*;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use thiserror::Error;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FsTrack {
    file_path: String,
    file_name: String,
    title: String,
    album: String,
    artist: String,
    album_artist: String,
    duration: f64,
    txt_lyrics: Option<String>,
    lrc_lyrics: Option<String>,
    track_number: Option<u32>,
    bitrate: Option<u32>,
}

#[derive(Error, Debug)]
pub enum FsTrackError {
    #[error("Cannot parse the tag info from track: `{0}`. Error: `{1}`")]
    ParseFailed(String, LoftyError),
    #[error("No title was found from track: `{0}`")]
    TitleNotFound(String),
    #[error("No album name was found from track: `{0}`")]
    AlbumNotFound(String),
    #[error("No artist name was found from track: `{0}`")]
    ArtistNotFound(String),
    #[error("No primary tag was found from track: `{0}`")]
    PrimaryTagNotFound(String),
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanProgress {
    progress: Option<f64>,
    files_scanned: usize,
    files_count: Option<usize>,
}

impl FsTrack {
    fn new(
        file_path: String,
        file_name: String,
        title: String,
        album: String,
        artist: String,
        album_artist: String,
        duration: f64,
        txt_lyrics: Option<String>,
        lrc_lyrics: Option<String>,
        track_number: Option<u32>,
        bitrate: Option<u32>,
    ) -> FsTrack {
        FsTrack {
            file_path,
            file_name,
            title,
            album,
            artist,
            album_artist,
            duration,
            txt_lyrics,
            lrc_lyrics,
            track_number,
            bitrate,
        }
    }

    fn new_from_path(path: &Path) -> Result<FsTrack> {
        let file_path = path.display().to_string();
        let file_name = path.file_name().unwrap().to_str().unwrap().to_owned();

        // Skip cover art reading to save memory and I/O
        let opts = ParseOptions::new().read_cover_art(false);
        match Probe::open(&file_path).and_then(|p| p.options(opts).read()) {
            Ok(tagged_file) => {
                Self::from_lofty_tagged_file(tagged_file, file_path, file_name, path)
            }
            Err(lofty_err) => {
                // Fallback: lofty failed (often due to corrupt APE tags alongside valid ID3v2).
                // Use id3 crate for tags, lofty with read_tags(false) for audio properties.
                println!(
                    "Warning: lofty failed for `{}`: {}. Trying id3 fallback...",
                    file_path, lofty_err
                );
                Self::from_id3_fallback(path, &file_path, &file_name, lofty_err)
            }
        }
    }

    fn from_lofty_tagged_file(
        tagged_file: lofty::file::TaggedFile,
        file_path: String,
        file_name: String,
        _path: &Path,
    ) -> Result<FsTrack> {
        let tag = tagged_file
            .primary_tag()
            .ok_or(FsTrackError::PrimaryTagNotFound(file_path.to_owned()))?
            .to_owned();
        let properties = tagged_file.properties();
        let title = tag
            .title()
            .ok_or(FsTrackError::TitleNotFound(file_path.to_owned()))?
            .to_string();
        let album = tag
            .album()
            .ok_or(FsTrackError::AlbumNotFound(file_path.to_owned()))?
            .to_string();
        let artist = tag
            .artist()
            .ok_or(FsTrackError::ArtistNotFound(file_path.to_owned()))?
            .to_string();
        let album_artist = tag
            .get_string(&lofty::tag::ItemKey::AlbumArtist)
            .map(|s| s.to_string())
            .unwrap_or_else(|| artist.clone());
        let duration = properties.duration().as_secs_f64();
        let track_number = tag.track();
        let bitrate = properties.audio_bitrate();

        let mut track = FsTrack::new(
            file_path, file_name, title, album, artist, album_artist, duration, None, None,
            track_number, bitrate,
        );
        let (txt, lrc) = track.read_sidecar_lyrics();
        track.txt_lyrics = txt;
        track.lrc_lyrics = lrc;

        Ok(track)
    }

    fn from_id3_fallback(
        path: &Path,
        file_path: &str,
        file_name: &str,
        lofty_err: LoftyError,
    ) -> Result<FsTrack> {
        // Read ID3v2 tags via the id3 crate (ignores APE tags entirely)
        let id3_tag = id3::Tag::read_from_path(path)
            .map_err(|_| FsTrackError::ParseFailed(file_path.to_owned(), lofty_err))?;

        let title = id3_tag
            .title()
            .ok_or(FsTrackError::TitleNotFound(file_path.to_owned()))?
            .to_string();
        let album = id3_tag
            .album()
            .ok_or(FsTrackError::AlbumNotFound(file_path.to_owned()))?
            .to_string();
        let artist = id3_tag
            .artist()
            .ok_or(FsTrackError::ArtistNotFound(file_path.to_owned()))?
            .to_string();
        let album_artist = id3_tag
            .album_artist()
            .map(|s: &str| s.to_string())
            .unwrap_or_else(|| artist.clone());
        let track_number = id3_tag.track();

        // Try lofty with tags disabled to get audio properties (duration, bitrate)
        let (duration, bitrate) = Probe::open(file_path)
            .and_then(|probe| {
                probe
                    .options(ParseOptions::new().read_tags(false).parsing_mode(ParsingMode::Relaxed))
                    .read()
            })
            .map(|f| {
                let props = f.properties();
                (props.duration().as_secs_f64(), props.audio_bitrate())
            })
            .unwrap_or((0.0, None));

        let mut track = FsTrack::new(
            file_path.to_owned(),
            file_name.to_owned(),
            title,
            album,
            artist,
            album_artist,
            duration,
            None,
            None,
            track_number,
            bitrate,
        );
        let (txt, lrc) = track.read_sidecar_lyrics();
        track.txt_lyrics = txt;
        track.lrc_lyrics = lrc;

        println!("Successfully loaded `{}` via id3 fallback", file_path);

        Ok(track)
    }

    pub fn file_path(&self) -> &str {
        &self.file_path
    }

    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn album(&self) -> &str {
        &self.album
    }

    pub fn artist(&self) -> &str {
        &self.artist
    }

    pub fn album_artist(&self) -> &str {
        &self.album_artist
    }

    pub fn duration(&self) -> f64 {
        self.duration
    }

    pub fn txt_lyrics(&self) -> Option<&str> {
        self.txt_lyrics.as_deref()
    }

    pub fn lrc_lyrics(&self) -> Option<&str> {
        self.lrc_lyrics.as_deref()
    }

    pub fn track_number(&self) -> Option<u32> {
        self.track_number
    }

    pub fn bitrate(&self) -> Option<u32> {
        self.bitrate
    }

    /// Returns (txt_lyrics, lrc_lyrics) by parsing the path once
    fn read_sidecar_lyrics(&self) -> (Option<String>, Option<String>) {
        let path = Path::new(&self.file_path);
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let parent = path.parent().unwrap_or(Path::new(""));

        let txt_lyrics = std::fs::read_to_string(parent.join(format!("{}.txt", stem))).ok();
        let lrc_lyrics = std::fs::read_to_string(parent.join(format!("{}.lrc", stem))).ok();

        (txt_lyrics, lrc_lyrics)
    }
}

fn load_tracks_from_entry_batch(entry_batch: &[DirEntry]) -> Result<Vec<FsTrack>> {
    let track_results: Vec<Result<FsTrack>> = entry_batch
        .par_iter()
        .map(|file| FsTrack::new_from_path(file.path()))
        .collect();

    let mut tracks: Vec<FsTrack> = vec![];

    for track_result in track_results {
        match track_result {
            Ok(track) => {
                tracks.push(track);
            }
            Err(error) => {
                println!("{}", error);
            }
        }
    }

    Ok(tracks)
}

const GLOB_PATTERN: &str = "/**/*.{mp3,m4a,flac,ogg,opus,wav,MP3,M4A,FLAC,OGG,OPUS,WAV}";

pub fn load_tracks_from_directories(
    directories: &Vec<String>,
    conn: &mut Connection,
    app_handle: AppHandle,
) -> Result<()> {
    let now = Instant::now();

    // Single filesystem scan: collect all entries, then process in batches
    let mut all_entries: Vec<DirEntry> = Vec::new();
    for directory in directories.iter() {
        let globwalker = glob(format!("{}{}", directory, GLOB_PATTERN))?;
        for item in globwalker {
            all_entries.push(item?);
        }
    }

    let files_count = all_entries.len();
    println!("Files count: {}", files_count);
    let mut files_scanned: usize = 0;

    // Persistent caches across all batches
    let mut artist_cache: HashMap<String, i64> = HashMap::new();
    let mut album_cache: HashMap<(String, String), i64> = HashMap::new();

    for batch in all_entries.chunks(500) {
        let tracks = load_tracks_from_entry_batch(batch)?;
        db::add_tracks(&tracks, conn, &mut artist_cache, &mut album_cache)?;
        files_scanned += batch.len();
        let progress = if files_count > 0 {
            Some(files_scanned as f64 / files_count as f64)
        } else {
            None
        };
        app_handle
            .emit(
                "initialize-progress",
                ScanProgress {
                    progress,
                    files_scanned,
                    files_count: Some(files_count),
                },
            )
            .unwrap();
    }

    println!("==> Scanning tracks take: {}ms", now.elapsed().as_millis());

    Ok(())
}

pub fn refresh_tracks_from_directories(
    directories: &Vec<String>,
    conn: &mut Connection,
    app_handle: AppHandle,
) -> Result<()> {
    let now = Instant::now();

    // Get existing file paths from DB
    let existing_paths = db::get_existing_file_paths(conn)?;
    println!("Existing tracks in DB: {}", existing_paths.len());

    // Scan filesystem
    let mut all_entries: Vec<DirEntry> = Vec::new();
    for directory in directories.iter() {
        let globwalker = glob(format!("{}{}", directory, GLOB_PATTERN))?;
        for item in globwalker {
            all_entries.push(item?);
        }
    }

    // Split into new files only (skip existing)
    let mut disk_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut new_entries: Vec<DirEntry> = Vec::new();
    for entry in all_entries {
        let path_str = entry.path().display().to_string();
        disk_paths.insert(path_str.clone());
        if !existing_paths.contains(&path_str) {
            new_entries.push(entry);
        }
    }

    let new_count = new_entries.len();
    println!("New files to add: {}", new_count);

    // Delete tracks that are no longer on disk
    let deleted = db::delete_tracks_not_in(&disk_paths, conn)?;
    println!("Removed {} tracks no longer on disk", deleted);

    // Clean up orphaned albums/artists
    if deleted > 0 {
        let orphan_albums = db::delete_orphan_albums(conn)?;
        let orphan_artists = db::delete_orphan_artists(conn)?;
        println!("Cleaned up {} orphan albums, {} orphan artists", orphan_albums, orphan_artists);
    }

    // Insert new tracks in batches
    if new_count > 0 {
        let mut files_scanned: usize = 0;
        let mut artist_cache: HashMap<String, i64> = HashMap::new();
        let mut album_cache: HashMap<(String, String), i64> = HashMap::new();

        for batch in new_entries.chunks(500) {
            let tracks = load_tracks_from_entry_batch(batch)?;
            db::add_tracks(&tracks, conn, &mut artist_cache, &mut album_cache)?;
            files_scanned += batch.len();
            let progress = Some(files_scanned as f64 / new_count as f64);
            app_handle
                .emit(
                    "initialize-progress",
                    ScanProgress {
                        progress,
                        files_scanned,
                        files_count: Some(new_count),
                    },
                )
                .unwrap();
        }
    }

    println!("==> Library refresh took: {}ms", now.elapsed().as_millis());

    Ok(())
}
