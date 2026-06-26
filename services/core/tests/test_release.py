from __future__ import annotations

from pathlib import Path

from fastapi.testclient import TestClient

from deyana_core.app import create_app
from deyana_core.runtime import RuntimeState
from deyana_core.settings import CoreSettings


def make_client(tmp_path) -> TestClient:
    settings = CoreSettings(data_dir=tmp_path / "data", log_dir=tmp_path / "logs")
    return TestClient(create_app(RuntimeState(settings)))


def complete_onboarding(client: TestClient, tmp_path) -> Path:
    vault_path = tmp_path / "vault"
    assert client.post("/vault/select", json={"path": str(vault_path)}).status_code == 200
    assert (
        client.post(
            "/onboarding/complete",
            json={
                "privacyMode": "local_only",
                "modelProfile": "low_spec",
                "vaultPath": str(vault_path),
            },
        ).status_code
        == 200
    )
    return vault_path


def test_release_status_readiness_update_plan_performance_and_crash_recovery(tmp_path) -> None:
    with make_client(tmp_path) as client:
        status = client.get("/status")
        readiness = client.get("/release/readiness")
        update_plan = client.get("/release/update-plan")
        performance = client.get("/release/performance")
        crash = client.get("/release/crash-recovery")

    assert status.status_code == 200
    assert status.json()["featureFlags"]["releaseQuality"] is True
    assert status.json()["featureFlags"]["logsViewer"] is True
    assert status.json()["featureFlags"]["privacyExport"] is True
    assert status.json()["featureFlags"]["deleteLocalData"] is True
    assert status.json()["featureFlags"]["connectorHealth"] is True
    assert status.json()["featureFlags"]["performanceProfiling"] is True
    assert status.json()["featureFlags"]["crashRecovery"] is True
    assert any(dependency["name"] == "release_readiness" for dependency in status.json()["dependencies"])
    assert readiness.status_code == 200
    assert {item["id"] for item in readiness.json()["items"]} >= {"installer_bundle", "update_plan", "core_service"}
    assert update_plan.status_code == 200
    assert update_plan.json()["automaticUpdatesEnabled"] is False
    assert performance.status_code == 200
    assert any(metric["name"] == "profileLatencyMs" for metric in performance.json()["metrics"])
    assert crash.status_code == 200
    assert crash.json()["currentSessionId"]


def test_release_logs_are_listed_and_read_from_local_files(tmp_path) -> None:
    log_dir = tmp_path / "logs"
    log_dir.mkdir(parents=True)
    log_file = log_dir / "core.log"
    log_file.write_text("2026-06-24 INFO [deyana] release log line\n", encoding="utf-8")

    with make_client(tmp_path) as client:
        listed = client.get("/release/logs")
        read = client.get("/release/logs/read", params={"path": "core.log"})
        traversal = client.get("/release/logs/read", params={"path": "../secret.log"})

    assert listed.status_code == 200
    assert listed.json()["total"] >= 1
    assert any(file["path"] == "core.log" for file in listed.json()["files"])
    assert read.status_code == 200
    assert "release log line" in read.json()["content"]
    assert read.json()["truncated"] is False
    assert traversal.status_code == 400


def test_release_privacy_export_and_connector_health_use_local_state(tmp_path) -> None:
    with make_client(tmp_path) as client:
        complete_onboarding(client, tmp_path)
        client.post(
            "/memory",
            json={
                "title": "Release note",
                "contentMarkdown": "Decision: keep release export local.",
            },
        )
        client.app.state.runtime.chat_store.append("user", "Summarize local release readiness.")
        client.app.state.runtime.chat_store.append("assistant", "Release readiness is local.")
        client.post(
            "/privacy/test-request",
            json={
                "url": "https://api.openai.com/v1/chat/completions",
                "method": "POST",
                "purpose": "cloud_ai",
                "dataCategory": "private_memory",
                "payloadPreview": "private release data",
            },
        )
        export = client.get("/release/privacy-export")
        health = client.get("/release/connector-health")

    assert export.status_code == 200
    body = export.json()
    assert body["counts"]["memoryItems"] >= 1
    assert body["counts"]["chatMessages"] >= 2
    assert body["counts"]["privacyAuditEvents"] >= 1
    assert "connectors" in body["sections"]
    assert "connectorTokensStored" not in str(body)
    assert "Raw voice audio is not stored" in " ".join(body["notes"])
    assert health.status_code == 200
    assert health.json()["items"]
    assert all("connectorId" in item for item in health.json()["items"])


def test_delete_local_data_requires_confirmation_and_can_delete_vault(tmp_path) -> None:
    with make_client(tmp_path) as client:
        vault_path = complete_onboarding(client, tmp_path)
        client.post(
            "/memory",
            json={
                "title": "Delete me",
                "contentMarkdown": "This should be removed by release deletion.",
            },
        )
        blocked = client.post(
            "/release/delete-local-data",
            json={"confirmationPhrase": "DELETE", "includeVault": True},
        )
        deleted = client.post(
            "/release/delete-local-data",
            json={"confirmationPhrase": "DELETE LOCAL DATA", "includeVault": True},
        )
        onboarding = client.get("/onboarding/state")
        memory = client.get("/memory")

    assert blocked.status_code == 400
    assert deleted.status_code == 200
    assert deleted.json()["deleted"] is True
    assert deleted.json()["vaultDeleted"] is True
    assert not vault_path.exists()
    assert onboarding.json()["completed"] is False
    assert memory.json()["total"] == 0
