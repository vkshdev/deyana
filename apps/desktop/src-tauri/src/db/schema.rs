pub const INIT_SQL: &str = r#"
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS memory_items (
  id TEXT PRIMARY KEY,
  type TEXT NOT NULL,
  title TEXT NOT NULL,
  summary TEXT NOT NULL,
  content_markdown TEXT NOT NULL,
  markdown_path TEXT,
  source_type TEXT NOT NULL,
  source_id TEXT,
  source_uri TEXT,
  importance INTEGER NOT NULL DEFAULT 3,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);

CREATE TABLE IF NOT EXISTS memory_tags (
  memory_id TEXT NOT NULL,
  tag TEXT NOT NULL,
  PRIMARY KEY (memory_id, tag),
  FOREIGN KEY (memory_id) REFERENCES memory_items(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS memory_chunks (
  id TEXT PRIMARY KEY,
  memory_id TEXT NOT NULL,
  chunk_text TEXT NOT NULL,
  chunk_index INTEGER NOT NULL,
  token_estimate INTEGER NOT NULL,
  embedding_model TEXT,
  embedding_ref TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (memory_id) REFERENCES memory_items(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS memory_entities (
  id TEXT PRIMARY KEY,
  memory_id TEXT NOT NULL,
  name TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  source_text TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (memory_id) REFERENCES memory_items(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS memory_insights (
  id TEXT PRIMARY KEY,
  memory_id TEXT NOT NULL,
  type TEXT NOT NULL,
  title TEXT NOT NULL,
  detail TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'open',
  due_at TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (memory_id) REFERENCES memory_items(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS memory_links (
  id TEXT PRIMARY KEY,
  source_id TEXT NOT NULL,
  target_id TEXT NOT NULL,
  relation_type TEXT NOT NULL CHECK (relation_type IN ('related_to', 'depends_on', 'contradicts', 'same_entity')),
  weight REAL NOT NULL DEFAULT 1.0 CHECK (weight >= 0.0 AND weight <= 1.0),
  metadata_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  CHECK (source_id != target_id),
  UNIQUE (source_id, target_id, relation_type),
  FOREIGN KEY (source_id) REFERENCES memory_items(id) ON DELETE CASCADE,
  FOREIGN KEY (target_id) REFERENCES memory_items(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS triage_inbox (
  id TEXT PRIMARY KEY,
  platform TEXT NOT NULL,
  sender TEXT NOT NULL,
  content TEXT NOT NULL,
  urgency_score TEXT NOT NULL,
  auto_draft TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending',
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS chat_messages (
  id TEXT PRIMARY KEY,
  session_id TEXT,
  role TEXT NOT NULL,
  content TEXT NOT NULL,
  model TEXT,
  source_context_json TEXT NOT NULL DEFAULT '[]',
  web_source_context_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS privacy_audit_logs (
  id TEXT PRIMARY KEY,
  event_type TEXT NOT NULL,
  decision TEXT NOT NULL,
  reason TEXT NOT NULL,
  target_domain TEXT NOT NULL,
  destination_category TEXT NOT NULL,
  data_category TEXT NOT NULL,
  purpose TEXT NOT NULL,
  method TEXT NOT NULL,
  risk_level TEXT NOT NULL DEFAULT 'low',
  user_approved INTEGER NOT NULL DEFAULT 0,
  connector_id TEXT,
  safe_alternative TEXT NOT NULL,
  payload_sha256 TEXT,
  payload_character_count INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS connectors (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  status TEXT NOT NULL,
  enabled INTEGER NOT NULL,
  scopes_json TEXT NOT NULL,
  sync_interval_minutes INTEGER NOT NULL,
  last_sync_at TEXT,
  next_sync_at TEXT,
  token_stored INTEGER NOT NULL,
  token_updated_at TEXT,
  last_error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS connector_tokens (
  connector_id TEXT PRIMARY KEY,
  encrypted_token TEXT NOT NULL,
  nonce TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS connector_oauth_states (
  state TEXT PRIMARY KEY,
  connector_id TEXT NOT NULL,
  redirect_uri TEXT NOT NULL,
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  used_at TEXT
);

CREATE TABLE IF NOT EXISTS connector_sync_runs (
  id TEXT PRIMARY KEY,
  connector_id TEXT NOT NULL,
  status TEXT NOT NULL,
  reason TEXT NOT NULL,
  started_at TEXT NOT NULL,
  completed_at TEXT,
  items_seen INTEGER NOT NULL DEFAULT 0,
  items_written INTEGER NOT NULL DEFAULT 0,
  error_message TEXT
);

CREATE TABLE IF NOT EXISTS connector_items (
  id TEXT PRIMARY KEY,
  connector_id TEXT NOT NULL,
  external_id TEXT NOT NULL,
  title TEXT NOT NULL,
  summary TEXT NOT NULL,
  source_uri TEXT,
  item_timestamp TEXT,
  normalized_json TEXT NOT NULL,
  memory_id TEXT NOT NULL,
  fetched_at TEXT NOT NULL,
  UNIQUE(connector_id, external_id)
);

CREATE TABLE IF NOT EXISTS browser_sessions (
  id TEXT PRIMARY KEY,
  origin TEXT NOT NULL,
  url TEXT NOT NULL,
  title TEXT NOT NULL,
  adapter_id TEXT NOT NULL,
  mode TEXT NOT NULL,
  character_count INTEGER NOT NULL,
  truncated INTEGER NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  expires_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS browser_permissions (
  origin TEXT NOT NULL,
  kind TEXT NOT NULL,
  granted INTEGER NOT NULL,
  granted_at TEXT,
  detail TEXT NOT NULL,
  PRIMARY KEY (origin, kind)
);

CREATE TABLE IF NOT EXISTS browser_audit_events (
  id TEXT PRIMARY KEY,
  event_type TEXT NOT NULL,
  decision TEXT NOT NULL,
  operation TEXT NOT NULL,
  origin TEXT,
  page_session_id TEXT,
  detail TEXT NOT NULL,
  payload_sha256 TEXT,
  payload_character_count INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS browser_action_plans (
  id TEXT PRIMARY KEY,
  status TEXT NOT NULL,
  summary TEXT NOT NULL,
  preview_markdown TEXT NOT NULL,
  origin TEXT,
  page_session_id TEXT,
  steps_json TEXT NOT NULL,
  confirmation_token_sha256 TEXT,
  expires_at TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  result_detail TEXT
);

CREATE TABLE IF NOT EXISTS browser_whatsapp_busy_policy (
  id TEXT PRIMARY KEY,
  enabled INTEGER NOT NULL,
  allowlisted_contacts_json TEXT NOT NULL,
  allow_groups INTEGER NOT NULL,
  timezone TEXT NOT NULL,
  window_start TEXT NOT NULL,
  window_end TEXT NOT NULL,
  cooldown_minutes INTEGER NOT NULL,
  daily_limit INTEGER NOT NULL,
  template TEXT NOT NULL,
  emergency_stopped INTEGER NOT NULL,
  permission_origin TEXT NOT NULL,
  permission_granted INTEGER NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS browser_whatsapp_busy_events (
  id TEXT PRIMARY KEY,
  contact_label TEXT NOT NULL,
  decision TEXT NOT NULL,
  reason TEXT NOT NULL,
  category TEXT NOT NULL,
  urgent INTEGER NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS browser_personality_profile (
  id TEXT PRIMARY KEY,
  preset TEXT NOT NULL,
  display_name TEXT NOT NULL,
  custom_instruction TEXT NOT NULL,
  writer_temperature REAL NOT NULL,
  max_draft_characters INTEGER NOT NULL,
  automation_disclosure TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS browser_contact_tones (
  adapter_id TEXT NOT NULL,
  contact_label TEXT NOT NULL,
  tone_instruction TEXT NOT NULL,
  approved INTEGER NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (adapter_id, contact_label)
);

CREATE TABLE IF NOT EXISTS memory_save_suggestions (
  id TEXT PRIMARY KEY,
  session_id TEXT,
  fact_text TEXT NOT NULL,
  entities_json TEXT NOT NULL DEFAULT '[]',
  category TEXT NOT NULL,
  importance INTEGER NOT NULL DEFAULT 3,
  rationale TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'dismissed', 'auto_committed')),
  created_memory_id TEXT,
  created_at TEXT NOT NULL,
  resolved_at TEXT,
  FOREIGN KEY (created_memory_id) REFERENCES memory_items(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS cognition_self_model (
  id TEXT PRIMARY KEY,
  identity_statement TEXT NOT NULL,
  core_values_json TEXT NOT NULL DEFAULT '[]',
  intellectual_interests_json TEXT NOT NULL DEFAULT '[]',
  self_observations_json TEXT NOT NULL DEFAULT '[]',
  user_observations_json TEXT NOT NULL DEFAULT '[]',
  version INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS cognition_self_model_audit_log (
  id TEXT PRIMARY KEY,
  version INTEGER NOT NULL,
  field_changed TEXT NOT NULL,
  operation TEXT NOT NULL,
  old_value_json TEXT,
  new_value_json TEXT NOT NULL,
  rationale TEXT NOT NULL,
  triggered_by TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS cognition_reflections (
  id TEXT PRIMARY KEY,
  session_id TEXT,
  turn_prompt TEXT NOT NULL,
  turn_response TEXT NOT NULL,
  accuracy_score REAL NOT NULL,
  completeness_score REAL NOT NULL,
  confidence_score REAL NOT NULL,
  missed_opportunities TEXT NOT NULL DEFAULT '',
  correction_guidance TEXT NOT NULL DEFAULT '',
  flagged_for_correction INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS cognition_proactive_thoughts (
  id TEXT PRIMARY KEY,
  trigger_reason TEXT NOT NULL,
  thought_summary TEXT NOT NULL,
  hypotheses_json TEXT NOT NULL DEFAULT '[]',
  source_memory_ids_json TEXT NOT NULL DEFAULT '[]',
  insight_id TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (insight_id) REFERENCES memory_insights(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_memory_items_deleted_at ON memory_items(deleted_at);
CREATE INDEX IF NOT EXISTS idx_memory_items_updated_at ON memory_items(updated_at);
CREATE INDEX IF NOT EXISTS idx_memory_items_type ON memory_items(type);
CREATE INDEX IF NOT EXISTS idx_memory_entities_name ON memory_entities(name);
CREATE INDEX IF NOT EXISTS idx_memory_insights_type ON memory_insights(type);
CREATE INDEX IF NOT EXISTS idx_memory_links_source ON memory_links(source_id);
CREATE INDEX IF NOT EXISTS idx_memory_links_target ON memory_links(target_id);
CREATE INDEX IF NOT EXISTS idx_memory_links_relation ON memory_links(relation_type);
CREATE INDEX IF NOT EXISTS idx_chat_messages_created_at ON chat_messages(created_at);
CREATE INDEX IF NOT EXISTS idx_triage_inbox_status ON triage_inbox(status);
CREATE INDEX IF NOT EXISTS idx_privacy_audit_logs_created_at ON privacy_audit_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_privacy_audit_logs_decision ON privacy_audit_logs(decision);
CREATE INDEX IF NOT EXISTS idx_connector_sync_runs_started_at ON connector_sync_runs(started_at);
CREATE INDEX IF NOT EXISTS idx_connector_items_connector_id ON connector_items(connector_id);
CREATE INDEX IF NOT EXISTS idx_browser_audit_created_at ON browser_audit_events(created_at);
CREATE INDEX IF NOT EXISTS idx_browser_action_plans_updated_at ON browser_action_plans(updated_at);
CREATE INDEX IF NOT EXISTS idx_memory_suggestions_status ON memory_save_suggestions(status);
CREATE INDEX IF NOT EXISTS idx_memory_suggestions_created ON memory_save_suggestions(created_at);
CREATE INDEX IF NOT EXISTS idx_cognition_audit_created ON cognition_self_model_audit_log(created_at);
CREATE INDEX IF NOT EXISTS idx_cognition_audit_field ON cognition_self_model_audit_log(field_changed);
CREATE INDEX IF NOT EXISTS idx_cognition_reflections_created ON cognition_reflections(created_at);
CREATE INDEX IF NOT EXISTS idx_cognition_reflections_confidence ON cognition_reflections(confidence_score);
CREATE INDEX IF NOT EXISTS idx_cognition_thoughts_created ON cognition_proactive_thoughts(created_at);

CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
  memory_id UNINDEXED,
  title,
  summary,
  content_markdown,
  tags
);

CREATE TABLE IF NOT EXISTS memory_embeddings (
  memory_id TEXT PRIMARY KEY,
  embedding_json TEXT NOT NULL,
  model TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (memory_id) REFERENCES memory_items(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS semantic_cache (
  id TEXT PRIMARY KEY,
  query_embedding_json TEXT NOT NULL,
  memory_context_hash TEXT NOT NULL,
  response TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_semantic_cache_created_at ON semantic_cache(created_at);

CREATE TABLE IF NOT EXISTS training_dataset (
  id TEXT PRIMARY KEY,
  instruction TEXT NOT NULL,
  input_context TEXT NOT NULL,
  output TEXT NOT NULL,
  model_used TEXT NOT NULL,
  window_title TEXT,
  feedback INTEGER NOT NULL DEFAULT 0,
  user_edit TEXT,
  session_id TEXT,
  pii_masked INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_training_dataset_created_at ON training_dataset(created_at);
CREATE INDEX IF NOT EXISTS idx_training_dataset_feedback ON training_dataset(feedback);
"#;


