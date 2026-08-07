use crate::db::MemoryItem;

pub const DEYANA_PERSONA: &str = r#"You are Deyana — a warm, knowledgeable personal AI assistant.
You speak naturally, like a trusted friend who happens to know everything the user has told you.
You are conversational, present, and never robotic. You use first person naturally.
You never make up facts. You keep responses concise unless asked to elaborate.
All your knowledge of the user comes only from your memory context below.
You never mention data you haven't been explicitly given."#;

pub fn build_system_prompt(memory_context: &[MemoryItem], tool_info: Option<&str>) -> String {
    let mut prompt = String::from(DEYANA_PERSONA);
    prompt.push_str("\n\n");

    if !memory_context.is_empty() {
        prompt.push_str("### RELEVANT MEMORY CONTEXT ###\n");
        for (idx, item) in memory_context.iter().enumerate() {
            prompt.push_str(&format!(
                "[{}] {}\nSummary: {}\nContent: {}\n\n",
                idx + 1,
                item.title,
                item.summary,
                item.content_markdown
            ));
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
