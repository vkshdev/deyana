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
mod tests {
    use super::*;

    #[test]
    fn test_calculate_rms() {
        let silent = vec![0.0; 320];
        assert_eq!(calculate_rms(&silent), 0.0);

        let constant = vec![0.02; 320];
        let rms = calculate_rms(&constant);
        assert!((rms - 0.02).abs() < 1e-5);
    }

    #[test]
    fn test_speech_start_detection() {
        let mut endpointer = SilenceEndpointer::new();
        assert_eq!(endpointer.state(), SpeechState::Idle);

        // Frame below 0.015 threshold -> Idle
        let silent_frame = vec![0.005; 320];
        let state = endpointer.process_frame(&silent_frame);
        assert_eq!(state, SpeechState::Idle);

        // Frame at/above 0.015 threshold -> Speaking
        let active_frame = vec![0.02; 320];
        let state = endpointer.process_frame(&active_frame);
        assert_eq!(state, SpeechState::Speaking);
    }

    #[test]
    fn test_1_5s_silence_trigger() {
        let mut endpointer = SilenceEndpointer::new();
        let active_frame = vec![0.02; 320];
        endpointer.process_frame(&active_frame);
        assert_eq!(endpointer.state(), SpeechState::Speaking);

        let silent_frame = vec![0.005; 320]; // below 0.008 silence threshold

        // 74 frames of silence (1480ms)
        for i in 0..74 {
            let state = endpointer.process_frame(&silent_frame);
            assert_eq!(state, SpeechState::Speaking, "Should remain Speaking on frame {}", i + 1);
        }

        // 75th frame of continuous silence (1500ms / 1.5s threshold)
        let state = endpointer.process_frame(&silent_frame);
        assert_eq!(state, SpeechState::SilenceDetected);
        assert!(endpointer.is_silence_detected());
    }

    #[test]
    fn test_silence_counter_reset_on_speech_resume() {
        let mut endpointer = SilenceEndpointer::new();
        let active_frame = vec![0.02; 320];
        endpointer.process_frame(&active_frame);

        let silent_frame = vec![0.005; 320];
        // 50 frames of silence
        for _ in 0..50 {
            endpointer.process_frame(&silent_frame);
        }
        assert_eq!(endpointer.state(), SpeechState::Speaking);

        // Resume speech -> resets silent frames count
        endpointer.process_frame(&active_frame);
        assert_eq!(endpointer.state(), SpeechState::Speaking);

        // Needs another full 75 frames of silence to trigger SilenceDetected
        for _ in 0..74 {
            endpointer.process_frame(&silent_frame);
        }
        assert_eq!(endpointer.state(), SpeechState::Speaking);

        endpointer.process_frame(&silent_frame);
        assert_eq!(endpointer.state(), SpeechState::SilenceDetected);
    }

    #[test]
    fn test_audio_buffer_flushing() {
        let mut endpointer = SilenceEndpointer::new();
        let silent_frame = vec![0.005; 320];
        endpointer.process_frame(&silent_frame); // 1 pre-speech frame

        let active_frame = vec![0.02; 320];
        endpointer.process_frame(&active_frame); // triggers Speaking, pre-speech + active in buffer

        for _ in 0..75 {
            endpointer.process_frame(&silent_frame); // 75 silent frames -> SilenceDetected
        }

        assert_eq!(endpointer.state(), SpeechState::SilenceDetected);

        // Flush buffer
        let flushed_audio = endpointer.flush();
        // 1 pre-speech + 1 active + 75 silent = 77 frames * 320 = 24640 samples
        assert_eq!(flushed_audio.len(), 77 * 320);

        // State reset to Idle after flush
        assert_eq!(endpointer.state(), SpeechState::Idle);
        assert_eq!(endpointer.sample_buffer().len(), 0);
    }

    #[test]
    fn test_process_samples_arbitrary_chunks() {
        let mut endpointer = SilenceEndpointer::new();

        // Feed samples in 100-sample chunks
        let active_samples = vec![0.02; 640]; // 2 full 320-sample frames
        endpointer.process_samples(&active_samples);
        assert_eq!(endpointer.state(), SpeechState::Speaking);

        let silent_samples = vec![0.005; 75 * 320]; // 75 frames of silence
        endpointer.process_samples(&silent_samples);
        assert_eq!(endpointer.state(), SpeechState::SilenceDetected);
    }
}
