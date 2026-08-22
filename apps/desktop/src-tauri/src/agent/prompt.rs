use crate::db::MemoryItem;

pub const DEYANA_PERSONA: &str = r#"You are Deyana — a warm, knowledgeable personal AI assistant.
You speak naturally, like a trusted friend who happens to know everything the user has told you.
You are conversational, present, and never robotic. You use first person naturally.
You never make up facts. You keep responses concise unless asked to elaborate.
All your knowledge of the user comes only from your memory context below.
You never mention data you haven't been explicitly given."#;

pub const MAX_MEMORY_ITEMS: usize = 3;
pub const MAX_ITEM_CONTENT_CHARS: usize = 200;
pub const MAX_TOTAL_CONTEXT_CHARS: usize = 2400; // ~600 tokens

pub fn build_system_prompt(memory_context: &[MemoryItem], tool_info: Option<&str>) -> String {
    let mut prompt = String::from(DEYANA_PERSONA);
    prompt.push_str("\n\n");

    if !memory_context.is_empty() {
        prompt.push_str("### RELEVANT MEMORY CONTEXT ###\n");
        let mut total_chars = 0;

        for (idx, item) in memory_context.iter().take(MAX_MEMORY_ITEMS).enumerate() {
            let truncated_content = if item.content_markdown.chars().count() > MAX_ITEM_CONTENT_CHARS {
                format!("{}...", item.content_markdown.chars().take(MAX_ITEM_CONTENT_CHARS).collect::<String>())
            } else {
                item.content_markdown.clone()
            };

            let block = format!(
                "[{}] {}\nSummary: {}\nContent: {}\n\n",
                idx + 1,
                item.title,
                item.summary,
                truncated_content
            );

            if total_chars + block.len() > MAX_TOTAL_CONTEXT_CHARS && idx > 0 {
                break;
            }

            total_chars += block.len();
            prompt.push_str(&block);
        }
    } else {
        prompt.push_str("### RELEVANT MEMORY CONTEXT ###\nNo relevant prior memory stored for this request.\n\n");
    }

    if let Some(tools) = tool_info {
        prompt.push_str("### AVAILABLE TOOLS ###\n");
        prompt.push_str(tools);
        prompt.push_str("\n\n");
    }

    prompt
}
