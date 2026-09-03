use super::*;

#[test]
fn test_ram_tier_mapping() {
    assert_eq!(HardwareProfileDetector::detect_profile_from_ram(7), ModelProfile::Balanced);
    assert_eq!(HardwareProfileDetector::detect_profile_from_ram(8), ModelProfile::Balanced);
    assert_eq!(HardwareProfileDetector::detect_profile_from_ram(15), ModelProfile::Balanced);
    assert_eq!(HardwareProfileDetector::detect_profile_from_ram(16), ModelProfile::Standard);
    assert_eq!(HardwareProfileDetector::detect_profile_from_ram(31), ModelProfile::Standard);
    assert_eq!(HardwareProfileDetector::detect_profile_from_ram(32), ModelProfile::Power);
    assert_eq!(HardwareProfileDetector::detect_profile_from_ram(63), ModelProfile::Power);
    assert_eq!(HardwareProfileDetector::detect_profile_from_ram(64), ModelProfile::Ultra);
    assert_eq!(HardwareProfileDetector::detect_profile_from_ram(128), ModelProfile::Ultra);
}

#[test]
fn test_profile_defaults() {
    let balanced = HardwareProfileDetector::profile_defaults(&ModelProfile::Balanced);
    assert_eq!(balanced.recommended_chat_model, "qwen3:1.7b");
    assert_eq!(balanced.n_ctx_tokens, 4096);
    assert!(!balanced.always_on_voice);

    let ultra = HardwareProfileDetector::profile_defaults(&ModelProfile::Ultra);
    assert_eq!(ultra.recommended_chat_model, "llama3.3:70b");
    assert_eq!(ultra.n_ctx_tokens, 65536);
    assert!(ultra.always_on_voice);
}

#[test]
fn test_system_ram_detection_returns_positive() {
    let ram = HardwareProfileDetector::get_total_ram_gb();
    assert!(ram > 0, "Expected system RAM to be > 0 GB, got {ram}");
}
