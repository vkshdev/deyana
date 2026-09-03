use super::*;

#[test]
fn test_parse_valid_tool_call() {
    let json = r#"{"thought": "Need to look up note", "tool": "search_memory", "arguments": {"query": "Rust"}}"#;
    let parsed = parse_tool_call(json).unwrap();
    assert_eq!(parsed.thought.as_deref(), Some("Need to look up note"));
    assert_eq!(parsed.tool.as_deref(), Some("search_memory"));
    assert!(parsed.arguments.is_some());
    assert_eq!(parsed.final_answer, None);
}

#[test]
fn test_parse_valid_final_answer() {
    let json = r#"{"thought": "I have the answer", "final_answer": "Rust is an ultra-fast systems language."}"#;
    let parsed = parse_tool_call(json).unwrap();
    assert_eq!(parsed.tool, None);
    assert_eq!(parsed.final_answer.as_deref(), Some("Rust is an ultra-fast systems language."));
}

#[test]
fn test_tool_whitelist_grammar_generation() {
    let grammar = generate_tool_whitelist_grammar(&["search_memory", "save_note", "get_status"]);
    assert!(grammar.contains("\"\\\"search_memory\\\"\""));
    assert!(grammar.contains("\"\\\"save_note\\\"\""));
    assert!(grammar.contains("\"\\\"get_status\\\"\""));
}
