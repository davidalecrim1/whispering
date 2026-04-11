use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use std::sync::{Arc, Mutex};

pub struct AudioCapture {
    _stream: Stream,
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
}

impl AudioCapture {
    pub fn start(device_name: Option<&str>) -> Result<Self> {
        let host = cpal::default_host();

        let device: Device = if let Some(name) = device_name {
            host.input_devices()?
                .find(|d| d.description().map(|desc| desc.name() == name).unwrap_or(false))
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

        let stream_config: StreamConfig = config.into();

        let stream = device.build_input_stream(
            &stream_config,
            move |data: &[f32], _| {
                // Downmix to mono by averaging channels
                let mut buf = buffer_clone.lock().unwrap();
                for chunk in data.chunks(channels) {
                    let sample: f32 = chunk.iter().sum::<f32>() / channels as f32;
                    buf.push(sample);
                }
            },
            |err| log::error!("Audio stream error: {}", err),
            None,
        )?;

        stream.play()?;

        Ok(Self {
            _stream: stream,
            buffer,
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

    pub fn available_devices() -> Result<Vec<String>> {
        let host = cpal::default_host();
        let names = host
            .input_devices()?
            .filter_map(|d| d.description().ok().map(|desc| desc.name().to_string()))
            .collect();
        Ok(names)
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
