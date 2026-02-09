use collapse::collapse;
use regex::Regex;
use secular::lower_lay_string;
use std::sync::LazyLock;

static RE_PUNCTUATION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"[`~!@#$%^&*()_|+\-=?;:",.<>\{\}\[\]\\\/]"#).unwrap());
static RE_QUOTES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"['']"#).unwrap());
static RE_TIMESTAMP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\[[^\]]*\] *").unwrap());
pub static RE_INSTRUMENTAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[au:\s*instrumental\]").unwrap());

pub fn prepare_input(input: &str) -> String {
    let mut prepared_input = lower_lay_string(&input);

    prepared_input = RE_PUNCTUATION.replace_all(&prepared_input, " ").to_string();
    prepared_input = RE_QUOTES.replace_all(&prepared_input, "").to_string();

    prepared_input = prepared_input.to_lowercase();
    prepared_input = collapse(&prepared_input);

    prepared_input
}

pub fn strip_timestamp(synced_lyrics: &str) -> String {
    let plain_lyrics = RE_TIMESTAMP.replace_all(synced_lyrics, "");
    plain_lyrics.to_string()
}
