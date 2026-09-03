use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::types::DatasetStats;

#[derive(Debug, Serialize, Deserialize)]
pub struct ShareGptSample {
    pub conversations: Vec<ShareGptMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShareGptMessage {
    pub from: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DpoPreferenceSample {
    pub prompt: String,
    pub chosen: String,
    pub rejected: String,
}

pub fn mask_pii(text: &str) -> String {
    let mut masked = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == 's' && chars.peek() == Some(&'k') {
            chars.next();
            if chars.peek() == Some(&'-') {
                chars.next();
                let mut skipped = 0;
                while let Some(&next_c) = chars.peek() {
                    if next_c.is_alphanumeric() || next_c == '_' || next_c == '-' {
                        chars.next();
                        skipped += 1;
                    } else {
                        break;
                    }
                }
                if skipped >= 10 {
                    masked.push_str("[API_KEY]");
                } else {
                    masked.push_str("sk-");
                }
            } else {
                masked.push('s');
                masked.push('k');
            }
        } else if c == 'g' && chars.peek() == Some(&'h') {
            chars.next();
            if chars.peek() == Some(&'p') || chars.peek() == Some(&'o') {
                chars.next();
                if chars.peek() == Some(&'_') {
                    chars.next();
                    while let Some(&next_c) = chars.peek() {
                        if next_c.is_alphanumeric() {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    masked.push_str("[GITHUB_TOKEN]");
                } else {
                    masked.push_str("gh");
                }
            } else {
                masked.push('g');
                masked.push('h');
            }
        } else {
            masked.push(c);
        }
    }

    let mut words: Vec<String> = Vec::new();
    for word in masked.split_whitespace() {
        if word.contains('@') && word.contains('.') && !word.starts_with("http") {
            words.push("[EMAIL]".to_string());
        } else {
            words.push(word.to_string());
        }
    }
    words.join(" ")
}


/// Automatically log an interaction into the `training_dataset` table
pub fn log_interaction(
    conn: &Connection,
    instruction: &str,
    input_context: &str,
    output: &str,
    model_used: &str,
    window_title: Option<&str>,
    session_id: Option<&str>,
) -> Result<String, rusqlite::Error> {
    let id = format!("train_{}", Uuid::new_v4().simple());
    let now = chrono::Utc::now().to_rfc3339();

    let masked_instruction = mask_pii(instruction);
    let masked_context = mask_pii(input_context);
    let masked_output = mask_pii(output);

    conn.execute(
        "INSERT INTO training_dataset (id, instruction, input_context, output, model_used, window_title, feedback, session_id, pii_masked, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, 1, ?8)",
        params![
            &id,
            &masked_instruction,
            &masked_context,
            &masked_output,
            model_used,
            window_title,
            session_id,
            &now,
        ],
    )?;

    Ok(id)
}

/// Record user feedback (+1 thumbs up, -1 thumbs down) and optional user edit correction
pub fn record_feedback(
    conn: &Connection,
    id: &str,
    feedback: i32,
    user_edit: Option<&str>,
) -> Result<(), rusqlite::Error> {
    let masked_edit = user_edit.map(mask_pii);
    conn.execute(
        "UPDATE training_dataset SET feedback = ?1, user_edit = ?2 WHERE id = ?3",
        params![feedback, masked_edit, id],
    )?;
    Ok(())
}

/// Query dataset statistics for telemetry / settings UI
pub fn get_dataset_stats(conn: &Connection) -> Result<DatasetStats, rusqlite::Error> {
    let total_samples: usize = conn
        .query_row("SELECT COUNT(*) FROM training_dataset", [], |r| r.get(0))
        .unwrap_or(0);

    let thumbs_up_count: usize = conn
        .query_row("SELECT COUNT(*) FROM training_dataset WHERE feedback > 0", [], |r| r.get(0))
        .unwrap_or(0);

    let thumbs_down_count: usize = conn
        .query_row("SELECT COUNT(*) FROM training_dataset WHERE feedback < 0", [], |r| r.get(0))
        .unwrap_or(0);

    let edited_samples_count: usize = conn
        .query_row("SELECT COUNT(*) FROM training_dataset WHERE user_edit IS NOT NULL AND user_edit != ''", [], |r| r.get(0))
        .unwrap_or(0);

    let mut stmt = conn.prepare(
        "SELECT model_used, COUNT(*) FROM training_dataset GROUP BY model_used"
    )?;

    let mut models_breakdown = HashMap::new();
    let rows = stmt.query_map([], |row| {
        let model: String = row.get(0)?;
        let count: usize = row.get(1)?;
        Ok((model, count))
    })?;

    for r in rows {
        if let Ok((m, c)) = r {
            models_breakdown.insert(m, c);
        }
    }

    Ok(DatasetStats {
        total_samples,
        thumbs_up_count,
        thumbs_down_count,
        edited_samples_count,
        models_breakdown,
    })
}

/// Export dataset into ShareGPT format JSONL (for Axolotl, Unsloth, LLaMA-Factory SFT)
pub fn export_sharegpt(conn: &Connection, output_path: &Path) -> Result<usize, String> {
    let mut stmt = conn
        .prepare("SELECT instruction, input_context, output, user_edit FROM training_dataset WHERE feedback >= 0 ORDER BY created_at ASC")
        .map_err(|e| e.to_string())?;

    let mut file = File::create(output_path).map_err(|e| e.to_string())?;
    let mut count = 0;

    let rows = stmt
        .query_map([], |row| {
            let instruction: String = row.get(0)?;
            let input_context: String = row.get(1)?;
            let output: String = row.get(2)?;
            let user_edit: Option<String> = row.get(3)?;

            let target_output = user_edit.unwrap_or(output);
            let human_msg = if input_context.is_empty() {
                instruction
            } else {
                format!("{input_context}\n\n{instruction}")
            };

            Ok(ShareGptSample {
                conversations: vec![
                    ShareGptMessage {
                        from: "human".to_string(),
                        value: human_msg,
                    },
                    ShareGptMessage {
                        from: "gpt".to_string(),
                        value: target_output,
                    },
                ],
            })
        })
        .map_err(|e| e.to_string())?;

    for sample_res in rows {
        let sample = sample_res.map_err(|e| e.to_string())?;
        let line = serde_json::to_string(&sample).map_err(|e| e.to_string())?;
        writeln!(file, "{line}").map_err(|e| e.to_string())?;
        count += 1;
    }

    file.flush().map_err(|e| e.to_string())?;
    Ok(count)
}

/// Export dataset into DPO / ORPO preference format JSONL (for Direct Preference Optimization)
pub fn export_dpo(conn: &Connection, output_path: &Path) -> Result<usize, String> {
    let mut stmt = conn
        .prepare("SELECT instruction, output, user_edit FROM training_dataset WHERE user_edit IS NOT NULL AND user_edit != '' ORDER BY created_at ASC")
        .map_err(|e| e.to_string())?;

    let mut file = File::create(output_path).map_err(|e| e.to_string())?;
    let mut count = 0;

    let rows = stmt
        .query_map([], |row| {
            let prompt: String = row.get(0)?;
            let rejected: String = row.get(1)?;
            let chosen: String = row.get(2)?;

            Ok(DpoPreferenceSample {
                prompt,
                chosen,
                rejected,
            })
        })
        .map_err(|e| e.to_string())?;

    for sample_res in rows {
        let sample = sample_res.map_err(|e| e.to_string())?;
        let line = serde_json::to_string(&sample).map_err(|e| e.to_string())?;
        writeln!(file, "{line}").map_err(|e| e.to_string())?;
        count += 1;
    }

    file.flush().map_err(|e| e.to_string())?;
    Ok(count)
}

/// Clear dataset rows
pub fn clear_training_dataset(conn: &Connection) -> Result<usize, rusqlite::Error> {
    conn.execute("DELETE FROM training_dataset", [])
}

#[cfg(test)]
#[path = "tests/dataset_tests.rs"]
mod tests;

