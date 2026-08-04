use std::path::{Path, PathBuf};
use std::process::Command;
use chrono::Utc;
use reqwest::Client;
use uuid::Uuid;

use crate::db::DbPool;
use crate::privacy::firewall::{
    PrivacyAuditEvent, PrivacyCheckRequest, PrivacyCheckResponse, PrivacyFirewall, PrivacyRules,
};
use super::types::*;

pub struct ToolService {
    pub db_pool: DbPool,
    pub http_client: Client,
}

impl ToolService {
    pub fn new(db_pool: DbPool) -> Self {
        Self {
            db_pool,
            http_client: Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn list_tools(&self) -> ToolListResponse {
        ToolListResponse {
            tools: vec![
                ToolManifest {
                    tool_id: "web_search".to_string(),
                    name: "Web search".to_string(),
                    description: "Search the public web with only the user's explicit public query.".to_string(),
                    risk: "public_web".to_string(),
                    requires_approval: true,
                    dangerous: false,
                    applies_changes: false,
                },
                ToolManifest {
                    tool_id: "fetch_page".to_string(),
                    name: "Fetch page".to_string(),
                    description: "Fetch and summarize a public webpage without sending private memory.".to_string(),
                    risk: "public_web".to_string(),
                    requires_approval: true,
                    dangerous: false,
                    applies_changes: false,
                },
                ToolManifest {
                    tool_id: "read_file".to_string(),
                    name: "Read file".to_string(),
                    description: "Read a file only when it is inside an approved folder root.".to_string(),
                    risk: "local_file".to_string(),
                    requires_approval: true,
                    dangerous: false,
                    applies_changes: false,
                },
                ToolManifest {
                    tool_id: "git_status".to_string(),
                    name: "Git status".to_string(),
                    description: "Read git status for an approved repository path.".to_string(),
                    risk: "source_code".to_string(),
                    requires_approval: true,
                    dangerous: false,
                    applies_changes: false,
                },
                ToolManifest {
                    tool_id: "git_diff".to_string(),
                    name: "Git diff".to_string(),
                    description: "Read git diff for an approved repository path.".to_string(),
                    risk: "source_code".to_string(),
                    requires_approval: true,
                    dangerous: false,
                    applies_changes: false,
                },
                ToolManifest {
                    tool_id: "commit_message".to_string(),
                    name: "Commit message".to_string(),
                    description: "Suggest a commit message from local git status and diff without committing.".to_string(),
                    risk: "source_code".to_string(),
                    requires_approval: true,
                    dangerous: false,
                    applies_changes: false,
                },
                ToolManifest {
                    tool_id: "code_task".to_string(),
                    name: "Code task".to_string(),
                    description: "Explain code and propose changes before any edits are applied.".to_string(),
                    risk: "source_code".to_string(),
                    requires_approval: true,
                    dangerous: false,
                    applies_changes: false,
                },
                ToolManifest {
                    tool_id: "day_planner".to_string(),
                    name: "Day planner".to_string(),
                    description: "Create a local day plan from user-provided commitments and local action items.".to_string(),
                    risk: "low".to_string(),
                    requires_approval: false,
                    dangerous: false,
                    applies_changes: false,
                },
            ],
        }
    }

    pub async fn web_search(&self, request: WebSearchRequest) -> Result<ToolRunResponse, String> {
        if !request.user_approved.unwrap_or(false) {
            return Ok(permission_required("web_search", "Public web search requires approval."));
        }
        let query = normalize_space(&request.query);
        if query.is_empty() {
            return Err("Search query is required.".to_string());
        }

        let url = format!("https://www.bing.com/search?q={}&format=rss", url_encode(&query));
        let privacy = self.evaluate_privacy(&url, "GET", "public_web_fetch", "public_query", &query);

        let response = self
            .http_client
            .get(&url)
            .header("user-agent", "DEYANA-local-tool/0.1")
            .header("accept", "application/rss+xml,application/xml,text/xml")
            .send()
            .await
            .map_err(|e| format!("Tool request failed: {}", e))?;

        let rss_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read search response: {}", e))?;

        let limit = request.limit.unwrap_or(5);
        let items = parse_bing_rss_results(&rss_text, limit);
        let summary = format!("Found {} public results for: {}", items.len(), query);
        let content = items
            .iter()
            .map(|item| format!("- {}: {}", item.title, item.url.as_deref().unwrap_or("")))
            .collect::<Vec<_>>()
            .join("\n");
        let content_final = if content.is_empty() { "No results found.".to_string() } else { content };

        Ok(ToolRunResponse {
            tool_id: "web_search".to_string(),
            status: "completed".to_string(),
            title: "Web search results".to_string(),
            summary,
            content: content_final,
            items,
            permission_required: false,
            confirmation_required: false,
            applies_changes: false,
            privacy: Some(privacy),
        })
    }

    pub async fn fetch_page(&self, request: WebFetchRequest) -> Result<ToolRunResponse, String> {
        if !request.user_approved.unwrap_or(false) {
            return Ok(permission_required("fetch_page", "Public webpage fetch requires approval."));
        }

        let privacy = self.evaluate_privacy(
            &request.url,
            "GET",
            "public_web_fetch",
            "public_content",
            "Public webpage fetch",
        );

        let response = self
            .http_client
            .get(&request.url)
            .header("user-agent", "DEYANA-local-tool/0.1")
            .header("accept", "text/html,text/plain")
            .send()
            .await
            .map_err(|e| format!("Tool request failed: {}", e))?;

        let raw = response
            .text()
            .await
            .map_err(|e| format!("Failed to read webpage response: {}", e))?;

        let text = html_to_text(&raw);
        let max_chars = request.max_characters.unwrap_or(8000);
        let content = truncate(&text, max_chars);
        let title = first_line(&content);
        let summary = compact_sentence(&content, 280);

        Ok(ToolRunResponse {
            tool_id: "fetch_page".to_string(),
            status: "completed".to_string(),
            title: if title.is_empty() { "Fetched webpage".to_string() } else { title },
            summary: if summary.is_empty() { "Fetched public webpage content.".to_string() } else { summary },
            content,
            items: vec![],
            permission_required: false,
            confirmation_required: false,
            applies_changes: false,
            privacy: Some(privacy),
        })
    }

    pub fn read_file(&self, request: FileReadRequest) -> Result<ToolRunResponse, String> {
        if !request.user_approved.unwrap_or(false) {
            return Ok(permission_required("read_file", "Reading a local file requires folder approval."));
        }

        let file_path = resolve_inside_root(&request.path, &request.allowed_root)?;
        if !file_path.is_file() {
            return Err("Approved path is not a readable file.".to_string());
        }

        let raw_content = std::fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let max_chars = request.max_characters.unwrap_or(8000);
        let content = truncate(&raw_content, max_chars);
        let filename = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());

        Ok(ToolRunResponse {
            tool_id: "read_file".to_string(),
            status: "completed".to_string(),
            title: filename,
            summary: format!("Read {} characters from an approved local file.", content.len()),
            content,
            items: vec![],
            permission_required: false,
            confirmation_required: false,
            applies_changes: false,
            privacy: None,
        })
    }

    pub fn git_status(&self, request: GitReadRequest) -> Result<ToolRunResponse, String> {
        if !request.user_approved.unwrap_or(false) {
            return Ok(permission_required("git_status", "Reading git status requires repository approval."));
        }

        let repo = require_git_repo(&request.repo_path)?;
        let max_chars = request.max_characters.unwrap_or(8000);
        let output = run_git(&repo, &["status", "--short"], max_chars)?;

        Ok(ToolRunResponse {
            tool_id: "git_status".to_string(),
            status: "completed".to_string(),
            title: "Git status".to_string(),
            summary: git_status_summary(&output),
            content: if output.is_empty() { "Working tree clean.".to_string() } else { output },
            items: vec![],
            permission_required: false,
            confirmation_required: false,
            applies_changes: false,
            privacy: None,
        })
    }

    pub fn git_diff(&self, request: GitReadRequest) -> Result<ToolRunResponse, String> {
        if !request.user_approved.unwrap_or(false) {
            return Ok(permission_required("git_diff", "Reading git diff requires repository approval."));
        }

        let repo = require_git_repo(&request.repo_path)?;
        let max_chars = request.max_characters.unwrap_or(8000);
        let output = run_git(&repo, &["diff", "--", "."], max_chars)?;

        Ok(ToolRunResponse {
            tool_id: "git_diff".to_string(),
            status: "completed".to_string(),
            title: "Git diff summary".to_string(),
            summary: diff_summary(&output),
            content: if output.is_empty() { "No unstaged diff.".to_string() } else { output },
            items: vec![],
            permission_required: false,
            confirmation_required: false,
            applies_changes: false,
            privacy: None,
        })
    }

    pub fn commit_message(&self, request: GitReadRequest) -> Result<ToolRunResponse, String> {
        if !request.user_approved.unwrap_or(false) {
            return Ok(permission_required("commit_message", "Commit message suggestion requires repository approval."));
        }

        let repo = require_git_repo(&request.repo_path)?;
        let max_chars = request.max_characters.unwrap_or(8000);
        let status = run_git(&repo, &["status", "--short"], max_chars)?;
        let diff = run_git(&repo, &["diff", "--stat"], max_chars)?;

        let message = suggest_commit_message(&status, &diff);
        let rationale = if !diff.is_empty() {
            diff
        } else if !status.is_empty() {
            status
        } else {
            "No local changes detected.".to_string()
        };
        let content = format!("{}\n\nRationale:\n{}", message, rationale);

        Ok(ToolRunResponse {
            tool_id: "commit_message".to_string(),
            status: "completed".to_string(),
            title: "Suggested commit message".to_string(),
            summary: message,
            content,
            items: vec![],
            permission_required: false,
            confirmation_required: false,
            applies_changes: false,
            privacy: None,
        })
    }

    pub fn code_task(&self, request: CodeTaskRequest) -> Result<ToolRunResponse, String> {
        if !request.user_approved.unwrap_or(false) {
            return Ok(permission_required("code_task", "Coding explanation/proposal requires approval for source context."));
        }

        let goal = normalize_space(&request.goal);
        let context = truncate(request.context.as_deref().unwrap_or(""), 8000);
        let proposal = build_code_proposal(&goal, &context);

        Ok(ToolRunResponse {
            tool_id: "code_task".to_string(),
            status: "completed".to_string(),
            title: "Coding proposal".to_string(),
            summary: "Generated a local proposal only; no files were changed.".to_string(),
            content: proposal,
            items: vec![],
            permission_required: false,
            confirmation_required: false,
            applies_changes: false,
            privacy: None,
        })
    }

    pub fn day_planner(&self, request: DayPlannerRequest) -> Result<ToolRunResponse, String> {
        let date = request
            .date
            .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());

        let mut action_items: Vec<String> = Vec::new();
        if let Ok(conn) = self.db_pool.get() {
            let stmt = conn.prepare(
                "SELECT title, due_at FROM memory_insights WHERE type = 'action_item' AND status = 'open' LIMIT 12"
            );
            if let Ok(mut stmt) = stmt {
                let rows = stmt.query_map([], |row| {
                    let title: String = row.get(0)?;
                    let due_at: Option<String> = row.get(1)?;
                    Ok((title, due_at))
                });
                if let Ok(mapped) = rows {
                    for item in mapped.flatten() {
                        let due = item.1.map(|d| format!(" Due: {}.", d)).unwrap_or_default();
                        action_items.push(format!("- {}{}", item.0, due));
                    }
                }
            }
        }

        let mut lines = vec![format!("# Day plan - {}", date), "".to_string(), "## Focus blocks".to_string(), "".to_string()];
        let raw_focus = request.focus.unwrap_or_default();
        let mut focus: Vec<String> = raw_focus
            .iter()
            .map(|item| normalize_space(item))
            .filter(|item| !item.is_empty())
            .collect();

        if focus.is_empty() {
            focus = vec![
                "Review priority work".to_string(),
                "Clear open action items".to_string(),
                "Plan tomorrow".to_string(),
            ];
        }

        for (idx, item) in focus.iter().take(5).enumerate() {
            lines.push(format!("{}. {}", idx + 1, item));
        }

        let raw_commitments = request.commitments.unwrap_or_default();
        let commitments: Vec<String> = raw_commitments
            .iter()
            .map(|item| normalize_space(item))
            .filter(|item| !item.is_empty())
            .collect();

        if !commitments.is_empty() {
            lines.push("".to_string());
            lines.push("## Commitments".to_string());
            lines.push("".to_string());
            for item in commitments.iter().take(10) {
                lines.push(format!("- {}", item));
            }
        }

        if !action_items.is_empty() {
            lines.push("".to_string());
            lines.push("## Local action items".to_string());
            lines.push("".to_string());
            lines.extend(action_items.iter().take(8).cloned());
        }

        if let Some(notes) = request.notes {
            let trimmed = notes.trim();
            if !trimmed.is_empty() {
                lines.push("".to_string());
                lines.push("## Notes".to_string());
                lines.push("".to_string());
                lines.push(trimmed.to_string());
            }
        }

        let summary = format!(
            "Local day plan with {} focus blocks and {} memory action items.",
            focus.len(),
            action_items.len()
        );

        Ok(ToolRunResponse {
            tool_id: "day_planner".to_string(),
            status: "completed".to_string(),
            title: format!("Day plan - {}", date),
            summary,
            content: lines.join("\n"),
            items: vec![],
            permission_required: false,
            confirmation_required: false,
            applies_changes: false,
            privacy: None,
        })
    }

