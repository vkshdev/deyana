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
