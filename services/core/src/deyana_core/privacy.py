from __future__ import annotations

import hashlib
import sqlite3
import uuid
from pathlib import Path
from urllib.parse import urlparse

from .models import (
    PrivacyAuditDeleteResponse,
    PrivacyAuditEvent,
    PrivacyAuditListResponse,
    PrivacyCheckRequest,
    PrivacyCheckResponse,
    PrivacyDataCategory,
    PrivacyDecision,
    PrivacyDestinationCategory,
    PrivacyRequestPurpose,
    PrivacyStatusResponse,
)
from .runtime_time import utc_timestamp
from .storage import CoreStore

BLOCKED_DESTINATION_CATEGORIES: list[PrivacyDestinationCategory] = [
    "cloud_ai",
    "hosted_embedding",
    "hosted_reranker",
    "cloud_stt",
    "cloud_tts",
]

SENSITIVE_DATA_CATEGORIES: set[PrivacyDataCategory] = {
    "private_memory",
    "memory_summary",
    "embedding_text",
    "audio",
    "transcript",
    "source_code",
    "local_file",
    "chat_history",
}

PUBLIC_WEB_DATA_CATEGORIES: set[PrivacyDataCategory] = {
    "public_query",
    "public_content",
    "unknown",
}

OAUTH_DATA_CATEGORIES: set[PrivacyDataCategory] = {
    "oauth_token",
    "connector_metadata",
    "public_query",
    "public_content",
    "unknown",
}

CLOUD_AI_HOSTS = {
    "api.openai.com",
    "chatgpt.com",
    "api.anthropic.com",
    "api.groq.com",
    "api.mistral.ai",
    "api.cohere.ai",
    "api.together.xyz",
    "api.perplexity.ai",
    "api.deepseek.com",
    "openrouter.ai",
    "api.openrouter.ai",
    "generativelanguage.googleapis.com",
    "aiplatform.googleapis.com",
}

EMBEDDING_HOSTS = {
    "api.jina.ai",
    "api.voyageai.com",
}

STT_HOSTS = {
    "api.deepgram.com",
    "api.assemblyai.com",
    "api.rev.ai",
    "speech.googleapis.com",
}

TTS_HOSTS = {
    "api.elevenlabs.io",
    "api.play.ht",
    "texttospeech.googleapis.com",
}

OAUTH_CONNECTOR_HOSTS = {
    "accounts.google.com",
    "oauth2.googleapis.com",
    "www.googleapis.com",
    "gmail.googleapis.com",
    "calendar-json.googleapis.com",
    "api.github.com",
    "github.com",
    "slack.com",
    "api.slack.com",
    "api.notion.com",
    "auth.atlassian.com",
    "api.atlassian.com",
    "linear.app",
    "api.linear.app",
}

LOCAL_HOSTS = {
    "localhost",
    "127.0.0.1",
    "::1",
    "0.0.0.0",
}


class PrivacyPolicyError(RuntimeError):
    def __init__(self, response: PrivacyCheckResponse) -> None:
        super().__init__(response.reason)
        self.response = response


