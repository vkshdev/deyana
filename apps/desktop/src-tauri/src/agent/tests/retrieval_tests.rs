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

#[test]
fn test_relational_graph_walk_retrieval_no_keyword_overlap() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(INIT_SQL).unwrap();

    let item_a = save_memory_item(
        &conn,
        "Kubernetes cluster architecture on AWS cloud infrastructure",
        "notes",
        Some(vec!["k8s".to_string(), "cloud".to_string()]),
    )
    .unwrap();

    // Item B has zero word overlap with "Kubernetes" or "AWS"
    let item_b = save_memory_item(
        &conn,
        "VPC subnet security group firewall rules configuration",
        "notes",
        Some(vec!["networking".to_string()]),
    )
    .unwrap();

    // Distractor with zero relation and zero query overlap
    let item_c = save_memory_item(
        &conn,
        "Cooking authentic Italian marinara pasta with fresh basil",
        "notes",
        Some(vec!["recipes".to_string()]),
    )
    .unwrap();

    // Connect Seed A -> Target B via depends_on link
    crate::db::repository::create_memory_link(
        &conn,
        &item_a.id,
        &item_b.id,
        crate::db::models::MemoryRelationType::DependsOn,
        0.95,
        None,
    )
    .unwrap();

    // Query matches ONLY Seed A lexically
    let retrieved = retrieve_memory_context(&conn, None, "Kubernetes AWS", 5).unwrap();

    assert!(retrieved.len() >= 2, "Should retrieve at least Seed A and Neighbor B");
    assert_eq!(retrieved[0].id, item_a.id, "Seed item A must be primary result");
    assert_eq!(
        retrieved[1].id, item_b.id,
        "Neighbor item B must be retrieved via graph walk ahead of unrelated distractor"
    );

    let b_idx = retrieved.iter().position(|it| it.id == item_b.id);
    let c_idx = retrieved.iter().position(|it| it.id == item_c.id);

    assert!(b_idx.is_some(), "Neighbor B must be in retrieved results");
    if let (Some(b_pos), Some(c_pos)) = (b_idx, c_idx) {
        assert!(b_pos < c_pos, "Neighbor B must rank ahead of distractor C");
    }
}

#[test]
fn test_relation_weight_and_type_discounting() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(INIT_SQL).unwrap();

    let seed = save_memory_item(
        &conn,
        "Deep learning neural network backpropagation gradient descent",
        "notes",
        Some(vec!["ai".to_string()]),
    )
    .unwrap();

    // Target 1: Linked via depends_on (factor 1.0), weight 1.0 -> effective multiplier 1.0
    let dep_high = save_memory_item(
        &conn,
        "Matrix multiplication tensors GPU compute acceleration",
        "notes",
        None,
    )
    .unwrap();

    // Target 2: Linked via contradicts (factor 0.7), weight 1.0 -> effective multiplier 0.7
    let contra_high = save_memory_item(
        &conn,
        "Biological brain synaptic plasticity non-gradient adaptation",
        "notes",
        None,
    )
    .unwrap();

    // Target 3: Linked via depends_on (factor 1.0), weight 0.5 -> effective multiplier 0.5
    let dep_low = save_memory_item(
        &conn,
        "Linear regression least squares algebraic closed form",
        "notes",
        None,
    )
    .unwrap();

    crate::db::repository::create_memory_link(
        &conn,
        &seed.id,
        &dep_high.id,
        crate::db::models::MemoryRelationType::DependsOn,
        1.0,
        None,
    )
    .unwrap();

    crate::db::repository::create_memory_link(
        &conn,
        &seed.id,
        &contra_high.id,
        crate::db::models::MemoryRelationType::Contradicts,
        1.0,
        None,
    )
    .unwrap();

    crate::db::repository::create_memory_link(
        &conn,
        &seed.id,
        &dep_low.id,
        crate::db::models::MemoryRelationType::DependsOn,
        0.5,
        None,
    )
    .unwrap();

    let retrieved = retrieve_memory_context(&conn, None, "Deep learning backpropagation", 10).unwrap();

    let pos_dep_high = retrieved.iter().position(|it| it.id == dep_high.id).expect("dep_high should be retrieved");
    let pos_contra_high = retrieved.iter().position(|it| it.id == contra_high.id).expect("contra_high should be retrieved");
    let pos_dep_low = retrieved.iter().position(|it| it.id == dep_low.id).expect("dep_low should be retrieved");

    // dep_high (rel 1.0, wt 1.0) must rank higher than contra_high (rel 0.7, wt 1.0)
    assert!(
        pos_dep_high < pos_contra_high,
        "depends_on (1.0) should rank ahead of contradicts (0.7)"
    );

    // dep_high (rel 1.0, wt 1.0) must rank higher than dep_low (rel 1.0, wt 0.5)
    assert!(
        pos_dep_high < pos_dep_low,
        "weight 1.0 should rank ahead of weight 0.5 under identical relation"
    );

    // contra_high (0.7 * 1.0 = 0.7) must rank higher than dep_low (1.0 * 0.5 = 0.5)
    assert!(
        pos_contra_high < pos_dep_low,
        "contra_high (eff 0.70) should rank ahead of dep_low (eff 0.50)"
    );
}
