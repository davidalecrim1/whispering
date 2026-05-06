use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use std::sync::{Arc, Mutex};

pub struct AudioCapture {
    _stream: Stream,
    buffer: Arc<Mutex<Vec<f32>>>,
    meter: RecordingLevel,
    sample_rate: u32,
}

#[derive(Clone)]
pub struct RecordingLevel {
    inner: Arc<Mutex<LevelMeter>>,
}

impl RecordingLevel {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(LevelMeter::default())),
        }
    }

    pub fn value(&self) -> f32 {
        self.inner.lock().unwrap().level()
    }

    fn update(&self, rms: f32) {
        self.inner.lock().unwrap().update(rms);
    }
}

#[derive(Debug, Default)]
struct LevelMeter {
    level: f32,
}

impl LevelMeter {
    fn update(&mut self, rms: f32) -> f32 {
        const INPUT_GAIN: f32 = 8.0;
        const ATTACK: f32 = 0.5;
        const RELEASE: f32 = 0.18;

        let normalized = (rms * INPUT_GAIN).clamp(0.0, 1.0);
        let smoothing = if normalized >= self.level {
            ATTACK
        } else {
            RELEASE
        };
        self.level += (normalized - self.level) * smoothing;
        self.level
    }

    fn level(&self) -> f32 {
        self.level
    }
}

impl AudioCapture {
    pub fn start(device_name: Option<&str>) -> Result<Self> {
        let host = cpal::default_host();

        let device: Device = if let Some(name) = device_name {
            host.input_devices()?
                .find(|d| {
                    d.description()
                        .map(|desc| desc.name() == name)
                        .unwrap_or(false)
                })
                .ok_or_else(|| anyhow!("Input device '{}' not found", name))?
        } else {
            host.default_input_device()
                .ok_or_else(|| anyhow!("No default input device available"))?
        };

        let config = device.default_input_config()?;
        let sample_rate = config.sample_rate();
        let channels = config.channels() as usize;

        let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let buffer_clone = buffer.clone();
        let meter = RecordingLevel::new();
        let meter_clone = meter.clone();

        let stream_config: StreamConfig = config.into();

        let stream = device.build_input_stream(
            &stream_config,
            move |data: &[f32], _| {
                // The overlay meter should reflect the same mono signal sent to Whisper.
                let mut buf = buffer_clone.lock().unwrap();
                let rms = downmix_into_buffer(data, channels, &mut buf);
                drop(buf);
                meter_clone.update(rms);
            },
            |err| log::error!("Audio stream error: {}", err),
            None,
        )?;

        stream.play()?;

        Ok(Self {
            _stream: stream,
            buffer,
            meter,
            sample_rate,
        })
    }

    /// Stops the stream and returns the captured PCM buffer resampled to 16kHz mono.
    pub fn stop(self) -> Result<Vec<f32>> {
        // Stream is dropped here, which stops capture
        let buffer = self.buffer.lock().unwrap().clone();
        if buffer.is_empty() {
            return Err(anyhow!("No audio captured"));
        }
        let resampled = resample_to_16k(&buffer, self.sample_rate);
        Ok(resampled)
    }

    pub fn level_reader(&self) -> RecordingLevel {
        self.meter.clone()
    }

    #[allow(dead_code)]
    pub fn available_devices() -> Result<Vec<String>> {
        let host = cpal::default_host();
        let names = host
            .input_devices()?
            .filter_map(|d| d.description().ok().map(|desc| desc.name().to_string()))
            .collect();
        Ok(names)
    }
}

fn downmix_into_buffer(data: &[f32], channels: usize, buffer: &mut Vec<f32>) -> f32 {
    if data.is_empty() || channels == 0 {
        return 0.0;
    }

    let mut sum_squares = 0.0_f32;
    let mut frames = 0_usize;

    for chunk in data.chunks(channels) {
        let sample: f32 = chunk.iter().sum::<f32>() / channels as f32;
        buffer.push(sample);
        sum_squares += sample * sample;
        frames += 1;
    }

    if frames == 0 {
        0.0
    } else {
        (sum_squares / frames as f32).sqrt()
    }
}

/// Linear interpolation resample to 16000 Hz.
fn resample_to_16k(input: &[f32], source_rate: u32) -> Vec<f32> {
    if source_rate == 16000 {
        return input.to_vec();
    }
    let ratio = source_rate as f64 / 16000.0;
    let out_len = (input.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let src_idx = src_pos as usize;
        let frac = src_pos - src_idx as f64;
        let s0 = input.get(src_idx).copied().unwrap_or(0.0);
        let s1 = input.get(src_idx + 1).copied().unwrap_or(s0);
        output.push(s0 + (s1 - s0) * frac as f32);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{downmix_into_buffer, LevelMeter};

    #[test]
    fn meter_stays_near_zero_for_silence() {
        let mut meter = LevelMeter::default();

        assert_eq!(meter.update(0.0), 0.0);
        assert_eq!(meter.level(), 0.0);
    }

    #[test]
    fn stronger_input_produces_higher_level() {
        let mut meter = LevelMeter::default();

        let quiet = meter.update(0.02);
        let louder = meter.update(0.12);

        assert!(louder > quiet);
    }

    #[test]
    fn meter_decay_is_smoothed() {
        let mut meter = LevelMeter::default();

        let peak = meter.update(0.2);
        let decayed = meter.update(0.0);

        assert!(peak > 0.0);
        assert!(decayed > 0.0);
        assert!(decayed < peak);
    }

    #[test]
    fn downmix_returns_rms_for_mono_samples() {
        let mut buffer = Vec::new();
        let rms = downmix_into_buffer(&[0.5, -0.5, 0.5, -0.5], 1, &mut buffer);

        assert_eq!(buffer, vec![0.5, -0.5, 0.5, -0.5]);
        assert!((rms - 0.5).abs() < 0.0001);
    }
}
