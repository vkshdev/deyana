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

    let silent_frame = vec![0.005; 320];
    let state = endpointer.process_frame(&silent_frame);
    assert_eq!(state, SpeechState::Idle);

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

    let silent_frame = vec![0.005; 320];

    for i in 0..74 {
        let state = endpointer.process_frame(&silent_frame);
        assert_eq!(state, SpeechState::Speaking, "Should remain Speaking on frame {}", i + 1);
    }

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
    for _ in 0..50 {
        endpointer.process_frame(&silent_frame);
    }
    assert_eq!(endpointer.state(), SpeechState::Speaking);

    endpointer.process_frame(&active_frame);
    assert_eq!(endpointer.state(), SpeechState::Speaking);

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
    endpointer.process_frame(&silent_frame);

    let active_frame = vec![0.02; 320];
    endpointer.process_frame(&active_frame);

    for _ in 0..75 {
        endpointer.process_frame(&silent_frame);
    }

    assert_eq!(endpointer.state(), SpeechState::SilenceDetected);

    let flushed_audio = endpointer.flush();
    assert_eq!(flushed_audio.len(), 77 * 320);

    assert_eq!(endpointer.state(), SpeechState::Idle);
    assert_eq!(endpointer.sample_buffer().len(), 0);
}

#[test]
fn test_process_samples_arbitrary_chunks() {
    let mut endpointer = SilenceEndpointer::new();

    let active_samples = vec![0.02; 640];
    endpointer.process_samples(&active_samples);
    assert_eq!(endpointer.state(), SpeechState::Speaking);

    let silent_samples = vec![0.005; 75 * 320];
    endpointer.process_samples(&silent_samples);
    assert_eq!(endpointer.state(), SpeechState::SilenceDetected);
}
