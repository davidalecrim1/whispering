const SOUND_BEGIN: &str = "/System/Library/Components/CoreAudio.component/Contents/SharedSupport/SystemSounds/system/begin_record.caf";
const SOUND_END: &str = "/System/Library/Components/CoreAudio.component/Contents/SharedSupport/SystemSounds/system/end_record.caf";

pub fn play_start() {
    play(SOUND_BEGIN);
}

pub fn play_stop() {
    play(SOUND_END);
}

fn play(path: &str) {
    #[cfg(target_os = "macos")]
    {
        use objc2::AnyThread;
        use objc2_app_kit::NSSound;
        use objc2_foundation::NSString;

        let path = NSString::from_str(path);
        if let Some(sound) = NSSound::initWithContentsOfFile_byReference(
            NSSound::alloc(),
            &path,
            true,
        ) {
            sound.play();
        }
    }
}
