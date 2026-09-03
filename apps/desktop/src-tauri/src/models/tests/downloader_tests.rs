use super::*;

#[test]
fn test_extract_quantization() {
    assert_eq!(extract_quantization("Llama-3.3-70B-Instruct-Q4_K_M.gguf"), "Q4_K_M");
    assert_eq!(extract_quantization("qwen2.5-7b-instruct-q8_0.gguf"), "Q8_0");
    assert_eq!(extract_quantization("mistral-7b-v0.1.Q3_K_L.gguf"), "Q3_K_L");
}

#[test]
fn test_estimate_ram_from_quant() {
    let ram_70b_q4 = estimate_ram_from_quant("Llama-3.3-70B-Instruct-Q4_K_M.gguf", "Q4_K_M");
    assert!(ram_70b_q4 >= 35.0 && ram_70b_q4 <= 55.0);

    let ram_7b_q4 = estimate_ram_from_quant("qwen2.5-7b-instruct-q4_k_m.gguf", "Q4_K_M");
    assert!(ram_7b_q4 >= 4.0 && ram_7b_q4 <= 8.0);
}