    fn evaluate_privacy(
        &self,
        url: &str,
        method: &str,
        purpose: &str,
        data_category: &str,
        payload_preview: &str,
    ) -> PrivacyCheckResponse {
        let rules = PrivacyRules::default();
        let req = PrivacyCheckRequest {
            url: url.to_string(),
            method: Some(method.to_string()),
            purpose: Some(purpose.to_string()),
            data_category: Some(data_category.to_string()),
            payload_preview: Some(payload_preview.to_string()),
            user_approved: Some(true),
            connector_id: None,
            external_write: None,
        };

        let (
            decision,
            reason,
            destination,
            destination_category,
            data_cat,
            purp,
            meth,
            risk_level,
            safe_alternative,
            payload_sha256,
            payload_character_count,
        ) = PrivacyFirewall::evaluate(&rules, &req);

        let event_type = if decision == "block" {
            "privacy.request.blocked".to_string()
        } else {
            "privacy.request.allowed".to_string()
        };

        PrivacyCheckResponse {
            allowed: decision == "allow",
            decision: decision.clone(),
            reason: reason.clone(),
            destination: destination.clone(),
            destination_category: destination_category.clone(),
            data_category: data_cat.clone(),
            purpose: purp.clone(),
            safe_alternative: safe_alternative.clone(),
            audit_event: PrivacyAuditEvent {
                id: format!("privacy_{}", Uuid::new_v4().simple()),
                event_type,
                decision,
                reason,
                destination,
                destination_category,
                data_category: data_cat,
                purpose: purp,
                method: meth,
                risk_level,
                user_approved: true,
                connector_id: None,
                safe_alternative,
                payload_sha256,
                payload_character_count,
                created_at: Utc::now().to_rfc3339(),
            },
        }
    }
}

