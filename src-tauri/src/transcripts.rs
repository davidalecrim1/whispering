use anyhow::Result;
use std::{
    path::Path,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const TRANSCRIPTS_DIR: &str = "transcripts";

pub fn save(text: &str, cache_dir: &Path) -> Result<PathBuf> {
    let dir = transcripts_dir(cache_dir);
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

pub fn transcripts_dir(cache_dir: &Path) -> PathBuf {
    cache_dir.join(TRANSCRIPTS_DIR)
}

#[cfg(test)]
mod tests {
    use super::{save, transcripts_dir};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TestCacheDir {
        root: PathBuf,
        cache_dir: PathBuf,
    }

    impl TestCacheDir {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be valid")
                .as_nanos();
            let root = std::env::temp_dir().join(format!("whispering-transcripts-test-{unique}"));
            let cache_dir = root.join("cache").join("Whispering");

            Self { root, cache_dir }
        }
    }

    impl Drop for TestCacheDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn save_writes_timestamped_file_and_latest_copy() {
        let test_cache = TestCacheDir::new();

        let saved_path =
            save("hello world", &test_cache.cache_dir).expect("transcript should save");
        let transcript_dir = transcripts_dir(&test_cache.cache_dir);
        let latest_path = transcript_dir.join("latest.txt");

        assert!(saved_path.exists());
        assert!(latest_path.exists());
        assert!(saved_path.starts_with(&transcript_dir));
        assert_eq!(
            fs::read_to_string(saved_path).expect("saved transcript should be readable"),
            "hello world\n"
        );
        assert_eq!(
            fs::read_to_string(latest_path).expect("latest transcript should be readable"),
            "hello world\n"
        );
    }
}
