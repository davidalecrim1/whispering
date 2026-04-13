use crate::settings::Config;

/// Request accessibility permission (for keystroke injection via enigo).
/// Shows the system dialog if not yet granted.
pub fn request_accessibility(config: &mut Config) {
    #[cfg(target_os = "macos")]
    unsafe {
        use accessibility_sys::{AXIsProcessTrusted, AXIsProcessTrustedWithOptions};
        use core_foundation::base::TCFType;
        use core_foundation::boolean::CFBoolean;
        use core_foundation::dictionary::CFDictionary;
        use core_foundation::string::CFString;

        if AXIsProcessTrusted() {
            return;
        }

        if config.permission_prompts.accessibility_requested {
            return;
        }

        let key = CFString::new("AXTrustedCheckOptionPrompt");
        let val = CFBoolean::true_value();
        let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), val.as_CFType())]);
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef());
        config.permission_prompts.accessibility_requested = true;
    }
}

/// Request microphone permission via AVFoundation.
/// On first call, shows the system mic permission dialog.
pub fn request_microphone() {
    #[cfg(target_os = "macos")]
    unsafe {
        use block2::StackBlock;
        use objc2::runtime::Bool;
        use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio};

        let audio_type = AVMediaTypeAudio.unwrap();
        let status = AVCaptureDevice::authorizationStatusForMediaType(audio_type);

        if status != AVAuthorizationStatus::NotDetermined {
            return;
        }

        let handler = StackBlock::new(|_granted: Bool| {});
        AVCaptureDevice::requestAccessForMediaType_completionHandler(audio_type, &handler);
    }
}
