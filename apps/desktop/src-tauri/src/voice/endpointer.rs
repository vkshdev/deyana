use serde::{Deserialize, Serialize};

pub const SAMPLE_RATE: usize = 16000;
pub const FRAME_SIZE_SAMPLES: usize = 320; // 20ms at 16kHz mono
pub const SPEECH_START_RMS_THRESHOLD: f32 = 0.015;
pub const SILENCE_RMS_THRESHOLD: f32 = 0.008;
pub const SILENCE_DURATION_FRAMES: usize = 75; // 75 frames * 20ms = 1500ms (1.5 seconds)
pub const MAX_PRE_SPEECH_FRAMES: usize = 15; // 300ms pre-speech buffer

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpeechState {
    Idle,
    Speaking,
    SilenceDetected,
}

pub fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

#[derive(Debug)]
pub struct SilenceEndpointer {
    state: SpeechState,
    speech_start_threshold: f32,
    silence_threshold: f32,
    silence_frames_threshold: usize,
    silent_frames_count: usize,
    sample_buffer: Vec<f32>,
    pre_speech_frames: Vec<Vec<f32>>,
    frame_size: usize,
    frame_accumulator: Vec<f32>,
}

impl Default for SilenceEndpointer {
    fn default() -> Self {
        Self::new()
    }
}

impl SilenceEndpointer {
    pub fn new() -> Self {
        Self {
            state: SpeechState::Idle,
            speech_start_threshold: SPEECH_START_RMS_THRESHOLD,
            silence_threshold: SILENCE_RMS_THRESHOLD,
            silence_frames_threshold: SILENCE_DURATION_FRAMES,
            silent_frames_count: 0,
            sample_buffer: Vec::new(),
            pre_speech_frames: Vec::new(),
            frame_size: FRAME_SIZE_SAMPLES,
            frame_accumulator: Vec::new(),
        }
    }

    pub fn with_thresholds(
        speech_start: f32,
        silence: f32,
        silence_frames: usize,
        frame_size: usize,
    ) -> Self {
        Self {
            state: SpeechState::Idle,
            speech_start_threshold: speech_start,
            silence_threshold: silence,
            silence_frames_threshold: silence_frames,
            silent_frames_count: 0,
            sample_buffer: Vec::new(),
            pre_speech_frames: Vec::new(),
            frame_size,
            frame_accumulator: Vec::new(),
        }
    }

    pub fn state(&self) -> SpeechState {
        self.state
    }

    pub fn is_silence_detected(&self) -> bool {
        self.state == SpeechState::SilenceDetected
    }

    pub fn is_speaking(&self) -> bool {
        self.state == SpeechState::Speaking
    }

    pub fn sample_buffer(&self) -> &[f32] {
        &self.sample_buffer
    }

    pub fn process_frame(&mut self, frame: &[f32]) -> SpeechState {
        let rms = calculate_rms(frame);

        match self.state {
            SpeechState::Idle => {
                if rms >= self.speech_start_threshold {
                    self.state = SpeechState::Speaking;
                    self.silent_frames_count = 0;

                    // Flush pre-speech buffer to preserve onset consonants
                    for prev_frame in self.pre_speech_frames.drain(..) {
                        self.sample_buffer.extend_from_slice(&prev_frame);
                    }
                    self.sample_buffer.extend_from_slice(frame);
                } else {
                    self.pre_speech_frames.push(frame.to_vec());
                    if self.pre_speech_frames.len() > MAX_PRE_SPEECH_FRAMES {
                        self.pre_speech_frames.remove(0);
                    }
                }
            }
            SpeechState::Speaking => {
                self.sample_buffer.extend_from_slice(frame);
                if rms < self.silence_threshold {
                    self.silent_frames_count += 1;
                    if self.silent_frames_count >= self.silence_frames_threshold {
                        self.state = SpeechState::SilenceDetected;
                    }
                } else {
                    self.silent_frames_count = 0;
                }
            }
            SpeechState::SilenceDetected => {
                // Remain in SilenceDetected state until take_audio_buffer or reset is called
            }
        }

        self.state
    }

    pub fn process_samples(&mut self, samples: &[f32]) -> SpeechState {
        self.frame_accumulator.extend_from_slice(samples);

        while self.frame_accumulator.len() >= self.frame_size {
            let frame: Vec<f32> = self.frame_accumulator.drain(..self.frame_size).collect();
            let state = self.process_frame(&frame);
            if state == SpeechState::SilenceDetected {
                break;
            }
        }

        self.state
    }

    pub fn take_audio_buffer(&mut self) -> Vec<f32> {
        let buffer = std::mem::take(&mut self.sample_buffer);
        self.pre_speech_frames.clear();
        self.frame_accumulator.clear();
        self.silent_frames_count = 0;
        self.state = SpeechState::Idle;
        buffer
    }

    pub fn flush(&mut self) -> Vec<f32> {
        self.take_audio_buffer()
    }

    pub fn reset(&mut self) {
        self.sample_buffer.clear();
        self.pre_speech_frames.clear();
        self.frame_accumulator.clear();
        self.silent_frames_count = 0;
        self.state = SpeechState::Idle;
    }
}

#[cfg(test)]
#[path = "tests/endpointer_tests.rs"]
mod tests;

