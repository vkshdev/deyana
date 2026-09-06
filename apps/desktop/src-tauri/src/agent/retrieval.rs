use std::collections::HashMap;
use rusqlite::Connection;
use crate::db::{self, MemoryItem};

pub const ALPHA_HOP: f32 = 0.65;
pub const MAX_SEEDS: usize = 5;
pub const MAX_NEIGHBORS_PER_SEED: usize = 3;

/// Retrieve relevant memory items using relational graph-walk retrieval:
/// 1. Seed candidate selection via dual-mode lexical (SQLite FTS5) and vector similarity.
/// 2. Bounded 1-hop graph walk across incident memory links (`ALPHA_HOP = 0.65`).
/// 3. Propagate scores scaled by link weight and relation type relevance multiplier:
///    - `depends_on`: 1.0
///    - `same_entity`: 0.9
///    - `related_to`: 0.8
///    - `contradicts`: 0.7
/// 4. Hydrate missing neighbor items and rerank candidate pool.
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

    let mut direct_scores: HashMap<String, f32> = HashMap::new();
    let mut candidate_ids: Vec<String> = Vec::new();

    // 1. Lexical Search (SQLite FTS5 matching, fallback to substring word matching)
    let fts_results = db::search_memory_fts(conn, query, 30).unwrap_or_default();
    if !fts_results.is_empty() {
        for (id, fts_score) in fts_results {
            candidate_ids.push(id.clone());
            direct_scores.insert(id, fts_score * 0.5);
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
                direct_scores.insert(item.id.clone(), kw_score * 0.5);
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
                *direct_scores.entry(id).or_insert(0.0) += vec_score * 0.5;
            }
        }
    }

    // Map existing candidates into memory item lookup table
    let mut item_map: HashMap<String, MemoryItem> = recent_items
        .into_iter()
        .map(|item| (item.id.clone(), item))
        .collect();

    // Rank candidates to discover top seeds for graph traversal
    let mut candidate_rankings: Vec<(String, f32)> = candidate_ids
        .iter()
        .filter_map(|id| {
            item_map.get(id).map(|item| {
                let base = direct_scores.get(id).copied().unwrap_or(0.0);
                let total = base + (item.importance as f32) * 0.5;
                (id.clone(), total)
            })
        })
        .filter(|(id, _)| direct_scores.get(id).copied().unwrap_or(0.0) > 0.0)
        .collect();

    candidate_rankings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // 3. Bounded 1-Hop Graph Traversal
    let seed_ids: Vec<String> = candidate_rankings
        .iter()
        .take(MAX_SEEDS)
        .map(|(id, _)| id.clone())
        .collect();

    let mut neighbor_scores: HashMap<String, f32> = HashMap::new();

    if !seed_ids.is_empty() {
        if let Ok(incident_links) = db::get_links_for_memory_ids(conn, &seed_ids) {
            let mut seed_neighbor_counts: HashMap<String, usize> = HashMap::new();

            for link in incident_links {
                let (seed_id, neighbor_id) = if seed_ids.contains(&link.source_id) {
                    (&link.source_id, &link.target_id)
                } else if seed_ids.contains(&link.target_id) {
                    (&link.target_id, &link.source_id)
                } else {
                    continue;
                };

                // Prevent self-cycles or looping back to an already selected seed
                if seed_ids.contains(neighbor_id) {
                    continue;
                }

                let count = seed_neighbor_counts.entry(seed_id.clone()).or_insert(0);
                if *count >= MAX_NEIGHBORS_PER_SEED {
                    continue;
                }
                *count += 1;

                let seed_relevance = direct_scores.get(seed_id).copied().unwrap_or(0.0);
                let rel_mult = link.relation_type.relevance_multiplier();
                let propagated_score = seed_relevance * ALPHA_HOP * link.weight * rel_mult;

                let curr = neighbor_scores.entry(neighbor_id.clone()).or_insert(0.0);
                if propagated_score > *curr {
                    *curr = propagated_score;
                }
            }
        }
    }

    // 4. Hydrate missing neighbor items not present in recent_items
    let missing_ids: Vec<String> = neighbor_scores
        .keys()
        .filter(|id| !item_map.contains_key(*id))
        .cloned()
        .collect();

    if !missing_ids.is_empty() {
        if let Ok(hydrated_items) = db::get_memory_items_by_ids(conn, &missing_ids) {
            for item in hydrated_items {
                item_map.insert(item.id.clone(), item);
            }
        }
    }

    // 5. Synthesize candidate pool and rerank
    let mut final_pool: Vec<(f32, MemoryItem)> = Vec::new();

    for (id, item) in &item_map {
        let direct_base = direct_scores.get(id).copied().unwrap_or(0.0);
        let propagated_base = neighbor_scores.get(id).copied().unwrap_or(0.0);
        let importance_bonus = (item.importance as f32) * 0.5;

        if direct_base > 0.0 || propagated_base > 0.0 {
            let relevance = direct_base.max(propagated_base);
            let final_score = relevance + importance_bonus;
            final_pool.push((final_score, item.clone()));
        }
    }

    final_pool.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.id.cmp(&b.1.id))
    });

    if final_pool.is_empty() {
        let fallback_items = db::get_memory_items(conn, Some(limit))?;
        return Ok(fallback_items);
    }

    Ok(final_pool
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

