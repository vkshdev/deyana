use serde::{Deserialize, Serialize};

/// Standard llama.cpp GBNF grammar for single or multi-turn tool calling.
/// Forces the model output to be strictly valid JSON with either a tool call or a final answer.
pub const STRICT_TOOL_CALL_GBNF: &str = r#"root ::= "{" ws "\"thought\":" ws string "," ws ("\"tool\":" ws string "," ws "\"arguments\":" ws object | "\"final_answer\":" ws string) ws "}"
value  ::= object | array | string | number | ("true" | "false" | "null") ws
object ::= "{" ws (string ":" ws value ("," ws string ":" ws value)*)? ws "}"
array  ::= "[" ws (value ("," ws value)*)? ws "]"
string ::= "\"" ([^"\\] | "\\" (["\\/bfnrt] | "u" [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F]))* "\"" ws
number ::= ("-"? ([0-9] | [1-9] [0-9]*)) ("." [0-9]+)? ([eE] [-+]? [0-9]+)? ws
ws ::= [ \t\n\r]*"#;

/// GBNF grammar enforcing pure JSON object output
pub const STRICT_JSON_OBJECT_GBNF: &str = r#"root ::= object
value  ::= object | array | string | number | ("true" | "false" | "null") ws
object ::= "{" ws (string ":" ws value ("," ws string ":" ws value)*)? ws "}"
array  ::= "[" ws (value ("," ws value)*)? ws "]"
string ::= "\"" ([^"\\] | "\\" (["\\/bfnrt] | "u" [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F]))* "\"" ws
number ::= ("-"? ([0-9] | [1-9] [0-9]*)) ("." [0-9]+)? ([eE] [-+]? [0-9]+)? ws
ws ::= [ \t\n\r]*"#;

/// GBNF grammar enforcing a binary choice or confidence score
pub const CLASSIFICATION_CONFIDENCE_GBNF: &str = r#"root ::= "{" ws "\"decision\":" ws ("\"allow\"" | "\"block\"" | "\"prompt\"") "," ws "\"confidence\":" ws [0-1] ("." [0-9]+)? "," ws "\"reason\":" ws string ws "}"
string ::= "\"" ([^"\\] | "\\" (["\\/bfnrt] | "u" [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F]))* "\"" ws
ws ::= [ \t\n\r]*"#;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCallInvocation {
    pub thought: Option<String>,
    pub tool: Option<String>,
    pub arguments: Option<serde_json::Value>,
    pub final_answer: Option<String>,
}

/// Parse a raw LLM completion into a ToolCallInvocation if it matches structured tool calling format
pub fn parse_tool_call(output: &str) -> Option<ToolCallInvocation> {
    let trimmed = output.trim();
    if !trimmed.starts_with('{') {
        return None;
    }

    // Try direct deserialization
    if let Ok(inv) = serde_json::from_str::<ToolCallInvocation>(trimmed) {
        if inv.tool.is_some() || inv.final_answer.is_some() {
            return Some(inv);
        }
    }

    // Try finding inner JSON substring if enclosed in markdown backticks
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if end > start {
                let json_slice = &trimmed[start..=end];
                if let Ok(inv) = serde_json::from_str::<ToolCallInvocation>(json_slice) {
                    if inv.tool.is_some() || inv.final_answer.is_some() {
                        return Some(inv);
                    }
                }
            }
        }
    }

    None
}

/// Generate custom GBNF grammar constraining the tool name to only registered tools
pub fn generate_tool_whitelist_grammar(allowed_tools: &[&str]) -> String {
    if allowed_tools.is_empty() {
        return STRICT_TOOL_CALL_GBNF.to_string();
    }

    let tool_options: Vec<String> = allowed_tools
        .iter()
        .map(|t| format!("\"\\\"{}\\\"\"", t))
        .collect();
    let tools_choice = tool_options.join(" | ");

    format!(
        r#"root ::= "{{" ws "\"thought\":" ws string "," ws ("\"tool\":" ws tool_name "," ws "\"arguments\":" ws object | "\"final_answer\":" ws string) ws "}}"
tool_name ::= {}
value  ::= object | array | string | number | ("true" | "false" | "null") ws
object ::= "{{" ws (string ":" ws value ("," ws string ":" ws value)*)? ws "}}"
array  ::= "[" ws (value ("," ws value)*)? ws "]"
string ::= "\"" ([^"\\] | "\\" (["\\/bfnrt] | "u" [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F] [0-9a-fA-F]))* "\"" ws
number ::= ("-"? ([0-9] | [1-9] [0-9]*)) ("." [0-9]+)? ([eE] [-+]? [0-9]+)? ws
ws ::= [ \t\n\r]*"#,
        tools_choice
    )
}

#[cfg(test)]
#[path = "tests/grammar_tests.rs"]
mod tests;

