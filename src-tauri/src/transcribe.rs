use anyhow::{anyhow, Result};
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct Transcriber {
    ctx: WhisperContext,
}

impl Transcriber {
    pub fn load(model_path: &Path) -> Result<Self> {
        if !model_path.exists() {
            return Err(anyhow!(
                "Model not found at {}. Download a ggml model and place it there.",
                model_path.display()
            ));
        }
        let ctx = WhisperContext::new_with_params(
            model_path.to_str().unwrap(),
            WhisperContextParameters::default(),
        )
        .map_err(|e| anyhow!("Failed to load whisper model: {:?}", e))?;

        Ok(Self { ctx })
    }

    pub fn transcribe(&self, audio: &[f32], language: Option<&str>) -> Result<String> {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(language);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        let mut state = self
            .ctx
            .create_state()
            .map_err(|e| anyhow!("Failed to create whisper state: {:?}", e))?;

        state
            .full(params, audio)
            .map_err(|e| anyhow!("Whisper inference failed: {:?}", e))?;

        let num_segments = state.full_n_segments();

        let mut text = String::new();
        for i in 0..num_segments {
            if let Some(segment) = state.get_segment(i) {
                if let Ok(s) = segment.to_str() {
                    text.push_str(s);
                }
            }
        }

        Ok(text.trim().to_string())
    }
}