class PrivacyFirewall:
    def __init__(self, data_dir: Path, store: CoreStore) -> None:
        self.data_dir = data_dir
        self.store = store
        self.database_path = data_dir / "privacy.sqlite3"

    def initialize(self) -> None:
        self.data_dir.mkdir(parents=True, exist_ok=True)
        with self.connect() as connection:
            connection.executescript(
                """
                CREATE TABLE IF NOT EXISTS privacy_audit_events (
                  id TEXT PRIMARY KEY,
                  event_type TEXT NOT NULL,
                  decision TEXT NOT NULL,
                  reason TEXT NOT NULL,
                  destination TEXT NOT NULL,
                  destination_category TEXT NOT NULL,
                  data_category TEXT NOT NULL,
                  purpose TEXT NOT NULL,
                  method TEXT NOT NULL,
                  user_approved INTEGER NOT NULL,
                  connector_id TEXT,
                  safe_alternative TEXT NOT NULL,
                  payload_sha256 TEXT,
                  payload_character_count INTEGER NOT NULL DEFAULT 0,
                  created_at TEXT NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_privacy_audit_created_at
                ON privacy_audit_events(created_at);

                CREATE INDEX IF NOT EXISTS idx_privacy_audit_decision
                ON privacy_audit_events(decision);
                """
            )

    def connect(self) -> sqlite3.Connection:
        connection = sqlite3.connect(self.database_path)
        connection.row_factory = sqlite3.Row
        return connection

    def check(self, request: PrivacyCheckRequest) -> PrivacyCheckResponse:
        self.initialize()
        normalized_method = request.method.strip().upper() or "GET"
        destination = normalize_destination(request.url)
        destination_category = classify_destination(destination, request.purpose)
        data_category = request.data_category or classify_payload(request)
        decision, reason = decide(
            destination_category=destination_category,
            data_category=data_category,
            purpose=request.purpose,
            method=normalized_method,
            user_approved=request.user_approved,
            external_write=request.external_write,
        )
        safe_alternative = safe_alternative_for(destination_category, data_category)
        event = self.record(
            request=request,
            method=normalized_method,
            destination=destination,
            destination_category=destination_category,
            data_category=data_category,
            decision=decision,
            reason=reason,
            safe_alternative=safe_alternative,
        )
        return PrivacyCheckResponse(
            allowed=decision == "allow",
            decision=decision,
            reason=reason,
            destination=destination,
            destination_category=destination_category,
            data_category=data_category,
            purpose=request.purpose,
            safe_alternative=safe_alternative,
            audit_event=event,
        )

    def guard(self, request: PrivacyCheckRequest) -> PrivacyCheckResponse:
        response = self.check(request)
        if not response.allowed:
            raise PrivacyPolicyError(response)
        return response

    def list_events(self, limit: int = 50) -> PrivacyAuditListResponse:
        self.initialize()
        limit = max(1, min(limit, 200))
        with self.connect() as connection:
            rows = connection.execute(
                """
                SELECT * FROM privacy_audit_events
                ORDER BY created_at DESC
                LIMIT ?
                """,
                (limit,),
            ).fetchall()
            total = connection.execute("SELECT COUNT(*) AS count FROM privacy_audit_events").fetchone()[
                "count"
            ]
        return PrivacyAuditListResponse(events=[row_to_event(row) for row in rows], total=total)

    def status(self) -> PrivacyStatusResponse:
        self.initialize()
        settings = self.store.read_settings()
        with self.connect() as connection:
            counts = {
                row["decision"]: row["count"]
                for row in connection.execute(
                    "SELECT decision, COUNT(*) AS count FROM privacy_audit_events GROUP BY decision"
                ).fetchall()
            }
            total = connection.execute("SELECT COUNT(*) AS count FROM privacy_audit_events").fetchone()[
                "count"
            ]
            last_blocked_row = connection.execute(
                """
                SELECT * FROM privacy_audit_events
                WHERE decision = 'block'
                ORDER BY created_at DESC
                LIMIT 1
                """
            ).fetchone()

        return PrivacyStatusResponse(
            mode=settings.privacy_mode,
            enforced=True,
            audit_events=total,
            blocked_events=counts.get("block", 0),
            allowed_events=counts.get("allow", 0),
            last_blocked=row_to_event(last_blocked_row) if last_blocked_row else None,
            blocked_categories=BLOCKED_DESTINATION_CATEGORIES,
        )

    def clear(self) -> PrivacyAuditDeleteResponse:
        self.initialize()
        with self.connect() as connection:
            with connection:
                cursor = connection.execute("DELETE FROM privacy_audit_events")
        return PrivacyAuditDeleteResponse(deleted=cursor.rowcount)

    def record(
        self,
        *,
        request: PrivacyCheckRequest,
        method: str,
        destination: str,
        destination_category: PrivacyDestinationCategory,
        data_category: PrivacyDataCategory,
        decision: PrivacyDecision,
        reason: str,
        safe_alternative: str,
    ) -> PrivacyAuditEvent:
        event = PrivacyAuditEvent(
            id=f"privacy_{uuid.uuid4().hex}",
            event_type="privacy.request.blocked" if decision == "block" else "privacy.request.allowed",
            decision=decision,
            reason=reason,
            destination=destination,
            destination_category=destination_category,
            data_category=data_category,
            purpose=request.purpose,
            method=method,
            user_approved=request.user_approved,
            connector_id=request.connector_id,
            safe_alternative=safe_alternative,
            payload_sha256=hash_payload(request.payload_preview),
            payload_character_count=len(request.payload_preview or ""),
            created_at=utc_timestamp(),
        )
        with self.connect() as connection:
            with connection:
                connection.execute(
                    """
                    INSERT INTO privacy_audit_events (
                      id, event_type, decision, reason, destination,
                      destination_category, data_category, purpose, method,
                      user_approved, connector_id, safe_alternative,
                      payload_sha256, payload_character_count, created_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        event.id,
                        event.event_type,
                        event.decision,
                        event.reason,
                        event.destination,
                        event.destination_category,
                        event.data_category,
                        event.purpose,
                        event.method,
                        1 if event.user_approved else 0,
                        event.connector_id,
                        event.safe_alternative,
                        event.payload_sha256,
                        event.payload_character_count,
                        event.created_at,
                    ),
                )
        return event


def normalize_destination(url: str) -> str:
    value = url.strip()
    if not value:
        return "unknown"
    return value


def classify_destination(
    destination: str,
    purpose: PrivacyRequestPurpose,
) -> PrivacyDestinationCategory:
    parsed = urlparse(destination)
    host = (parsed.hostname or "").lower()
    path = parsed.path.lower()

    if parsed.scheme == "file" or host in LOCAL_HOSTS:
        return "local"
    if host.endswith(".localhost"):
        return "local"

    if purpose == "embedding":
        return "hosted_embedding"
    if purpose == "reranking":
        return "hosted_reranker"
    if purpose == "speech_to_text":
        return "cloud_stt"
    if purpose == "text_to_speech":
        return "cloud_tts"
    if purpose == "cloud_ai":
        return "cloud_ai"

    if is_host_match(host, CLOUD_AI_HOSTS):
        if "embedding" in path or "embeddings" in path:
            return "hosted_embedding"
        if "rerank" in path:
            return "hosted_reranker"
        if "audio/transcription" in path or "audio/translations" in path:
            return "cloud_stt"
        if "audio/speech" in path:
            return "cloud_tts"
        return "cloud_ai"

    if is_host_match(host, EMBEDDING_HOSTS) or "embedding" in path or "embeddings" in path:
        return "hosted_embedding"
    if "rerank" in path:
        return "hosted_reranker"
    if is_host_match(host, STT_HOSTS):
        return "cloud_stt"
    if is_host_match(host, TTS_HOSTS):
        return "cloud_tts"
    if "openai.azure.com" in host:
        return "cloud_ai"
    if is_host_match(host, OAUTH_CONNECTOR_HOSTS):
        return "oauth_connector"
    if parsed.scheme in {"http", "https"}:
        return "public_web"
    return "unknown_external"


def classify_payload(request: PrivacyCheckRequest) -> PrivacyDataCategory:
    purpose_to_data: dict[PrivacyRequestPurpose, PrivacyDataCategory] = {
        "embedding": "embedding_text",
        "speech_to_text": "audio",
        "text_to_speech": "transcript",
        "cloud_ai": "private_memory",
        "oauth_api_fetch": "oauth_token",
        "connector_api_fetch": "connector_metadata",
        "public_web_fetch": "public_query",
    }
    if request.purpose in purpose_to_data:
        return purpose_to_data[request.purpose]

    preview = (request.payload_preview or "").lower()
    if not preview:
        return "unknown"
    if any(token in preview for token in ["source code", "private repo", "stack trace", "api_key"]):
        return "source_code"
    if any(token in preview for token in ["gmail", "calendar", "slack", "notion", "connector"]):
        return "connector_metadata"
    if any(token in preview for token in ["voice recording", "audio", "microphone"]):
        return "audio"
    if any(token in preview for token in ["transcript", "dictation"]):
        return "transcript"
    if any(token in preview for token in ["vault", "memory", "chat history", "summary", "private note"]):
        return "memory_summary"
    return "public_query"


def decide(
    *,
    destination_category: PrivacyDestinationCategory,
    data_category: PrivacyDataCategory,
    purpose: PrivacyRequestPurpose,
    method: str,
    user_approved: bool,
    external_write: bool,
) -> tuple[PrivacyDecision, str]:
    if destination_category == "local":
        return "allow", "Local destination is allowed."

    if destination_category in BLOCKED_DESTINATION_CATEGORIES:
        return "block", blocked_destination_reason(destination_category)

    if destination_category == "oauth_connector":
        if purpose not in {"oauth_api_fetch", "connector_api_fetch"}:
            return "block", "Connector endpoints are allowed only for approved OAuth or connector fetches."
        if not user_approved:
            return "block", "Connector/OAuth request requires explicit user approval."
        if external_write and method not in {"GET", "HEAD"}:
            return "block", "External writes require a later confirmation flow before they can run."
        if data_category not in OAUTH_DATA_CATEGORIES:
            return "block", "Sensitive private payload cannot be sent to connector/OAuth endpoints."
        return "allow", "Approved connector/OAuth request is allowed."

    if destination_category == "public_web":
        if purpose != "public_web_fetch":
            return "block", "Public web access is limited to explicit public web fetch requests."
        if method not in {"GET", "HEAD"}:
            return "block", "Public web fetch is read-only in this phase."
        if data_category not in PUBLIC_WEB_DATA_CATEGORIES:
            return "block", "Sensitive private payload cannot be sent to public web endpoints."
        return "allow", "Public web fetch is allowed."

    if data_category in SENSITIVE_DATA_CATEGORIES:
        return "block", "Sensitive local data cannot leave the device."

    return "block", "Unknown external destination is blocked until a policy explicitly allows it."


def blocked_destination_reason(category: PrivacyDestinationCategory) -> str:
    reasons = {
        "cloud_ai": "External AI model endpoints are blocked in local-only mode.",
        "hosted_embedding": "Hosted embedding endpoints are blocked; embeddings must run locally.",
        "hosted_reranker": "Hosted reranker endpoints are blocked; reranking must run locally.",
        "cloud_stt": "Cloud speech-to-text endpoints are blocked; voice processing must run locally.",
        "cloud_tts": "Cloud text-to-speech endpoints are blocked; speech output must run locally.",
    }
    return reasons.get(category, "External destination is blocked.")


def safe_alternative_for(
    destination_category: PrivacyDestinationCategory,
    data_category: PrivacyDataCategory,
) -> str:
    if destination_category == "cloud_ai":
        return "Use local Ollama model"
    if destination_category == "hosted_embedding":
        return "Use local embedding model"
    if destination_category == "hosted_reranker":
        return "Use local retrieval and scoring"
    if destination_category == "cloud_stt":
        return "Use local STT"
    if destination_category == "cloud_tts":
        return "Use local TTS"
    if data_category in SENSITIVE_DATA_CATEGORIES:
        return "Keep private data local"
    return "Request explicit approval or add a specific policy"


def hash_payload(payload: str | None) -> str | None:
    if not payload:
        return None
    return hashlib.sha256(payload.encode("utf-8")).hexdigest()


def is_host_match(host: str, known_hosts: set[str]) -> bool:
    return any(host == known or host.endswith(f".{known}") for known in known_hosts)


def row_to_event(row: sqlite3.Row) -> PrivacyAuditEvent:
    return PrivacyAuditEvent(
        id=row["id"],
        event_type=row["event_type"],
        decision=row["decision"],
        reason=row["reason"],
        destination=row["destination"],
        destination_category=row["destination_category"],
        data_category=row["data_category"],
        purpose=row["purpose"],
        method=row["method"],
        user_approved=bool(row["user_approved"]),
        connector_id=row["connector_id"],
        safe_alternative=row["safe_alternative"],
        payload_sha256=row["payload_sha256"],
        payload_character_count=row["payload_character_count"],
        created_at=row["created_at"],
    )
