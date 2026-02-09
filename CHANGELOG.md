# Changelog

## [1.0.3] - 2026-02-09

### Fixed
- Fix off-by-one bug in LRCLIB challenge solver nonce verification
- Fix multiline regex in `strip_timestamp` missing lines and being too greedy
- Fix parameter shadowing in search-library composable causing filter state loss
- Replace `unwrap()` on mutex locks with proper error handling across 26+ locations
- Fix event listener memory leaks in 7 Vue components (NowPlaying, Library, TrackItem, EditLyrics, PublishLyrics, PublishPlainText, FlagLyrics)
- Move challenge solver to `spawn_blocking()` to avoid blocking the async runtime
- Reduce mutex hold duration during library init/refresh operations

### Changed
- Cache compiled regexes using `LazyLock` for better performance
- Share HTTP client with connection pooling across all LRCLIB API modules
- Consolidate duplicated `ResponseError` type into shared lrclib module
- Split monolithic `main.rs` (842 lines) into `commands/` modules (library, lyrics, player)
- Replace 100ms notification polling with Tauri event-based system

### Meta
- Bump version to 1.0.3
- Update User-Agent to reference fork repository
