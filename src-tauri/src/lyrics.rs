use crate::lrclib::get::{request, Response};
use crate::utils::strip_timestamp;
use crate::lrclib::search;
use crate::persistent_entities::PersistentTrack;
use anyhow::Result;
use lofty::{
    config::{ParseOptions, WriteOptions},
    file::AudioFile,
    flac::FlacFile,
    id3::v2::{
        BinaryFrame, Frame, FrameId, Id3v2Tag, SyncTextContentType, SynchronizedTextFrame,
        TimestampFormat, UnsynchronizedTextFrame,
    },
    mpeg::MpegFile,
    TextEncoding,
};
use lrc::Lyrics;
use std::collections::HashSet;
use std::fs::{remove_file, write, OpenOptions};
use std::io::Seek;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Clone, Debug)]
pub enum GetLyricsError {
    #[error("This track does not exist in LRCLIB database")]
    NotFound,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MatchSource {
    Exact,
    DurationFallback,
    FuzzyFallback,
    None,
}

const MIN_TITLE_SIMILARITY: f64 = 0.3;

pub async fn download_lyrics_for_track(
    track: PersistentTrack,
    is_try_embed_lyrics: bool,
    lrclib_instance: &str,
    duration_tolerance: f64,
    fuzzy_search_enabled: bool,
) -> Result<(Response, MatchSource)> {
    let lyrics = request(
        &track.title,
        &track.album_name,
        &track.artist_name,
        track.duration,
        lrclib_instance,
    )
    .await?;

    // If exact match found, use it
    if !matches!(lyrics, Response::None) {
        let response = apply_lyrics_for_track(track, lyrics, is_try_embed_lyrics).await?;
        return Ok((response, MatchSource::Exact));
    }

    // Skip fallback searches if tolerance is 0
    if duration_tolerance <= 0.0 {
        let response = apply_lyrics_for_track(track, Response::None, is_try_embed_lyrics).await?;
        return Ok((response, MatchSource::None));
    }

    // Fallback 1: field-based search with duration tolerance
    let fallback = search_with_duration_tolerance(
        &track.title,
        &track.album_name,
        &track.artist_name,
        track.duration,
        duration_tolerance,
        lrclib_instance,
    )
    .await;

    if let Ok(ref lyrics) = fallback {
        if !matches!(lyrics, Response::None) {
            let response = apply_lyrics_for_track(track, fallback.unwrap(), is_try_embed_lyrics).await?;
            return Ok((response, MatchSource::DurationFallback));
        }
    }

    if !fuzzy_search_enabled {
        let response = apply_lyrics_for_track(track, Response::None, is_try_embed_lyrics).await?;
        return Ok((response, MatchSource::None));
    }

    // Fallback 2: fuzzy q-based search with text similarity validation
    let fuzzy = search_fuzzy_fallback(
        &track.title,
        &track.artist_name,
        track.duration,
        duration_tolerance,
        lrclib_instance,
    )
    .await;

    match fuzzy {
        Ok(lyrics) => {
            let source = if matches!(lyrics, Response::None) {
                MatchSource::None
            } else {
                MatchSource::FuzzyFallback
            };
            let response = apply_lyrics_for_track(track, lyrics, is_try_embed_lyrics).await?;
            Ok((response, source))
        }
        Err(_) => {
            let response = apply_lyrics_for_track(track, Response::None, is_try_embed_lyrics).await?;
            Ok((response, MatchSource::None))
        }
    }
}

fn normalize_text(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn text_similarity(a: &str, b: &str) -> f64 {
    let a_norm = normalize_text(a);
    let b_norm = normalize_text(b);

    if a_norm.is_empty() && b_norm.is_empty() {
        return 1.0;
    }
    if a_norm.is_empty() || b_norm.is_empty() {
        return 0.0;
    }

    let a_words: HashSet<&str> = a_norm.split_whitespace().collect();
    let b_words: HashSet<&str> = b_norm.split_whitespace().collect();

    let intersection = a_words.intersection(&b_words).count();
    let union = a_words.union(&b_words).count();

    if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
}

fn search_item_to_response(item: search::SearchItem) -> Response {
    match item.synced_lyrics {
        Some(synced) => {
            let plain = item.plain_lyrics.unwrap_or_else(|| strip_timestamp(&synced));
            Response::SyncedLyrics(synced, plain)
        }
        None => match item.plain_lyrics {
            Some(plain) => Response::UnsyncedLyrics(plain),
            None => {
                if item.instrumental {
                    Response::IsInstrumental
                } else {
                    Response::None
                }
            }
        },
    }
}

fn pick_best_match(
    results: impl IntoIterator<Item = search::SearchItem>,
    duration: f64,
    duration_tolerance: f64,
) -> Option<search::SearchItem> {
    results
        .into_iter()
        .filter(|item| {
            item.duration
                .map(|d| (d - duration).abs() <= duration_tolerance)
                .unwrap_or(false)
        })
        .min_by(|a, b| {
            let score = |item: &search::SearchItem| -> i32 {
                if item.synced_lyrics.is_some() { 0 }
                else if item.plain_lyrics.is_some() { 1 }
                else if item.instrumental { 2 }
                else { 3 }
            };
            let score_cmp = score(a).cmp(&score(b));
            if score_cmp != std::cmp::Ordering::Equal {
                return score_cmp;
            }
            let da = a.duration.map(|d| (d - duration).abs()).unwrap_or(f64::MAX);
            let db = b.duration.map(|d| (d - duration).abs()).unwrap_or(f64::MAX);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
}

async fn search_with_duration_tolerance(
    title: &str,
    album_name: &str,
    artist_name: &str,
    duration: f64,
    duration_tolerance: f64,
    lrclib_instance: &str,
) -> Result<Response> {
    let results = search::request(title, album_name, artist_name, "", lrclib_instance).await?;

    match pick_best_match(results.0, duration, duration_tolerance) {
        Some(item) => Ok(search_item_to_response(item)),
        None => Ok(Response::None),
    }
}

async fn search_fuzzy_fallback(
    title: &str,
    artist_name: &str,
    duration: f64,
    duration_tolerance: f64,
    lrclib_instance: &str,
) -> Result<Response> {
    let q = format!("{} {}", title, artist_name);
    let results = search::request("", "", "", &q, lrclib_instance).await?;

    let candidates: Vec<_> = results.0.into_iter()
        .filter(|item| {
            let title_sim = item.name.as_deref()
                .map(|n| text_similarity(title, n))
                .unwrap_or(0.0);
            title_sim >= MIN_TITLE_SIMILARITY
        })
        .collect();

    match pick_best_match(candidates, duration, duration_tolerance) {
        Some(item) => Ok(search_item_to_response(item)),
        None => Ok(Response::None),
    }
}

pub async fn apply_string_lyrics_for_track(
    track: &PersistentTrack,
    plain_lyrics: &str,
    synced_lyrics: &str,
    is_try_embed_lyrics: bool,
) -> Result<()> {
    save_plain_lyrics(&track.file_path, plain_lyrics)?;
    save_synced_lyrics(&track.file_path, synced_lyrics)?;

    if is_try_embed_lyrics {
        embed_lyrics(&track.file_path, &plain_lyrics, &synced_lyrics);
    }

    Ok(())
}

pub async fn apply_lyrics_for_track(
    track: PersistentTrack,
    lyrics: Response,
    is_try_embed_lyrics: bool,
) -> Result<Response> {
    match &lyrics {
        Response::SyncedLyrics(synced_lyrics, plain_lyrics) => {
            save_synced_lyrics(&track.file_path, &synced_lyrics)?;
            if is_try_embed_lyrics {
                embed_lyrics(&track.file_path, &plain_lyrics, &synced_lyrics);
            }
            Ok(lyrics)
        }
        Response::UnsyncedLyrics(plain_lyrics) => {
            save_plain_lyrics(&track.file_path, &plain_lyrics)?;
            if is_try_embed_lyrics {
                embed_lyrics(&track.file_path, &plain_lyrics, "");
            }
            Ok(lyrics)
        }
        Response::IsInstrumental => {
            save_instrumental(&track.file_path)?;
            Ok(lyrics)
        }
        _ => Ok(lyrics),
    }
}

fn save_plain_lyrics(track_path: &str, lyrics: &str) -> Result<()> {
    let txt_path = build_txt_path(track_path)?;
    let lrc_path = build_lrc_path(track_path)?;

    let _ = remove_file(lrc_path);

    if lyrics.is_empty() {
        let _ = remove_file(txt_path);
    } else {
        write(txt_path, lyrics)?;
    }
    Ok(())
}

fn save_synced_lyrics(track_path: &str, lyrics: &str) -> Result<()> {
    let txt_path = build_txt_path(track_path)?;
    let lrc_path = build_lrc_path(track_path)?;
    if lyrics.is_empty() {
        let _ = remove_file(lrc_path);
    } else {
        let _ = remove_file(txt_path);
        write(lrc_path, lyrics)?;
    }
    Ok(())
}

fn save_instrumental(track_path: &str) -> Result<()> {
    let txt_path = build_txt_path(track_path)?;
    let lrc_path = build_lrc_path(track_path)?;

    let _ = remove_file(&lrc_path);
    let _ = remove_file(txt_path);

    write(lrc_path, "[au: instrumental]")?;

    Ok(())
}

fn build_txt_path(track_path: &str) -> Result<PathBuf> {
    let path = Path::new(track_path);
    let parent_path = path.parent().unwrap();
    let file_name_without_extension = path.file_stem().unwrap().to_str().unwrap();
    let txt_path =
        Path::new(parent_path).join(format!("{}.{}", file_name_without_extension, "txt"));

    Ok(txt_path)
}

fn build_lrc_path(track_path: &str) -> Result<PathBuf> {
    let path = Path::new(track_path);
    let parent_path = path.parent().unwrap();
    let file_name_without_extension = path.file_stem().unwrap().to_str().unwrap();
    let lrc_path =
        Path::new(parent_path).join(format!("{}.{}", file_name_without_extension, "lrc"));

    Ok(lrc_path)
}

fn embed_lyrics(track_path: &str, plain_lyrics: &str, synced_lyrics: &str) {
    if track_path.to_lowercase().ends_with(".mp3") {
        match embed_lyrics_mp3(track_path, plain_lyrics, synced_lyrics) {
            Ok(_) => (),
            Err(e) => println!("Error embedding lyrics in MP3: {}", e),
        }
    } else if track_path.to_lowercase().ends_with(".flac") {
        match embed_lyrics_flac(track_path, plain_lyrics, synced_lyrics) {
            Ok(_) => (),
            Err(e) => println!("Error embedding lyrics in FLAC: {}", e),
        }
    }
}

fn embed_lyrics_flac(track_path: &str, plain_lyrics: &str, synced_lyrics: &str) -> Result<()> {
    let mut file_content = OpenOptions::new().read(true).write(true).open(track_path)?;
    let mut flac_file = FlacFile::read_from(&mut file_content, ParseOptions::new())?;

    if let Some(vorbis_comments) = flac_file.vorbis_comments_mut() {
        if !plain_lyrics.is_empty() {
            vorbis_comments.insert("UNSYNCEDLYRICS".to_string(), plain_lyrics.to_string());
        } else {
            let _ = vorbis_comments.remove("UNSYNCEDLYRICS");
        }

        if !synced_lyrics.is_empty() {
            vorbis_comments.insert("LYRICS".to_string(), synced_lyrics.to_string());
        } else {
            let _ = vorbis_comments.remove("LYRICS");
        }

        file_content.seek(std::io::SeekFrom::Start(0))?;
        flac_file.save_to(&mut file_content, WriteOptions::default())?;
    }

    Ok(())
}

fn embed_lyrics_mp3(track_path: &str, plain_lyrics: &str, synced_lyrics: &str) -> Result<()> {
    let mut file_content = OpenOptions::new().read(true).write(true).open(track_path)?;
    let mut mp3_file = MpegFile::read_from(&mut file_content, ParseOptions::new())?;

    if let Some(id3v2) = mp3_file.id3v2_mut() {
        insert_id3v2_uslt_frame(id3v2, plain_lyrics)?;
        insert_id3v2_sylt_frame(id3v2, synced_lyrics)?;

        file_content.seek(std::io::SeekFrom::Start(0))?;
        mp3_file.save_to(&mut file_content, WriteOptions::default())?;
    }

    Ok(())
}

fn insert_id3v2_uslt_frame(id3v2: &mut Id3v2Tag, plain_lyrics: &str) -> Result<()> {
    if !plain_lyrics.is_empty() {
        let uslt_frame = UnsynchronizedTextFrame::new(
            TextEncoding::UTF8,
            [b'X', b'X', b'X'],
            "".to_string(),
            plain_lyrics.to_string(),
        );
        id3v2.insert(Frame::UnsynchronizedText(uslt_frame));
    } else {
        let _ = id3v2.remove(&FrameId::new("USLT")?);
    }

    Ok(())
}

fn insert_id3v2_sylt_frame(id3v2: &mut Id3v2Tag, synced_lyrics: &str) -> Result<()> {
    if !synced_lyrics.is_empty() {
        let synced_lyrics_vec = synced_lyrics_to_sylt_vec(synced_lyrics)?;

        let sylt_frame = SynchronizedTextFrame::new(
            TextEncoding::UTF8,
            [b'X', b'X', b'X'],
            TimestampFormat::MS,
            SyncTextContentType::Lyrics,
            None,
            synced_lyrics_vec,
        );

        let sylt_frame_byte = sylt_frame.as_bytes()?;
        let sylt_frame_id = FrameId::new("SYLT")?;
        id3v2.insert(Frame::Binary(BinaryFrame::new(
            sylt_frame_id,
            sylt_frame_byte,
        )));
    } else {
        let _ = id3v2.remove(&FrameId::new("SYLT")?);
    }

    Ok(())
}

fn synced_lyrics_to_sylt_vec(synced_lyrics: &str) -> Result<Vec<(u32, String)>> {
    let lyrics = Lyrics::from_str(synced_lyrics)?;
    let lyrics_vec = lyrics.get_timed_lines();

    let converted_lyrics: Vec<(u32, String)> = lyrics_vec
        .iter()
        .map(|(time_tag, text)| (time_tag.get_timestamp() as u32, text.to_string()))
        .collect();

    Ok(converted_lyrics)
}