fn permission_required(tool_id: &str, message: &str) -> ToolRunResponse {
    ToolRunResponse {
        tool_id: tool_id.to_string(),
        status: "permission_required".to_string(),
        title: "Permission required".to_string(),
        summary: message.to_string(),
        content: message.to_string(),
        items: vec![],
        permission_required: true,
        confirmation_required: false,
        applies_changes: false,
        privacy: None,
    }
}

fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }
    encoded
}

fn parse_bing_rss_results(content: &str, limit: usize) -> Vec<ToolResultItem> {
    let mut results = Vec::new();
    let mut search_pos = 0;
    while let Some(start_idx) = content[search_pos..].find("<item>") {
        let item_start = search_pos + start_idx + 6;
        if let Some(end_idx) = content[item_start..].find("</item>") {
            let item_str = &content[item_start..item_start + end_idx];
            let title = extract_xml_tag(item_str, "title").unwrap_or_default();
            let link = extract_xml_tag(item_str, "link").unwrap_or_default();
            let desc = extract_xml_tag(item_str, "description").unwrap_or_default();

            let clean_title = normalize_space(&title);
            let clean_link = normalize_space(&link);
            let clean_summary = html_to_text(&desc);
            let final_summary = if clean_summary.is_empty() {
                clean_title.clone()
            } else {
                clean_summary
            };

            if !clean_title.is_empty() && !clean_link.is_empty() {
                results.push(ToolResultItem {
                    title: clean_title,
                    summary: final_summary,
                    url: Some(clean_link),
                    source: Some("bing".to_string()),
                });
            }

            search_pos = item_start + end_idx + 7;
            if results.len() >= limit {
                break;
            }
        } else {
            break;
        }
    }
    results
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);
    if let Some(start_idx) = xml.find(&start_tag) {
        let content_start = start_idx + start_tag.len();
        if let Some(end_idx) = xml[content_start..].find(&end_tag) {
            return Some(xml[content_start..content_start + end_idx].trim().to_string());
        }
    }
    None
}

