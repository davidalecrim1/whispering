use anyhow::Result;
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const APP_CACHE_DIR: &str = "Whispering";
const TRANSCRIPTS_DIR: &str = "transcripts";

pub fn save(text: &str) -> Result<PathBuf> {
    let dir = transcripts_dir();
    std::fs::create_dir_all(&dir)?;

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let path = dir.join(format!("transcript-{}.txt", timestamp));
    let contents = if text.ends_with('\n') {
        text.to_string()
    } else {
        format!("{}\n", text)
    };

    std::fs::write(&path, &contents)?;
    std::fs::write(dir.join("latest.txt"), contents)?;

    Ok(path)
}

pub fn transcripts_dir() -> PathBuf {
    dirs_next::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(APP_CACHE_DIR)
        .join(TRANSCRIPTS_DIR)
}
