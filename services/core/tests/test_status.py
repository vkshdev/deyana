from __future__ import annotations

from fastapi.testclient import TestClient

from deyana_core.app import create_app
from deyana_core.runtime import RuntimeState
from deyana_core.settings import CoreSettings


def test_status_endpoint_exposes_current_core_capabilities(tmp_path) -> None:
    settings = CoreSettings(data_dir=tmp_path / "data", log_dir=tmp_path / "logs")
    with TestClient(create_app(RuntimeState(settings))) as client:
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
    assert body["featureFlags"]["memorySummarization"] is True
    assert body["featureFlags"]["memoryExtraction"] is True
    assert body["featureFlags"]["memoryRollups"] is True
    assert body["featureFlags"]["models"] is True
    assert body["featureFlags"]["chat"] is True
    assert body["featureFlags"]["memoryRetrieval"] is True
    assert body["featureFlags"]["voice"] is True
    assert body["featureFlags"]["localStt"] is True
    assert body["featureFlags"]["localTts"] is True
    assert body["featureFlags"]["voiceSettings"] is True
    assert body["featureFlags"]["floatingUiPolish"] is True
    assert body["featureFlags"]["reduceMotion"] is True
    assert body["featureFlags"]["lowPowerMode"] is True
    assert body["featureFlags"]["multiMonitorPositioning"] is True
    assert body["featureFlags"]["releaseQuality"] is True
    assert body["featureFlags"]["logsViewer"] is True
    assert body["featureFlags"]["privacyExport"] is True
    assert body["featureFlags"]["deleteLocalData"] is True
    assert body["featureFlags"]["connectorHealth"] is True
    assert body["featureFlags"]["performanceProfiling"] is True
    assert body["featureFlags"]["crashRecovery"] is True
    assert any(dependency["name"] == "python" for dependency in body["dependencies"])
    assert any(dependency["name"] == "ollama" for dependency in body["dependencies"])
    assert any(dependency["name"] == "local_voice" for dependency in body["dependencies"])
    assert any(dependency["name"] == "release_readiness" for dependency in body["dependencies"])
