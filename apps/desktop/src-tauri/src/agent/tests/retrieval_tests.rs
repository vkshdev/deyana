use super::*;
use rusqlite::Connection;
use crate::db::schema::INIT_SQL;
use crate::db::repository::{save_memory_item, save_memory_embedding, search_memory_fts, get_all_memory_embeddings};

#[test]
fn test_cosine_similarity_edge_cases() {
    let v1 = vec![1.0, 0.0, 0.0];
    let v2 = vec![1.0, 0.0, 0.0];
    assert!((cosine_similarity(&v1, &v2) - 1.0).abs() < 1e-5);

    let v3 = vec![0.0, 1.0, 0.0];
    assert!((cosine_similarity(&v1, &v3) - 0.0).abs() < 1e-5);

    let empty: Vec<f32> = vec![];
    assert_eq!(cosine_similarity(&empty, &v1), 0.0);

    let mismatched = vec![1.0, 0.0];
    assert_eq!(cosine_similarity(&mismatched, &v1), 0.0);
}

#[test]
fn test_fts_and_embedding_retrieval_flow() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(INIT_SQL).unwrap();

    let item1 = save_memory_item(
        &conn,
        "Rust async programming with Tauri desktop apps",
        "notes",
        Some(vec!["rust".to_string(), "tauri".to_string()]),
    ).unwrap();

    let item2 = save_memory_item(
        &conn,
        "Python machine learning and PyTorch neural networks",
        "notes",
        Some(vec!["python".to_string(), "pytorch".to_string()]),
    ).unwrap();

    let emb1 = vec![0.8f32, 0.2f32, 0.1f32];
    let emb2 = vec![0.1f32, 0.9f32, 0.3f32];
    save_memory_embedding(&conn, &item1.id, &emb1, "all-minilm:latest").unwrap();
    save_memory_embedding(&conn, &item2.id, &emb2, "all-minilm:latest").unwrap();

    let fts_res = search_memory_fts(&conn, "Rust async", 5).unwrap();
    assert!(!fts_res.is_empty());
    assert_eq!(fts_res[0].0, item1.id);

    let all_embs = get_all_memory_embeddings(&conn).unwrap();
    assert_eq!(all_embs.len(), 2);

    let query_emb = vec![0.75f32, 0.25f32, 0.05f32];
    let retrieved = retrieve_memory_context(&conn, Some(&query_emb), "Rust async", 5).unwrap();
    assert!(!retrieved.is_empty());
    assert_eq!(retrieved[0].id, item1.id);
}
