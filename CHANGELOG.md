# Changelog

## [1.1.0] - 2026-02-09

### Added
- Sort options for track lists: sort by title, duration, track number, or lyrics status (asc/desc)
- Lyrics type preference setting: choose between synced & plain (both), synced only, or plain only
- Lyrics coverage dashboard: stacked progress bar showing synced/plain/instrumental/missing counts
- Duration tolerance fallback: when exact match fails, search for tracks within a configurable duration window (default ± 3s)
- Fuzzy text matching fallback: broader search with Jaccard word similarity validation when field-based search fails
- Configurable duration tolerance (0–5s range slider) and fuzzy search toggle in settings
- Retry failed downloads button in download viewer
- Separate download status counters: Found, Skipped (orange), Not Found, Failed (red)
- Apply button in lyrics Preview modal to apply search results directly
- Instrumental tracks now saved as `.lrc` with `[au: instrumental]` marker

### Changed
- Bump version to 1.1.0

## [1.0.3] - 2026-02-09

### Fixed
- Fix off-by-one bug in LRCLIB challenge solver nonce verification
- Fix multiline regex in `strip_timestamp` missing lines and being too greedy
- Fix parameter shadowing in search-library composable causing filter state loss
- Replace `unwrap()` on mutex locks with proper error handling across 26+ locations
- Fix event listener memory leaks in 7 Vue components (NowPlaying, Library, TrackItem, EditLyrics, PublishLyrics, PublishPlainText, FlagLyrics)
- Move challenge solver to `spawn_blocking()` to avoid blocking the async runtime
- Reduce mutex hold duration during library init/refresh operations
- Skip unnecessary lyrics re-downloads when track already has same type (plain→plain, synced→synced)
- Fix DownloadViewer showing "Configuration" instead of "Downloading"/"Downloaded"
- Remove undefined `downloadUpdate` click handlers in About page

### Changed
- Cache compiled regexes using `LazyLock` for better performance
- Share HTTP client with connection pooling across all LRCLIB API modules
- Consolidate duplicated `ResponseError` type into shared lrclib module
- Split monolithic `main.rs` (842 lines) into `commands/` modules (library, lyrics, player)
- Replace 100ms notification polling with Tauri event-based system
- Tighten CSP: restrict media-src and connect-src to required origins only
- Remove duplicate `.textarea` and `.modal-content` CSS definitions
- Remove 4 stale Vite aliases pointing to non-existent directories
- Remove unused code (imports, functions, variables)

### Meta
- Bump version to 1.0.3
- Update User-Agent to reference fork repository
