from __future__ import annotations

import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any

from fastapi.testclient import TestClient

from deyana_core.app import create_app
from deyana_core.runtime import RuntimeState
from deyana_core.settings import CoreSettings


def make_client(tmp_path) -> TestClient:
    settings = CoreSettings(data_dir=tmp_path / "data", log_dir=tmp_path / "logs")
    return TestClient(create_app(RuntimeState(settings)))


class FakeConnectorApi:
    def __init__(self) -> None:
        self.server: ThreadingHTTPServer | None = None
        self.thread: threading.Thread | None = None
        self.endpoint = ""
        self.requests: list[dict[str, Any]] = []

    def __enter__(self) -> "FakeConnectorApi":
        parent = self

        class Handler(BaseHTTPRequestHandler):
            def do_POST(self) -> None:
                length = int(self.headers.get("content-length", "0"))
                body = self.rfile.read(length).decode("utf-8")
                parent.requests.append({"method": "POST", "path": self.path, "body": body})
                if self.path in {"/oauth/google/token", "/oauth/github/token"}:
                    token_prefix = "github" if "github" in self.path else "google"
                    self.send_json(
                        200,
                        {
                            "access_token": f"{token_prefix}_real_access",
                            "refresh_token": f"{token_prefix}_real_refresh",
                            "expires_in": 3600,
                            "token_type": "Bearer",
                            "scope": "read",
                        },
                    )
                    return
                self.send_json(404, {"error": "not found"})

            def do_GET(self) -> None:
                parent.requests.append(
                    {
                        "method": "GET",
                        "path": self.path,
                        "authorization": self.headers.get("authorization"),
                    }
                )
                if self.path.startswith("/gmail/users/me/messages/gmail-1"):
                    self.send_json(
                        200,
                        {
                            "id": "gmail-1",
                            "snippet": "Launch checklist is ready for review.",
                            "payload": {
                                "headers": [
                                    {"name": "From", "value": "founder@example.com"},
                                    {"name": "Subject", "value": "Launch checklist"},
                                    {"name": "Date", "value": "Fri, 19 Jun 2026 09:00:00 +0000"},
                                ]
                            },
                        },
                    )
                    return
                if self.path.startswith("/gmail/users/me/messages"):
                    self.send_json(200, {"messages": [{"id": "gmail-1"}]})
                    return
                if self.path.startswith("/calendar/calendars/primary/events"):
                    self.send_json(
                        200,
                        {
                            "items": [
                                {
                                    "id": "calendar-1",
                                    "summary": "Investor follow-up",
                                    "start": {"dateTime": "2026-06-20T10:00:00Z"},
                                    "end": {"dateTime": "2026-06-20T10:30:00Z"},
                                    "location": "Video call",
                                    "htmlLink": "https://calendar.google.com/event?eid=calendar-1",
                                }
                            ]
                        },
                    )
                    return
                if self.path.startswith("/github/user/repos"):
                    self.send_json(
                        200,
                        [
                            {
                                "id": 42,
                                "full_name": "deyana/local-assistant",
                                "description": "Local-first assistant",
                                "private": True,
                                "language": "Python",
                                "updated_at": "2026-06-19T08:00:00Z",
                                "html_url": "https://github.com/deyana/local-assistant",
                            }
                        ],
                    )
                    return
                self.send_json(404, {"error": "not found"})

            def log_message(self, _format: str, *_args: object) -> None:
                return

            def send_json(self, status: int, payload: object) -> None:
                body = json.dumps(payload).encode("utf-8")
                self.send_response(status)
                self.send_header("content-type", "application/json")
                self.send_header("content-length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.endpoint = f"http://127.0.0.1:{self.server.server_address[1]}"
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        return self

    def __exit__(self, *_args: object) -> None:
        if self.server:
            self.server.shutdown()
            self.server.server_close()
        if self.thread:
            self.thread.join(timeout=2)


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


def complete_onboarding(client: TestClient, tmp_path) -> None:
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


def test_connectors_register_and_show_initial_status(tmp_path) -> None:
    with make_client(tmp_path) as client:
        response = client.get("/connectors")
        status = client.get("/status")

    assert response.status_code == 200
    connectors = response.json()["items"]
    assert [connector["id"] for connector in connectors] == [
        "gmail",
        "calendar",
        "github",
        "drive",
        "slack",
        "notion",
        "jira",
        "linear",
    ]
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
        disconnect = client.post("/connectors/github/disconnect")

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
    assert disconnect.status_code == 200
    assert disconnect.json()["tokenDeleted"] is True
    assert disconnect.json()["connector"]["status"] == "not_connected"
    assert disconnect.json()["connector"]["tokenStored"] is False


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


def test_real_connectors_fetch_normalize_and_write_local_memory(tmp_path, monkeypatch) -> None:
    with FakeConnectorApi() as api:
        monkeypatch.setenv("DEYANA_GOOGLE_OAUTH_CLIENT_ID", "google-client")
        monkeypatch.setenv("DEYANA_GOOGLE_OAUTH_CLIENT_SECRET", "google-secret")
        monkeypatch.setenv("DEYANA_GITHUB_OAUTH_CLIENT_ID", "github-client")
        monkeypatch.setenv("DEYANA_GITHUB_OAUTH_CLIENT_SECRET", "github-secret")
        monkeypatch.setenv("DEYANA_GOOGLE_OAUTH_TOKEN_URL", f"{api.endpoint}/oauth/google/token")
        monkeypatch.setenv("DEYANA_GITHUB_OAUTH_TOKEN_URL", f"{api.endpoint}/oauth/github/token")
        monkeypatch.setenv("DEYANA_GMAIL_API_BASE_URL", f"{api.endpoint}/gmail")
        monkeypatch.setenv("DEYANA_CALENDAR_API_BASE_URL", f"{api.endpoint}/calendar")
        monkeypatch.setenv("DEYANA_GITHUB_API_BASE_URL", f"{api.endpoint}/github")

        with make_client(tmp_path) as client:
            complete_onboarding(client, tmp_path)
            for connector_id in ["gmail", "calendar", "github"]:
                start = client.post(
                    f"/connectors/{connector_id}/oauth/start",
                    json={"redirectUri": "deyana://oauth/callback"},
                )
                assert start.status_code == 200
                assert start.json()["mock"] is False
                connected = client.post(
                    f"/connectors/{connector_id}/oauth/complete",
                    json={
                        "state": start.json()["state"],
                        "code": f"{connector_id}-code",
                        "userApproved": True,
                    },
                )
                assert connected.status_code == 200
                assert connected.json()["tokenStored"] is True
                assert connected.json()["oauthConfigured"] is True

            first_runs = {
                connector_id: client.post(
                    f"/connectors/{connector_id}/sync",
                    json={"reason": "manual"},
                ).json()
                for connector_id in ["gmail", "calendar", "github"]
            }
            second_gmail = client.post("/connectors/gmail/sync", json={"reason": "manual"})
            memory = client.get("/memory", params={"query": "connector", "limit": 20}).json()

    assert {response["run"]["status"] for response in first_runs.values()} == {"completed"}
    assert {response["run"]["itemsWritten"] for response in first_runs.values()} == {1}
    assert second_gmail.status_code == 200
    assert second_gmail.json()["run"]["itemsSeen"] == 1
    assert second_gmail.json()["run"]["itemsWritten"] == 0

    source_types = {item["sourceType"] for item in memory["items"]}
    assert {"gmail", "calendar", "github"}.issubset(source_types)
    markdown_paths = {item["sourceType"]: item["markdownPath"] for item in memory["items"]}
    assert "Emails" in markdown_paths["gmail"]
    assert "Meetings" in markdown_paths["calendar"]
    assert "GitHub" in markdown_paths["github"]
    for path in markdown_paths.values():
        assert path
