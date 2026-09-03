use std::collections::HashMap;
use rusqlite::Connection;
use crate::db::{self, MemoryItem};

/// Retrieve relevant memory items using dual-mode retrieval with pre-filtering:
/// 1. SQLite FTS5 lexical pre-filtering to identify candidate items.
/// 2. Selective vector embedding loading and cosine similarity scoring on candidates.
/// 3. Recency fallback to guarantee context availability.
/// Combined score = 0.5 * FTS_score + 0.5 * Vec_similarity_score + 0.5 * importance.
pub fn retrieve_memory_context(
    conn: &Connection,
    query_embedding: Option<&[f32]>,
    query: &str,
    limit: usize,
) -> Result<Vec<MemoryItem>, rusqlite::Error> {
    let recent_items = db::get_memory_items(conn, Some(30))?;
    if recent_items.is_empty() || query.trim().is_empty() {
        return Ok(recent_items.into_iter().take(limit).collect());
    }

    let mut scores: HashMap<String, f32> = HashMap::new();
    let mut candidate_ids: Vec<String> = Vec::new();

    // 1. Lexical Search (SQLite FTS5 matching, fallback to substring word matching)
    let fts_results = db::search_memory_fts(conn, query, 30).unwrap_or_default();
    if !fts_results.is_empty() {
        for (id, fts_score) in fts_results {
            candidate_ids.push(id.clone());
            scores.insert(id, fts_score * 0.5);
        }
    } else {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower
            .split_whitespace()
            .filter(|w| w.len() > 1)
            .collect();
        for item in &recent_items {
            let mut kw_score = 0.0f32;
            let title_lower = item.title.to_lowercase();
            let summary_lower = item.summary.to_lowercase();
            let content_lower = item.content_markdown.to_lowercase();
            for word in &query_words {
                if title_lower.contains(word) {
                    kw_score += 5.0;
                }
                if summary_lower.contains(word) {
                    kw_score += 3.0;
                }
                if content_lower.contains(word) {
                    kw_score += 1.0;
                }
            }
            if kw_score > 0.0 {
                candidate_ids.push(item.id.clone());
                scores.insert(item.id.clone(), kw_score * 0.5);
            }
        }
    }

    // Always include recent item IDs as baseline candidates
    for item in &recent_items {
        if !candidate_ids.contains(&item.id) {
            candidate_ids.push(item.id.clone());
        }
    }

    // 2. Pre-Filtered Vector Search: Load embeddings ONLY for candidates
    if let Some(q_emb) = query_embedding {
        if let Ok(candidate_embeddings) = db::get_memory_embeddings_by_ids(conn, &candidate_ids) {
            for (id, item_embedding) in candidate_embeddings {
                let sim = cosine_similarity(q_emb, &item_embedding);
                let vec_score = sim * 10.0;
                *scores.entry(id).or_insert(0.0) += vec_score * 0.5;
            }
        }
    }

    // 3. Map candidates and rank top memory items
    let item_map: HashMap<String, MemoryItem> = recent_items
        .into_iter()
        .map(|item| (item.id.clone(), item))
        .collect();

    let mut scored_list: Vec<(f32, MemoryItem)> = candidate_ids
        .into_iter()
        .filter_map(|id| {
            item_map.get(&id).map(|item| {
                let base_score = scores.get(&id).copied().unwrap_or(0.0);
                let final_score = base_score + (item.importance as f32) * 0.5;
                (final_score, item.clone())
            })
        })
        .filter(|(s, _)| *s > 0.0)
        .collect();

    scored_list.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    if scored_list.is_empty() {
        let fallback_items = db::get_memory_items(conn, Some(limit))?;
        return Ok(fallback_items);
    }

    Ok(scored_list
        .into_iter()
        .take(limit)
        .map(|(_, item)| item)
        .collect())
}

/// Calculate cosine similarity between two vector slices
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|y| y * y).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

#[cfg(test)]
#[path = "tests/retrieval_tests.rs"]
mod tests;

