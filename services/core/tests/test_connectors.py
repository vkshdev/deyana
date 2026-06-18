from __future__ import annotations

from fastapi.testclient import TestClient

from deyana_core.app import create_app
from deyana_core.runtime import RuntimeState
from deyana_core.settings import CoreSettings


def make_client(tmp_path) -> TestClient:
    settings = CoreSettings(data_dir=tmp_path / "data", log_dir=tmp_path / "logs")
    return TestClient(create_app(RuntimeState(settings)))


def connect_mock_connector(client: TestClient, connector_id: str = "gmail") -> dict:
    start = client.post(
        f"/connectors/{connector_id}/oauth/start",
        json={"redirectUri": "deyana://oauth/callback"},
    )
    assert start.status_code == 200

    complete = client.post(
        f"/connectors/{connector_id}/oauth/complete",
        json={
            "state": start.json()["state"],
            "code": "CONNECTOR_TEST_CODE",
            "userApproved": True,
        },
    )
    assert complete.status_code == 200
    return complete.json()


def test_connectors_register_and_show_initial_status(tmp_path) -> None:
    with make_client(tmp_path) as client:
        response = client.get("/connectors")
        status = client.get("/status")

    assert response.status_code == 200
    connectors = response.json()["items"]
    assert [connector["id"] for connector in connectors] == ["gmail", "calendar", "github"]
    assert {connector["status"] for connector in connectors} == {"not_connected"}
    assert all(connector["tokenStored"] is False for connector in connectors)
    assert status.json()["featureFlags"]["connectors"] is True
    assert status.json()["featureFlags"]["connectorScheduler"] is True
    assert status.json()["featureFlags"]["encryptedTokenStorage"] is True


def test_mock_oauth_stores_connector_token_encrypted_locally(tmp_path) -> None:
    with make_client(tmp_path) as client:
        connector = connect_mock_connector(client, "gmail")
        audit = client.get("/privacy/audit")

    assert connector["id"] == "gmail"
    assert connector["status"] == "connected"
    assert connector["enabled"] is True
    assert connector["tokenStored"] is True
    assert connector["tokenUpdatedAt"]
    assert connector["nextSyncAt"]

    assert audit.status_code == 200
    assert audit.json()["events"][0]["eventType"] == "privacy.request.allowed"
    assert audit.json()["events"][0]["connectorId"] == "gmail"
    assert audit.json()["events"][0]["purpose"] == "oauth_api_fetch"

    database_bytes = (tmp_path / "data" / "connectors.sqlite3").read_bytes()
    assert b"CONNECTOR_TEST_CODE" not in database_bytes
    assert b"mock_access_" not in database_bytes
    assert b"mock_refresh_" not in database_bytes


def test_connector_settings_update_and_manual_sync_run(tmp_path) -> None:
    with make_client(tmp_path) as client:
        connect_mock_connector(client, "github")

        settings = client.patch(
            "/connectors/github/settings",
            json={"enabled": True, "syncIntervalMinutes": 60},
        )
        sync = client.post("/connectors/github/sync", json={"reason": "manual"})
        runs = client.get("/connectors/sync-runs")
        connector = client.get("/connectors/github")

    assert settings.status_code == 200
    assert settings.json()["syncIntervalMinutes"] == 60

    assert sync.status_code == 200
    assert sync.json()["connector"]["status"] == "connected"
    assert sync.json()["connector"]["lastSyncAt"]
    assert sync.json()["connector"]["nextSyncAt"]
    assert sync.json()["run"]["status"] == "completed"
    assert sync.json()["run"]["reason"] == "manual"

    assert runs.status_code == 200
    assert runs.json()["total"] == 1
    assert runs.json()["items"][0]["status"] == "completed"
    assert runs.json()["items"][0]["connectorId"] == "github"

    assert connector.status_code == 200
    assert connector.json()["lastError"] is None


def test_manual_sync_emits_websocket_events(tmp_path) -> None:
    with make_client(tmp_path) as client:
        connect_mock_connector(client, "calendar")

        with client.websocket_connect("/ws") as websocket:
            assert websocket.receive_json()["type"] == "app.ready"
            response = client.post("/connectors/calendar/sync", json={"reason": "manual"})
            assert response.status_code == 200

            events = [websocket.receive_json()["type"] for _ in range(4)]

    assert "connector.sync.started" in events
    assert "privacy.request.allowed" in events
    assert "connector.sync.completed" in events
    assert "connector.status.changed" in events
