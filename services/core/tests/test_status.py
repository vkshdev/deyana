from __future__ import annotations

from fastapi.testclient import TestClient

from deyana_core.app import create_app


def test_status_endpoint_exposes_current_core_capabilities() -> None:
    with TestClient(create_app()) as client:
        response = client.get("/status")

    assert response.status_code == 200
    body = response.json()
    assert body["service"] == "deyana-core"
    assert body["featureFlags"]["websocketEvents"] is True
    assert body["featureFlags"]["localLogging"] is True
    assert body["featureFlags"]["settings"] is True
    assert body["featureFlags"]["onboarding"] is True
    assert body["featureFlags"]["vaultSetup"] is True
    assert body["featureFlags"]["memory"] is True
    assert body["featureFlags"]["models"] is False
    assert any(dependency["name"] == "python" for dependency in body["dependencies"])
