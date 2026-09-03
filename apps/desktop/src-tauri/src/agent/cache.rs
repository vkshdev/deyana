use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use lru::LruCache;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use rusqlite::Connection;

use crate::db::{self, MemoryItem};

pub const DEFAULT_SIMILARITY_THRESHOLD: f32 = 0.92;
pub const DEFAULT_CACHE_CAPACITY: usize = 128;
pub const DEFAULT_TTL_MINUTES: u64 = 30;

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub id: String,
    pub query_embedding: Vec<f32>,
    pub memory_context_hash: String,
    pub response: String,
    pub created_at: Instant,
}

#[derive(Clone)]
pub struct SemanticCache {
    lru: Arc<Mutex<LruCache<String, CacheEntry>>>,
    ttl: Duration,
    similarity_threshold: f32,
}

impl Default for SemanticCache {
    fn default() -> Self {
        Self::new(DEFAULT_CACHE_CAPACITY, DEFAULT_TTL_MINUTES, DEFAULT_SIMILARITY_THRESHOLD)
    }
}

impl SemanticCache {
    pub fn new(capacity: usize, ttl_minutes: u64, similarity_threshold: f32) -> Self {
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(128).unwrap());
        Self {
            lru: Arc::new(Mutex::new(LruCache::new(cap))),
            ttl: Duration::from_secs(ttl_minutes * 60),
            similarity_threshold,
        }
    }

    /// Compute a deterministic SHA-256 hash for the current memory context items.
    pub fn compute_context_hash(items: &[MemoryItem]) -> String {
        if items.is_empty() {
            return "empty_context".to_string();
        }

        let mut hasher = Sha256::new();
        let mut sorted_keys: Vec<String> = items
            .iter()
            .map(|item| format!("{}:{}:{}", item.id, item.updated_at, item.title))
            .collect();
        sorted_keys.sort();

        for key in sorted_keys {
            hasher.update(key.as_bytes());
            hasher.update(b";");
        }

        format!("{:x}", hasher.finalize())
    }

    /// Check if an incoming query embedding matches a cached prompt with the same context hash.
    /// Returns the cached response if cosine similarity >= threshold and context hash matches.
    pub fn lookup(&self, query_embedding: &[f32], context_hash: &str) -> Option<String> {
        if query_embedding.is_empty() {
            return None;
        }

        let mut lock = self.lru.lock().ok()?;
        let now = Instant::now();

        // Scan entries for best match
        let mut best_id: Option<String> = None;
        let mut best_sim = self.similarity_threshold;

        for (id, entry) in lock.iter() {
            // Check TTL
            if now.duration_since(entry.created_at) > self.ttl {
                continue;
            }

            // Check Context Hash match
            if entry.memory_context_hash != context_hash {
                continue;
            }

            // Calculate cosine similarity
            let sim = cosine_similarity(query_embedding, &entry.query_embedding);
            if sim >= best_sim {
                best_sim = sim;
                best_id = Some(id.clone());
            }
        }

        if let Some(id) = best_id {
            // Promote in LRU
            if let Some(entry) = lock.get(&id) {
                return Some(entry.response.clone());
            }
        }

        None
    }

    /// Insert a response into the in-memory cache and persist to SQLite.
    pub fn insert(
        &self,
        query_embedding: Vec<f32>,
        context_hash: String,
        response: String,
        conn: Option<&Connection>,
    ) -> String {
        let id = format!("cache_{}", Uuid::new_v4().simple());
        let entry = CacheEntry {
            id: id.clone(),
            query_embedding: query_embedding.clone(),
            memory_context_hash: context_hash.clone(),
            response: response.clone(),
            created_at: Instant::now(),
        };

        if let Ok(mut lock) = self.lru.lock() {
            lock.put(id.clone(), entry);
        }

        if let Some(c) = conn {
            let _ = db::save_cache_entry(c, &id, &query_embedding, &context_hash, &response);
        }

        id
    }

    /// Clear all entries from the cache (both memory and SQLite if connection provided).
    pub fn clear(&self, conn: Option<&Connection>) -> usize {
        let mut cleared = 0;
        if let Ok(mut lock) = self.lru.lock() {
            cleared = lock.len();
            lock.clear();
        }

        if let Some(c) = conn {
            let _ = db::clear_all_cache_entries(c);
        }

        cleared
    }

    /// Hydrate the in-memory LRU cache from SQLite persisted records.
    pub fn load_from_db(conn: &Connection) -> Self {
        let cache = Self::default();
        if let Ok(rows) = db::load_cache_entries(conn, DEFAULT_CACHE_CAPACITY) {
            if let Ok(mut lock) = cache.lru.lock() {
                for row in rows {
                    let entry = CacheEntry {
                        id: row.id.clone(),
                        query_embedding: row.query_embedding,
                        memory_context_hash: row.memory_context_hash,
                        response: row.response,
                        created_at: Instant::now(),
                    };
                    lock.put(row.id, entry);
                }
            }
        }
        cache
    }

    /// Get current cache stats (item count, capacity)
    pub fn stats(&self) -> (usize, usize) {
        if let Ok(lock) = self.lru.lock() {
            (lock.len(), lock.cap().get())
        } else {
            (0, DEFAULT_CACHE_CAPACITY)
        }
    }
}

/// Calculate cosine similarity between two vector slices.
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
#[path = "tests/cache_tests.rs"]
mod tests;

