use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use ringbuf::traits::{Consumer, Producer, Split};
use ringbuf::HeapRb;
use rustpotter::{
    DetectorConfig, Rustpotter, RustpotterConfig, ScoreMode, WakewordRef,
    WakewordRefBuildFromBuffers,
};
use tauri::{AppHandle, Emitter};

use super::types::VoiceWakePayload;

pub struct WakeWordDetectorManager {
    stop_flag: Arc<AtomicBool>,
    sensitivity: Arc<Mutex<f32>>,
    worker_handle: Option<JoinHandle<()>>,
}

impl WakeWordDetectorManager {
    pub fn start(app_handle: AppHandle, initial_sensitivity: f32) -> Result<Self, String> {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone = stop_flag.clone();
        let sensitivity = Arc::new(Mutex::new(initial_sensitivity));
        let sensitivity_clone = sensitivity.clone();

        // Spawn background worker thread for audio capture & keyword detection
        let worker_handle = thread::spawn(move || {
            // 1. Initialize cpal WASAPI input stream inside the background thread
            let host = cpal::default_host();
            let device = match host.default_input_device() {
                Some(d) => d,
                None => {
                    eprintln!("[VoiceDetector] No default audio input device available.");
                    return;
                }
            };

            let config: StreamConfig = match device.default_input_config() {
                Ok(c) => c.into(),
                Err(e) => {
                    eprintln!("[VoiceDetector] Failed to obtain audio input config: {}", e);
                    return;
                }
            };

            let sample_rate = config.sample_rate.0;
            let channels = config.channels as usize;

            // 2. Lock-free ring buffer for continuous low-overhead audio streaming
            let buffer_capacity = (sample_rate as usize) * 3;
            let rb = HeapRb::<f32>::new(buffer_capacity);
            let (mut producer, mut consumer) = rb.split();

            // 3. Build and start audio input stream
            let stream = match device.build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    for chunk in data.chunks_exact(channels) {
                        let mono_sample = chunk.iter().sum::<f32>() / channels as f32;
                        let _ = producer.try_push(mono_sample);
                    }
                },
                move |err| {
                    eprintln!("[VoiceDetector] Audio capture stream error: {}", err);
                },
                None,
            ) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[VoiceDetector] Failed to build audio input stream: {}", e);
                    return;
                }
            };

            if let Err(e) = stream.play() {
                eprintln!("[VoiceDetector] Failed to start audio stream: {}", e);
                return;
            }

            // 4. Initialize Rustpotter detector
            let current_sens = *sensitivity_clone.lock().unwrap();

            let mut rustpotter_config = RustpotterConfig::default();
            rustpotter_config.fmt.sample_rate = 16000;
            rustpotter_config.fmt.sample_format = rustpotter::SampleFormat::I16;
            rustpotter_config.fmt.channels = 1;
            rustpotter_config.detector.threshold = current_sens;
            rustpotter_config.detector.score_mode = ScoreMode::Max;

            let mut rustpotter = match Rustpotter::new(&rustpotter_config) {
                Ok(rp) => rp,
                Err(e) => {
                    eprintln!("[VoiceDetector] Failed to initialize Rustpotter: {:?}", e);
                    return;
                }
            };

            // Setup 'Deyana' keyword template ref from synthetic WAV sample
            let deyana_wav = generate_deyana_keyword_wav();
            let mut sample_buffers = HashMap::new();
            sample_buffers.insert("sample_1".to_string(), deyana_wav);

            match WakewordRef::new_from_sample_buffers(
                "Deyana".to_string(),
                Some(current_sens),
                None,
                sample_buffers,
                10,
            ) {
                Ok(wakeword_ref) => {
                    if let Err(e) = rustpotter.add_wakeword_ref("Deyana", wakeword_ref) {
                        eprintln!("[VoiceDetector] Error adding Deyana wakeword ref: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("[VoiceDetector] Failed to construct Deyana wakeword ref: {}", e);
                }
            }

            let mut audio_buffer_16k: Vec<f32> = Vec::new();
            let mut last_wake = Instant::now() - Duration::from_secs(5);

            while !stop_flag_clone.load(Ordering::Relaxed) {
                let mut raw_samples = Vec::new();
                while let Some(sample) = consumer.try_pop() {
                    raw_samples.push(sample);
                }

                if raw_samples.is_empty() {
                    thread::sleep(Duration::from_millis(20));
                    continue;
                }

                // Resample audio samples to 16kHz mono f32 PCM
                let resampled = resample_to_16k(&raw_samples, sample_rate);
                audio_buffer_16k.extend(resampled);

                // Update sensitivity threshold dynamically if modified
                if let Ok(sens_guard) = sensitivity_clone.try_lock() {
                    let active_sens = *sens_guard;
                    rustpotter.update_detector_config(&DetectorConfig {
                        threshold: active_sens,
                        score_mode: ScoreMode::Max,
                        ..Default::default()
                    });
                }

                let samples_per_frame = rustpotter.get_samples_per_frame();
                while audio_buffer_16k.len() >= samples_per_frame {
                    let frame: Vec<f32> = audio_buffer_16k.drain(..samples_per_frame).collect();
                    let i16_samples: Vec<i16> = frame
                        .iter()
                        .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                        .collect();

                    if let Some(detection) = rustpotter.process_samples(i16_samples) {
                        if detection.score >= current_sens
                            && last_wake.elapsed() >= Duration::from_millis(1500)
                        {
                            last_wake = Instant::now();
                            println!(
                                "[VoiceWake] Detected wake word 'Deyana' (score: {:.3})",
                                detection.score
                            );

                            let now_secs = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();

                            let payload = VoiceWakePayload {
                                keyword: "Deyana".to_string(),
                                score: detection.score,
                                timestamp: now_secs,
                            };

                            let _ = app_handle.emit("voice:wake", &payload);
                        }
                    }
                }
            }

            // Stream drops naturally when thread exits
            drop(stream);
        });

        Ok(Self {
            stop_flag,
            sensitivity,
            worker_handle: Some(worker_handle),
        })
    }

    pub fn set_sensitivity(&self, new_sensitivity: f32) {
        if let Ok(mut lock) = self.sensitivity.lock() {
            *lock = new_sensitivity.clamp(0.1, 0.99);
        }
    }

    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for WakeWordDetectorManager {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Linear sample resampler converting raw PCM from src_rate to 16,000Hz mono f32 PCM.
pub fn resample_to_16k(input: &[f32], src_rate: u32) -> Vec<f32> {
    if input.is_empty() {
        return Vec::new();
    }
    if src_rate == 16000 {
        return input.to_vec();
    }
    let ratio = 16000.0 / src_rate as f32;
    let target_len = ((input.len() as f32) * ratio).round() as usize;
    if target_len == 0 {
        return Vec::new();
    }
    let mut output = Vec::with_capacity(target_len);

    for i in 0..target_len {
        let src_idx = (i as f32) / ratio;
        let idx0 = src_idx.floor() as usize;
        let idx1 = (idx0 + 1).min(input.len() - 1);
        let frac = src_idx - (idx0 as f32);

        let s0 = input.get(idx0).copied().unwrap_or(0.0);
        let s1 = input.get(idx1).copied().unwrap_or(s0);
        let sample = s0 * (1.0 - frac) + s1 * frac;
        output.push(sample);
    }
    output
}

/// Generates a synthetic 16kHz mono WAV sample for "Deyana" keyword template matching using hound.
fn generate_deyana_keyword_wav() -> Vec<u8> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = std::io::Cursor::new(Vec::new());
    if let Ok(mut writer) = hound::WavWriter::new(&mut cursor, spec) {
        let total_samples = 16000 * 1; // 1 second
        for i in 0..total_samples {
            let t = (i as f32) / 16000.0;
            let freq = if t < 0.3 {
                220.0 + t * 300.0
            } else if t < 0.6 {
                440.0 - (t - 0.3) * 200.0
            } else {
                330.0 + (t - 0.6) * 150.0
            };
            let sample_f32 = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.5;
            let sample_i16 = (sample_f32 * (i16::MAX as f32)) as i16;
            let _ = writer.write_sample(sample_i16);
        }
        let _ = writer.finalize();
    }
    cursor.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_to_16k_passthrough() {
        let input = vec![0.1, 0.2, 0.3, 0.4];
        let resampled = resample_to_16k(&input, 16000);
        assert_eq!(resampled, input);
    }

    #[test]
    fn test_resample_to_16k_downsampling() {
        let input: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
        let resampled = resample_to_16k(&input, 32000);
        assert_eq!(resampled.len(), 50);
    }

    #[test]
    fn test_generate_deyana_keyword_wav_validity() {
        let wav_bytes = generate_deyana_keyword_wav();
        assert!(!wav_bytes.is_empty());
        let reader = hound::WavReader::new(std::io::Cursor::new(wav_bytes));
        assert!(reader.is_ok());
        let r = reader.unwrap();
        assert_eq!(r.spec().sample_rate, 16000);
        assert_eq!(r.spec().channels, 1);
    }
}
