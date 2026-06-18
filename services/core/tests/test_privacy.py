from __future__ import annotations

from fastapi.testclient import TestClient

from deyana_core.app import create_app
from deyana_core.runtime import RuntimeState
from deyana_core.settings import CoreSettings


def make_client(tmp_path) -> TestClient:
    settings = CoreSettings(data_dir=tmp_path / "data", log_dir=tmp_path / "logs")
    return TestClient(create_app(RuntimeState(settings)))


def test_external_ai_endpoint_is_blocked_and_logged(tmp_path) -> None:
    with make_client(tmp_path) as client:
        response = client.post(
            "/privacy/check",
            json={
                "url": "https://api.openai.com/v1/chat/completions",
                "method": "POST",
                "purpose": "cloud_ai",
                "dataCategory": "private_memory",
                "payloadPreview": "Private memory summary from my vault",
            },
        )
        audit = client.get("/privacy/audit")
        status = client.get("/privacy/status")

    assert response.status_code == 200
    body = response.json()
    assert body["allowed"] is False
    assert body["destinationCategory"] == "cloud_ai"
    assert body["dataCategory"] == "private_memory"
    assert body["safeAlternative"] == "Use local Ollama model"
    assert body["auditEvent"]["decision"] == "block"
    assert body["auditEvent"]["payloadSha256"]
    assert "Private memory" not in str(body["auditEvent"])

    assert audit.status_code == 200
    assert audit.json()["total"] == 1
    assert audit.json()["events"][0]["eventType"] == "privacy.request.blocked"
    assert audit.json()["events"][0]["reason"] == body["reason"]

    assert status.status_code == 200
    assert status.json()["blockedEvents"] == 1
    assert status.json()["lastBlocked"]["destinationCategory"] == "cloud_ai"


def test_hosted_embeddings_and_cloud_voice_are_blocked(tmp_path) -> None:
    with make_client(tmp_path) as client:
        embedding = client.post(
            "/privacy/check",
            json={
                "url": "https://api.openai.com/v1/embeddings",
                "method": "POST",
                "purpose": "embedding",
                "dataCategory": "embedding_text",
                "payloadPreview": "Memory chunk to embed",
            },
        )
        stt = client.post(
            "/privacy/check",
            json={
                "url": "https://api.deepgram.com/v1/listen",
                "method": "POST",
                "purpose": "speech_to_text",
                "dataCategory": "audio",
                "payloadPreview": "voice recording bytes",
            },
        )
        tts = client.post(
            "/privacy/check",
            json={
                "url": "https://api.elevenlabs.io/v1/text-to-speech",
                "method": "POST",
                "purpose": "text_to_speech",
                "dataCategory": "transcript",
                "payloadPreview": "private transcript",
            },
        )
        audit = client.get("/privacy/audit")

    assert embedding.json()["allowed"] is False
    assert embedding.json()["destinationCategory"] == "hosted_embedding"
    assert "local embedding" in embedding.json()["safeAlternative"].lower()

    assert stt.json()["allowed"] is False
    assert stt.json()["destinationCategory"] == "cloud_stt"
    assert "local stt" in stt.json()["safeAlternative"].lower()

    assert tts.json()["allowed"] is False
    assert tts.json()["destinationCategory"] == "cloud_tts"
    assert "local tts" in tts.json()["safeAlternative"].lower()

    assert audit.json()["total"] == 3
    assert {event["destinationCategory"] for event in audit.json()["events"]} == {
        "hosted_embedding",
        "cloud_stt",
        "cloud_tts",
    }


def test_public_web_fetch_and_approved_oauth_fetch_are_allowed(tmp_path) -> None:
    with make_client(tmp_path) as client:
        public_fetch = client.post(
            "/privacy/check",
            json={
                "url": "https://example.com/docs",
                "method": "GET",
                "purpose": "public_web_fetch",
                "dataCategory": "public_query",
            },
        )
        oauth_fetch = client.post(
            "/privacy/check",
            json={
                "url": "https://oauth2.googleapis.com/token",
                "method": "POST",
                "purpose": "oauth_api_fetch",
                "dataCategory": "oauth_token",
                "userApproved": True,
                "connectorId": "google",
            },
        )
        audit = client.get("/privacy/audit")

    assert public_fetch.json()["allowed"] is True
    assert public_fetch.json()["destinationCategory"] == "public_web"
    assert oauth_fetch.json()["allowed"] is True
    assert oauth_fetch.json()["destinationCategory"] == "oauth_connector"
    assert audit.json()["total"] == 2
    assert {event["decision"] for event in audit.json()["events"]} == {"allow"}


def test_sensitive_payload_to_public_web_and_unapproved_oauth_are_blocked(tmp_path) -> None:
    with make_client(tmp_path) as client:
        sensitive_public = client.post(
            "/privacy/check",
            json={
                "url": "https://example.com/search",
                "method": "GET",
                "purpose": "public_web_fetch",
                "dataCategory": "memory_summary",
                "payloadPreview": "private memory about launch pricing",
            },
        )
        unapproved_oauth = client.post(
            "/privacy/check",
            json={
                "url": "https://oauth2.googleapis.com/token",
                "method": "POST",
                "purpose": "oauth_api_fetch",
                "dataCategory": "oauth_token",
                "userApproved": False,
            },
        )

    assert sensitive_public.json()["allowed"] is False
    assert "Sensitive private payload" in sensitive_public.json()["reason"]
    assert sensitive_public.json()["safeAlternative"] == "Keep private data local"

    assert unapproved_oauth.json()["allowed"] is False
    assert "requires explicit user approval" in unapproved_oauth.json()["reason"]


def test_status_flags_and_audit_clear(tmp_path) -> None:
    with make_client(tmp_path) as client:
        client.post(
            "/privacy/check",
            json={
                "url": "https://api.anthropic.com/v1/messages",
                "method": "POST",
                "purpose": "cloud_ai",
                "dataCategory": "chat_history",
                "payloadPreview": "chat history",
            },
        )
        core_status = client.get("/status")
        delete_response = client.delete("/privacy/audit")
        audit = client.get("/privacy/audit")

    assert core_status.status_code == 200
    assert core_status.json()["featureFlags"]["privacyFirewall"] is True
    assert core_status.json()["featureFlags"]["privacyAudit"] is True
    assert any(
        dependency["name"] == "privacy_firewall"
        and dependency["status"] == "available"
        for dependency in core_status.json()["dependencies"]
    )
    assert delete_response.json()["deleted"] == 1
    assert audit.json()["total"] == 0