fn html_to_text(content: &str) -> String {
    let mut text = content.to_string();
    while let Some(s) = text.find("<script") {
        if let Some(e) = text[s..].find("</script>") {
            text.replace_range(s..s + e + 9, " ");
        } else {
            break;
        }
    }
    while let Some(s) = text.find("<style") {
        if let Some(e) = text[s..].find("</style>") {
            text.replace_range(s..s + e + 8, " ");
        } else {
            break;
        }
    }

    text = text.replace("<br>", "\n").replace("<br/>", "\n").replace("<br />", "\n");
    let tag_ends = ["</p>", "</div>", "</li>", "</h1>", "</h2>", "</h3>", "4>", "5>", "6>"];
    for end in tag_ends {
        text = text.replace(end, "\n");
    }

    let mut result = String::new();
    let mut inside = false;
    for c in text.chars() {
        if c == '<' {
            inside = true;
            result.push(' ');
        } else if c == '>' {
            inside = false;
        } else if !inside {
            result.push(c);
        }
    }

    let unescaped = result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    normalize_space(&unescaped)
}

fn resolve_inside_root(path: &str, allowed_root: &str) -> Result<PathBuf, String> {
    let root = Path::new(allowed_root)
        .canonicalize()
        .map_err(|e| format!("Invalid allowed root path: {}", e))?;
    let target = Path::new(path)
        .canonicalize()
        .map_err(|e| format!("Invalid file path: {}", e))?;
    if target.starts_with(&root) {
        Ok(target)
    } else {
        Err("File is outside the approved folder root.".to_string())
    }
}

