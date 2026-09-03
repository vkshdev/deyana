use crate::db::MemoryItem;

pub const DEYANA_PERSONA: &str = r#"You are Deyana — a warm, knowledgeable personal AI assistant.
You speak naturally, like a trusted friend who happens to know everything the user has told you.
You are conversational, present, and never robotic. You use first person naturally.
You never make up facts. You keep responses concise unless asked to elaborate.
All your knowledge of the user comes only from your memory context below.
You never mention data you haven't been explicitly given."#;

pub fn calculate_context_budget(n_ctx_tokens: u32) -> (usize, usize, usize) {
    if n_ctx_tokens <= 4096 {
        (3, 200, 2400)
    } else if n_ctx_tokens <= 8192 {
        (6, 500, 8000)
    } else if n_ctx_tokens <= 16384 {
        (12, 1000, 24000)
    } else {
        (30, 2500, 80000)
    }
}


pub fn build_system_prompt(
    memory_context: &[MemoryItem],
    tool_info: Option<&str>,
    n_ctx_tokens: Option<u32>,
) -> String {
    let (max_items, max_item_chars, max_total_chars) =
        calculate_context_budget(n_ctx_tokens.unwrap_or(4096));

    let mut prompt = String::from(DEYANA_PERSONA);
    prompt.push_str("\n\n");

    if !memory_context.is_empty() {
        prompt.push_str("### RELEVANT MEMORY CONTEXT ###\n");
        let mut total_chars = 0;

        for (idx, item) in memory_context.iter().take(max_items).enumerate() {
            let truncated_content = if item.content_markdown.chars().count() > max_item_chars {
                format!(
                    "{}...",
                    item.content_markdown.chars().take(max_item_chars).collect::<String>()
                )
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

            if total_chars + block.len() > max_total_chars && idx > 0 {
                break;
            }

            total_chars += block.len();
            prompt.push_str(&block);
        }
    } else {
        prompt.push_str(
            "### RELEVANT MEMORY CONTEXT ###\nNo relevant prior memory stored for this request.\n\n",
        );
    }

    if let Some(tools) = tool_info {
        prompt.push_str("### AVAILABLE TOOLS ###\n");
        prompt.push_str(tools);
        prompt.push_str("\n\n");
    }

    prompt
}

