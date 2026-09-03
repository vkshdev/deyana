use rusqlite::Connection;
use serde_json::Value;

use crate::db;
use crate::models::hardware::HardwareProfileDetector;

pub struct ToolDispatcher;

impl ToolDispatcher {
    /// List all native tool names supported by the dispatcher
    pub fn available_tools() -> Vec<&'static str> {
        vec![
            "search_memory",
            "save_note",
            "get_system_status",
            "get_triage_inbox",
        ]
    }

    /// Execute a tool invocation by name and argument payload
    pub fn dispatch(
        conn: &Connection,
        tool_name: &str,
        arguments: &Value,
    ) -> Result<String, String> {
        match tool_name {
            "search_memory" => {
                let query = arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let limit = arguments
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(5) as usize;

                let items = db::search_memory_fts(conn, query, limit)
                    .map_err(|e| format!("Memory search failed: {e}"))?;

                serde_json::to_string(&items)
                    .map_err(|e| format!("Serialization error: {e}"))
            }

            "save_note" => {
                let content = arguments
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing required parameter 'content'".to_string())?;

                let tags = arguments
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect::<Vec<String>>()
                    });

                let item = db::save_memory_item(conn, content, "agent_tool", tags)
                    .map_err(|e| format!("Failed to save note: {e}"))?;

                Ok(format!(
                    "Successfully saved note '{}' with ID {}",
                    item.title, item.id
                ))
            }

            "get_system_status" => {
                let ram_gb = HardwareProfileDetector::get_total_ram_gb();
                let profile = HardwareProfileDetector::detect_profile_from_ram(ram_gb);
                let defaults = HardwareProfileDetector::profile_defaults(&profile);

                let status_obj = serde_json::json!({
                    "total_ram_gb": ram_gb,
                    "active_profile": profile,
                    "recommended_chat_model": defaults.recommended_chat_model,
                    "n_ctx_tokens": defaults.n_ctx_tokens,
                });

                serde_json::to_string_pretty(&status_obj)
                    .map_err(|e| format!("Serialization error: {e}"))
            }


            "get_triage_inbox" => {
                let inbox = db::get_triage_inbox(conn)
                    .map_err(|e| format!("Failed to read triage inbox: {e}"))?;

                serde_json::to_string(&inbox)
                    .map_err(|e| format!("Serialization error: {e}"))
            }

            _ => Err(format!("Unknown or unsupported tool: '{tool_name}'")),
        }
    }
}

#[cfg(test)]
#[path = "tests/dispatcher_tests.rs"]
mod tests;


