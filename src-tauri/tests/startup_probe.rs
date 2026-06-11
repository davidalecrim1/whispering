use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

struct ProbeRoot {
    root: PathBuf,
}

impl ProbeRoot {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("whispering-startup-probe-{unique}"));
        fs::create_dir_all(&root).expect("probe root should be created");
        Self { root }
    }

    fn home_dir(&self) -> PathBuf {
        self.root.join("home")
    }

    fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }
}

impl Drop for ProbeRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn startup_probe_reports_platform_hotkey_and_bootstrap_paths() {
    let probe_root = ProbeRoot::new();
    let binary = env!("CARGO_BIN_EXE_whispering");

    let mut command = Command::new(binary);
    command.env("WHISPERING_TEST_MODE", "startup-probe");

    if cfg!(target_os = "windows") {
        command.env("USERPROFILE", probe_root.home_dir());
        command.env("LOCALAPPDATA", probe_root.cache_dir());
    } else {
        command.env("HOME", probe_root.home_dir());
        command.env("XDG_CACHE_HOME", probe_root.cache_dir());
    }

    let output = command.output().expect("startup probe should run");
    assert!(
        output.status.success(),
        "startup probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    let expected_platform = if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    };
    let expected_hotkey = if cfg!(target_os = "macos") {
        "Ctrl+Cmd+M"
    } else {
        "Ctrl+Alt+M"
    };

    assert!(stdout.contains(&format!("platform={expected_platform}")));
    assert!(stdout.contains(&format!("hotkey={expected_hotkey}")));
    assert!(stdout.contains("config_dir="));
    assert!(stdout.contains("model_dir="));
    assert!(stdout.contains("cache_dir="));

    let config_dir = probe_root.home_dir().join(".whispering");
    let model_dir = config_dir.join("models");
    assert!(config_dir.is_dir());
    assert!(model_dir.is_dir());
}