fn require_git_repo(repo_path: &str) -> Result<PathBuf, String> {
    let repo = Path::new(repo_path)
        .canonicalize()
        .map_err(|e| format!("Invalid repository path: {}", e))?;
    if !repo.join(".git").exists() {
        return Err("Approved path is not a git repository.".to_string());
    }
    Ok(repo)
}

fn run_git(repo: &Path, args: &[&str], max_characters: usize) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| format!("Git command execution failed: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    let combined = if !stdout.is_empty() { stdout } else { stderr };
    if !output.status.success() {
        return Err(if combined.is_empty() {
            "Git command failed.".to_string()
        } else {
            combined
        });
    }

    Ok(truncate(&combined, max_characters))
}

fn git_status_summary(output: &str) -> String {
    if output.trim().is_empty() {
        return "Working tree clean.".to_string();
    }
    let lines = output.lines().filter(|l| !l.trim().is_empty()).count();
    format!("{} changed paths in working tree.", lines)
}

fn diff_summary(output: &str) -> String {
    if output.trim().is_empty() {
        return "No unstaged diff.".to_string();
    }
    let additions = output
        .lines()
        .filter(|line| line.starts_with('+') && !line.starts_with("+++"))
        .count();
    let deletions = output
        .lines()
        .filter(|line| line.starts_with('-') && !line.starts_with("---"))
        .count();
    format!(
        "Unstaged diff has about {} additions and {} deletions.",
        additions, deletions
    )
}

fn suggest_commit_message(status: &str, diff_stat: &str) -> String {
    let text = format!("{}\n{}", status, diff_stat).to_lowercase();
    if text.contains("connector") {
        "feat(connectors): expand local connector sync support".to_string()
    } else if text.contains("tool") {
        "feat(tools): add permissioned local assistant tools".to_string()
    } else if text.contains("memory") {
        "feat(memory): improve local memory workflow".to_string()
    } else if !status.trim().is_empty() {
        "chore: update local assistant implementation".to_string()
    } else {
        "chore: no local changes detected".to_string()
    }
}

fn build_code_proposal(goal: &str, context: &str) -> String {
    let goal_str = if goal.is_empty() {
        "Explain or improve the provided code."
    } else {
        goal
    };
    let context_summary = if !context.is_empty() {
        compact_sentence(context, 900)
    } else {
        "No source context was provided.".to_string()
    };

    vec![
        "# Coding proposal".to_string(),
        "".to_string(),
        format!("Goal: {}", goal_str),
        "".to_string(),
        "## Explanation".to_string(),
        "".to_string(),
        context_summary,
        "".to_string(),
        "## Proposed change plan".to_string(),
        "".to_string(),
        "1. Identify the smallest production path that satisfies the goal.".to_string(),
        "2. Keep business logic outside UI code and preserve existing architecture boundaries.".to_string(),
        "3. Update tests/docs beside the implementation.".to_string(),
        "4. Apply edits only after explicit user confirmation.".to_string(),
    ]
    .join("\n")
}

fn first_line(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            return compact_sentence(trimmed, 120);
        }
    }
    String::new()
}

fn compact_sentence(value: &str, limit: usize) -> String {
    let normalized = normalize_space(value);
    if normalized.len() <= limit {
        normalized
    } else {
        let truncated = normalized[..limit - 1].trim_end_matches(&[' ', ',', '.', ';', ':'][..]);
        format!("{}.", truncated)
    }
}

fn normalize_space(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate(value: &str, limit: usize) -> String {
    if value.len() <= limit {
        value.to_string()
    } else {
        format!("{}\n\n[truncated]", value[..limit].trim_end())
    }
}
